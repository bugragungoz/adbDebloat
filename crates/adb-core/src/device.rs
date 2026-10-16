//! What adbDebloat asks adb to do, on top of a `Runner`.

use crate::actions::{outcome, Action, Failure, FailureCode};
use crate::devices::{is_network_address, parse_devices, parse_version, DeviceEntry};
use crate::info::{info_script, parse_info, DeviceInfo};
use crate::knowledge::Knowledge;
use crate::packages::{is_package_name, list_script, parse_list, Package, PackageState};
use crate::runner::{Output, RunError, Runner, LONG, SHORT};
use crate::safety::{assess, Verdict};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AdbError {
    #[error(transparent)]
    Run(#[from] RunError),
    /// adb ran and said no; the text is what it printed.
    #[error("{0}")]
    Failed(String),
    /// Input that never reaches adb (an address or code that cannot be right).
    #[error("{0}")]
    Invalid(String),
}

pub struct Adb<'a> {
    runner: &'a dyn Runner,
}

fn checked(out: Output) -> Result<Output, AdbError> {
    if out.success() {
        Ok(out)
    } else {
        Err(AdbError::Failed(out.text()))
    }
}

/// adb answers `connect` and `pair` with exit code 0 even when they fail, so their words decide.
fn said(out: &Output, ok_words: &[&str]) -> Result<String, AdbError> {
    let text = out.text();
    let lower = text.to_ascii_lowercase();
    if ok_words.iter().any(|w| lower.contains(w)) && !lower.contains("failed") && !lower.contains("cannot") {
        Ok(text)
    } else {
        Err(AdbError::Failed(text))
    }
}

/// adb's own complaints, as opposed to the phone's answer: the device is gone or not ready.
fn lost_device(out: &Output) -> bool {
    !out.success()
        && out.stdout.trim().is_empty()
        && out.stderr.lines().any(|l| {
            let l = l.trim();
            (l.starts_with("adb: ") || l.starts_with("error: "))
                && (l.contains("not found")
                    || l.contains("offline")
                    || l.contains("closed")
                    || l.contains("no devices")
                    || l.contains("unauthorized"))
        })
}

impl<'a> Adb<'a> {
    pub fn new(runner: &'a dyn Runner) -> Self {
        Adb { runner }
    }

    fn adb(&self, args: &[&str], timeout: Duration) -> Result<Output, AdbError> {
        Ok(self.runner.run(args, timeout)?)
    }

    fn shell(&self, serial: &str, command: &str, timeout: Duration) -> Result<Output, AdbError> {
        self.adb(&["-s", serial, "shell", command], timeout)
    }

    pub fn version(&self) -> Result<String, AdbError> {
        let out = checked(self.adb(&["version"], SHORT)?)?;
        parse_version(&out.stdout).ok_or_else(|| AdbError::Failed(out.text()))
    }

    pub fn devices(&self) -> Result<Vec<DeviceEntry>, AdbError> {
        Ok(parse_devices(&checked(self.adb(&["devices", "-l"], SHORT)?)?.stdout))
    }

    /// Runs a script of several commands. Its exit code is the last command's, so only adb's own
    /// complaints count as failure.
    fn script(&self, serial: &str, script: &str, timeout: Duration) -> Result<Output, AdbError> {
        let out = self.shell(serial, script, timeout)?;
        if lost_device(&out) {
            return Err(AdbError::Failed(out.text()));
        }
        Ok(out)
    }

    pub fn info(&self, serial: &str) -> Result<DeviceInfo, AdbError> {
        Ok(parse_info(serial, &self.script(serial, &info_script(), SHORT)?.stdout))
    }

    pub fn packages(&self, serial: &str) -> Result<Vec<Package>, AdbError> {
        let out = self.script(serial, &list_script(), LONG)?;
        parse_list(&out.stdout).map_err(|e| AdbError::Failed(e.to_string()))
    }

    /// Runs one action on one package and reads the answer.
    pub fn change(&self, serial: &str, action: Action, package: &str, state: PackageState) -> Result<(), Failure> {
        if !is_package_name(package) {
            return Err(Failure {
                code: FailureCode::Unknown,
                detail: package.chars().take(100).collect(),
            });
        }
        let Some(command) = action.command(package, state) else {
            return Err(Failure {
                code: FailureCode::NothingToDo,
                detail: String::new(),
            });
        };
        match self.shell(serial, &command.join(" "), SHORT) {
            Ok(out) if lost_device(&out) => Err(Failure {
                code: FailureCode::NoAnswer,
                detail: out.text(),
            }),
            Ok(out) => outcome(action, state, &out.text()),
            Err(e) => Err(Failure {
                code: FailureCode::NoAnswer,
                detail: e.to_string(),
            }),
        }
    }

    /// Pairs with a phone's Wireless debugging (Android 11 and later): the address and six-digit
    /// code from "Pair device with pairing code".
    pub fn pair(&self, address: &str, code: &str) -> Result<String, AdbError> {
        let address = address.trim();
        let code = code.trim();
        if !is_network_address(address) {
            return Err(AdbError::Invalid(address.to_string()));
        }
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(AdbError::Invalid(code.to_string()));
        }
        said(&self.adb(&["pair", address, code], LONG)?, &["successfully paired"])
    }

    pub fn connect(&self, address: &str) -> Result<String, AdbError> {
        let address = address.trim();
        if !is_network_address(address) {
            return Err(AdbError::Invalid(address.to_string()));
        }
        said(&self.adb(&["connect", address], LONG)?, &["connected to"])
    }

    pub fn disconnect(&self, serial: &str) -> Result<(), AdbError> {
        checked(self.adb(&["disconnect", serial], SHORT)?).map(|_| ())
    }

    /// The phone's Wi-Fi address, to switch a USB connection to Wi-Fi.
    pub fn wifi_address(&self, serial: &str) -> Result<Option<Ipv4Addr>, AdbError> {
        let out = self.shell(serial, "ip -f inet addr show wlan0", SHORT)?;
        Ok(out.stdout.lines().find_map(|line| {
            let rest = line.trim().strip_prefix("inet ")?;
            rest.split(['/', ' ']).next()?.parse().ok()
        }))
    }

    /// Makes a USB-connected phone listen for adb on the network (until it restarts) and
    /// connects to it there, so the cable can come out.
    pub fn switch_to_wifi(&self, serial: &str) -> Result<String, AdbError> {
        let ip = self
            .wifi_address(serial)?
            .ok_or_else(|| AdbError::Failed("no Wi-Fi address".into()))?;
        checked(self.adb(&["-s", serial, "tcpip", "5555"], SHORT)?)?;
        let address = format!("{ip}:5555");
        // The phone restarts adb on the network; give it a moment before connecting.
        let mut last = AdbError::Failed(String::new());
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(800));
            match self.connect(&address) {
                Ok(_) => return Ok(address),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub fn reboot(&self, serial: &str) -> Result<(), AdbError> {
        checked(self.adb(&["-s", serial, "reboot"], LONG)?).map(|_| ())
    }

    pub fn kill_server(&self) -> Result<(), AdbError> {
        self.adb(&["kill-server"], SHORT).map(|_| ())
    }
}

/// One package of a batch, as the history keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub package: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The state before; `None` when the phone does not have the package.
    pub before: Option<PackageState>,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
}

/// Runs `action` on each package in turn, checking each against the phone's current list and the
/// safety rules first. `progress` hears about every package as it is done; `stop` ends the batch
/// before the next package.
#[allow(clippy::too_many_arguments)]
pub fn run_batch(
    adb: &Adb,
    serial: &str,
    action: Action,
    packages: &[String],
    knowledge: &Knowledge,
    device: &DeviceInfo,
    expert: bool,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(usize, &ItemResult),
) -> Result<Vec<ItemResult>, AdbError> {
    let current = adb.packages(serial)?;
    let mut results = Vec::with_capacity(packages.len());
    // A phone that did not answer twice in a row (a cable out, adb gone) will not answer the rest.
    let mut silent = 0;
    for (index, name) in packages.iter().enumerate() {
        let label = knowledge.get(name).and_then(|e| e.label.clone());
        let found = current.iter().find(|p| &p.name == name);
        let before = found.map(|p| p.state);
        let fail = |code| {
            Err(Failure {
                code,
                detail: String::new(),
            })
        };
        let verdict = || assess(name, knowledge.get(name), device, expert);
        let result = match before {
            _ if stop.load(Ordering::Relaxed) || silent >= 2 => fail(FailureCode::NoAnswer),
            None => fail(FailureCode::Unknown),
            Some(_) if action != Action::Restore && verdict() == Verdict::NeedsExpert => fail(FailureCode::NeedsExpert),
            Some(_) if action != Action::Restore && verdict() != Verdict::Allowed => fail(FailureCode::Protected),
            Some(state) => {
                let r = adb.change(serial, action, name, state);
                silent = if matches!(&r, Err(f) if f.code == FailureCode::NoAnswer) {
                    silent + 1
                } else {
                    0
                };
                r
            }
        };
        let item = ItemResult {
            package: name.clone(),
            label,
            before,
            ok: result.is_ok(),
            failure: result.err(),
        };
        progress(index, &item);
        results.push(item);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            code: Some(code),
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    #[test]
    fn tells_adb_complaints_from_the_phone_answering() {
        assert!(lost_device(&out(1, "", "adb: device '4b2a1c3d' not found\n")));
        assert!(lost_device(&out(1, "", "error: device offline\n")));
        assert!(lost_device(&out(1, "", "error: closed\n")));
        assert!(lost_device(&out(
            1,
            "",
            "error: device unauthorized.\nThis adb server's $ADB_VENDOR_KEYS is not set\n"
        )));
        assert!(!lost_device(&out(1, "Failure [DELETE_FAILED_INTERNAL_ERROR]\n", "")));
        assert!(!lost_device(&out(0, "Success\n", "")));
        assert!(!lost_device(&out(
            255,
            "",
            "Exception occurred while executing 'disable-user':\n"
        )));
    }
}
