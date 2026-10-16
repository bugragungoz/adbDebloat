//! A simulated phone behind a `Runner`, answering the way a real one does. The tests drive the
//! whole flow with it: device list, information, package list, changes and their failures.

use crate::packages::PackageState;
use crate::runner::{Output, RunError, Runner};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPackage {
    pub path: String,
    pub system: bool,
    pub state: PackageState,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Phone {
    pub serial: String,
    pub props: Vec<(String, String)>,
    pub launcher: String,
    pub keyboard: String,
    pub packages: BTreeMap<String, SimPackage>,
    /// Packages whose maker refuses `disable-user` (as MIUI does for its own apps).
    pub refuse_disable: HashSet<String>,
    /// Device administrators: uninstall fails until that is switched off.
    pub admins: HashSet<String>,
    /// When set, every shell command fails as if the cable came out.
    pub unplugged: bool,
    /// Pulls the cable right after the next package list.
    pub unplug_after_list: bool,
}

pub struct Simulator {
    pub phone: Mutex<Phone>,
    /// Every command line run, for the tests to check.
    pub log: Mutex<Vec<String>>,
}

impl Phone {
    /// A Redmi 5 Plus on MIUI 12 with a handful of packages.
    pub fn redmi() -> Self {
        let mut p = Phone {
            serial: "4b2a1c3d".into(),
            props: [
                ("ro.product.brand", "xiaomi"),
                ("ro.product.manufacturer", "Xiaomi"),
                ("ro.product.model", "Redmi 5 Plus"),
                ("ro.product.device", "vince"),
                ("ro.build.version.release", "8.1.0"),
                ("ro.build.version.sdk", "27"),
                ("ro.miui.ui.version.name", "V12"),
                ("ro.build.display.id", "OPM1.171019.019"),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
            launcher: "com.miui.home".into(),
            keyboard: "com.google.android.inputmethod.latin".into(),
            ..Default::default()
        };
        for (name, system) in [
            ("android", true),
            ("com.android.systemui", true),
            ("com.miui.home", true),
            ("com.miui.msa.global", true),
            ("com.miui.analytics", true),
            ("com.miui.weather2", true),
            ("com.miui.powerkeeper", true),
            ("com.miui.securitycenter", true),
            ("com.xiaomi.finddevice", true),
            ("com.google.android.inputmethod.latin", true),
            ("com.google.android.youtube", true),
            ("com.spotify.music", false),
        ] {
            let dir = if system { "/system/app" } else { "/data/app" };
            p.packages.insert(
                name.into(),
                SimPackage {
                    path: format!("{dir}/{name}/base.apk"),
                    system,
                    state: PackageState::Installed,
                },
            );
        }
        p.refuse_disable.insert("com.miui.weather2".into());
        p.admins.insert("com.xiaomi.finddevice".into());
        p
    }
}

impl Simulator {
    pub fn new(phone: Phone) -> Self {
        Simulator {
            phone: Mutex::new(phone),
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn state(&self, package: &str) -> Option<PackageState> {
        self.phone.lock().unwrap().packages.get(package).map(|p| p.state)
    }

    fn list(phone: &Phone, flags: &[&str]) -> String {
        let mut out = String::new();
        for (name, p) in &phone.packages {
            let installed = p.state != PackageState::Removed;
            if !flags.contains(&"-u") && !installed {
                continue;
            }
            if flags.contains(&"-d") && p.state != PackageState::Disabled {
                continue;
            }
            if flags.contains(&"-s") && !p.system {
                continue;
            }
            if flags.contains(&"-f") {
                out.push_str(&format!("package:{}={name}\n", p.path));
            } else {
                out.push_str(&format!("package:{name}\n"));
            }
        }
        out
    }

    fn command(phone: &mut Phone, cmd: &str) -> String {
        let words: Vec<&str> = cmd.split_whitespace().collect();
        let package = words.last().copied().unwrap_or("");
        match words.as_slice() {
            ["echo", marker] => format!("{marker}\n"),
            ["getprop"] => phone.props.iter().map(|(k, v)| format!("[{k}]: [{v}]\n")).collect(),
            ["settings", "get", "secure", "default_input_method", ..] => {
                format!("{}/com.android.inputmethod.latin.LatinIME\n", phone.keyboard)
            }
            ["cmd", "package", "resolve-activity", ..] => {
                format!("priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\n{}/.Launcher\n", phone.launcher)
            }
            ["cmd", "package", "query-activities", ..] => format!("{}/.Launcher\n", phone.launcher),
            ["dumpsys", "battery", ..] => "Current Battery Service state:\n  level: 76\n".into(),
            ["pm", "list", "packages", flags @ ..] => Self::list(phone, flags),
            ["pm", "uninstall", "-k", "--user", "0", _] => {
                if phone.admins.contains(package) {
                    return "Failure [DELETE_FAILED_DEVICE_POLICY_MANAGER]\n".into();
                }
                match phone.packages.get_mut(package) {
                    Some(p) if p.state != PackageState::Removed => {
                        p.state = PackageState::Removed;
                        "Success\n".into()
                    }
                    _ => "Failure [not installed for 0]\n".into(),
                }
            }
            ["pm", "disable-user", "--user", "0", _] => {
                if phone.refuse_disable.contains(package) {
                    return format!("Exception occurred while executing 'disable-user':\njava.lang.SecurityException: Shell cannot change component state for {package}/null to 3\n");
                }
                match phone.packages.get_mut(package) {
                    Some(p) if p.state == PackageState::Installed => {
                        p.state = PackageState::Disabled;
                        format!("Package {package} new state: disabled-user\n")
                    }
                    _ => format!("Error: Package {package} not found\n"),
                }
            }
            ["pm", "enable", "--user", "0", _] => match phone.packages.get_mut(package) {
                Some(p) if p.state != PackageState::Removed => {
                    p.state = PackageState::Installed;
                    format!("Package {package} new state: enabled\n")
                }
                _ => format!("Error: Package {package} not found\n"),
            },
            ["pm", "install-existing", "--user", "0", _] => match phone.packages.get_mut(package) {
                Some(p) => {
                    p.state = PackageState::Installed;
                    format!("Package {package} installed for user: 0\n")
                }
                None => format!("Error: Package {package} doesn't exist\n"),
            },
            ["ip", "-f", "inet", "addr", "show", "wlan0"] => {
                "32: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500\n    inet 192.168.1.23/24 brd 192.168.1.255 scope global wlan0\n".into()
            }
            _ => format!("/system/bin/sh: {}: not found\n", words.first().unwrap_or(&"")),
        }
    }
}

impl Runner for Simulator {
    fn run(&self, args: &[&str], _timeout: Duration) -> Result<Output, RunError> {
        self.log.lock().unwrap().push(args.join(" "));
        let mut phone = self.phone.lock().unwrap();
        match args {
            ["version"] => Ok(Output::ok("Android Debug Bridge version 1.0.41\nVersion 35.0.2-12147458\n")),
            ["devices", "-l"] => Ok(Output::ok(format!(
                "List of devices attached\n{}  device usb:1-1 product:vince model:Redmi_5_Plus device:vince transport_id:1\n\n",
                phone.serial
            ))),
            ["-s", serial, "shell", script] if *serial == phone.serial => {
                if phone.unplugged {
                    return Ok(Output { code: Some(1), stdout: String::new(), stderr: format!("adb: device '{serial}' not found\n") });
                }
                // Scripts are `echo <marker>; <command>; ...`; single commands have no `;`.
                let out: String = script.split("; ").map(|c| Self::command(&mut phone, c)).collect();
                if script.contains("pm list packages") && phone.unplug_after_list {
                    phone.unplugged = true;
                }
                Ok(Output::ok(out))
            }
            ["-s", serial, ..] if *serial != phone.serial => {
                Ok(Output { code: Some(1), stdout: String::new(), stderr: format!("adb: device '{serial}' not found\n") })
            }
            ["-s", _, "tcpip", port] => Ok(Output::ok(format!("restarting in TCP mode port: {port}\n"))),
            ["-s", _, "reboot"] => Ok(Output::ok("")),
            ["pair", address, "123456"] => Ok(Output::ok(format!("Successfully paired to {address} [guid=adb-4b2a1c3d-AbCdEf]\n"))),
            ["pair", ..] => Ok(Output::ok("Failed: Wrong password or connection was dropped.\n")),
            ["connect", address] => Ok(Output::ok(format!("connected to {address}\n"))),
            ["disconnect", address] => Ok(Output::ok(format!("disconnected {address}\n"))),
            ["kill-server"] => Ok(Output::ok("")),
            _ => Ok(Output { code: Some(1), stdout: String::new(), stderr: "adb: usage: unknown command\n".into() }),
        }
    }
}
