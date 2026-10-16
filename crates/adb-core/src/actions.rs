//! What can be done to a package, the adb shell command for it, and what the phone's answer means.

use crate::packages::PackageState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// `pm uninstall -k --user 0`: gone for the main user, data kept, can be restored.
    Remove,
    /// `pm disable-user --user 0`: stays installed but never runs. Some phones (MIUI) refuse it
    /// for their own apps; Remove works there.
    Disable,
    /// Brings a removed package back (`pm install-existing`) or switches a disabled one on
    /// (`pm enable`).
    Restore,
}

impl Action {
    /// Whether the action changes a package in this state.
    pub fn applies_to(self, state: PackageState) -> bool {
        match self {
            Action::Remove => state != PackageState::Removed,
            Action::Disable => state == PackageState::Installed,
            Action::Restore => state != PackageState::Installed,
        }
    }

    /// The `adb shell` arguments, or `None` when there is nothing to do.
    pub fn command(self, package: &str, state: PackageState) -> Option<Vec<String>> {
        if !self.applies_to(state) {
            return None;
        }
        let args: &[&str] = match (self, state) {
            (Action::Remove, _) => &["pm", "uninstall", "-k", "--user", "0"],
            (Action::Disable, _) => &["pm", "disable-user", "--user", "0"],
            (Action::Restore, PackageState::Removed) => &["pm", "install-existing", "--user", "0"],
            (Action::Restore, _) => &["pm", "enable", "--user", "0"],
        };
        Some(
            args.iter()
                .map(|s| s.to_string())
                .chain([package.to_string()])
                .collect(),
        )
    }

    /// The state a package is in after the action worked.
    pub fn result_state(self) -> PackageState {
        match self {
            Action::Remove => PackageState::Removed,
            Action::Disable => PackageState::Disabled,
            Action::Restore => PackageState::Installed,
        }
    }
}

/// Why a package was not changed, as a code the interface translates, and the phone's own words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub code: FailureCode,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    /// The app is a device administrator (Find my device, a work profile): switch that off first.
    DeviceAdmin,
    /// The phone's maker does not let adb disable this app; removing it works.
    DisableRefused,
    /// Android refused for a reason of its own (often a package the system needs).
    Refused,
    /// The package is not on the phone (any more).
    Unknown,
    /// Not in a state the action changes (already removed, already on).
    NothingToDo,
    /// Kept by adbDebloat's safety rules.
    Protected,
    /// Marked Expert, and expert mode is off.
    NeedsExpert,
    /// adb or the phone did not answer (a cable came out, the phone locked up).
    NoAnswer,
    Other,
}

/// Reads the phone's answer to `action`'s command.
pub fn outcome(action: Action, state: PackageState, text: &str) -> Result<(), Failure> {
    let lower = text.to_ascii_lowercase();
    let ok = match (action, state) {
        (Action::Remove, _) => text.lines().any(|l| l.trim() == "Success"),
        (Action::Disable, _) => lower.contains("new state: disabled-user"),
        (Action::Restore, PackageState::Removed) => lower.contains("installed for user"),
        (Action::Restore, _) => lower.contains("new state: enabled"),
    };
    if ok {
        return Ok(());
    }
    let detail = text.trim().lines().take(4).collect::<Vec<_>>().join("\n");
    let code = if lower.contains("device_policy_manager") || lower.contains("device admin") {
        FailureCode::DeviceAdmin
    } else if action == Action::Disable
        && (lower.contains("cannot change component state") || lower.contains("securityexception"))
    {
        FailureCode::DisableRefused
    } else if lower.contains("not installed for")
        || lower.contains("doesn't exist")
        || lower.contains("unknown package")
        || lower.contains("does not exist")
    {
        FailureCode::Unknown
    } else if lower.contains("failure [") || lower.contains("exception") {
        FailureCode::Refused
    } else if detail.is_empty() {
        FailureCode::NoAnswer
    } else {
        FailureCode::Other
    };
    Err(Failure { code, detail })
}

#[cfg(test)]
mod tests {
    use super::*;
    use PackageState::*;

    #[test]
    fn builds_the_command_for_each_state() {
        let cmd = |a: Action, s| a.command("com.miui.msa.global", s).map(|v| v.join(" "));
        assert_eq!(
            cmd(Action::Remove, Installed).unwrap(),
            "pm uninstall -k --user 0 com.miui.msa.global"
        );
        assert_eq!(
            cmd(Action::Remove, Disabled).unwrap(),
            "pm uninstall -k --user 0 com.miui.msa.global"
        );
        assert_eq!(cmd(Action::Remove, Removed), None);
        assert_eq!(
            cmd(Action::Disable, Installed).unwrap(),
            "pm disable-user --user 0 com.miui.msa.global"
        );
        assert_eq!(cmd(Action::Disable, Disabled), None);
        assert_eq!(cmd(Action::Disable, Removed), None);
        assert_eq!(
            cmd(Action::Restore, Removed).unwrap(),
            "pm install-existing --user 0 com.miui.msa.global"
        );
        assert_eq!(
            cmd(Action::Restore, Disabled).unwrap(),
            "pm enable --user 0 com.miui.msa.global"
        );
        assert_eq!(cmd(Action::Restore, Installed), None);
    }

    #[test]
    fn reads_what_android_answers() {
        assert_eq!(outcome(Action::Remove, Installed, "Success\n"), Ok(()));
        assert_eq!(
            outcome(Action::Disable, Installed, "Package com.x new state: disabled-user\n"),
            Ok(())
        );
        assert_eq!(
            outcome(Action::Restore, Removed, "Package com.x installed for user: 0\n"),
            Ok(())
        );
        assert_eq!(
            outcome(Action::Restore, Disabled, "Package com.x new state: enabled\n"),
            Ok(())
        );

        let code = |a, s, t: &str| outcome(a, s, t).unwrap_err().code;
        assert_eq!(
            code(
                Action::Remove,
                Installed,
                "Failure [DELETE_FAILED_DEVICE_POLICY_MANAGER]"
            ),
            FailureCode::DeviceAdmin
        );
        assert_eq!(
            code(Action::Remove, Installed, "Failure [DELETE_FAILED_INTERNAL_ERROR]"),
            FailureCode::Refused
        );
        assert_eq!(
            code(Action::Remove, Installed, "Failure [not installed for 0]"),
            FailureCode::Unknown
        );
        assert_eq!(
            code(
                Action::Disable,
                Installed,
                "Exception occurred while executing 'disable-user':\njava.lang.SecurityException: Shell cannot change component state for com.miui.weather2/null to 3\n\tat com.android.server.pm.PackageManagerService.setEnabledSetting"
            ),
            FailureCode::DisableRefused
        );
        assert_eq!(
            code(Action::Restore, Removed, "Error: Package com.nope doesn't exist"),
            FailureCode::Unknown
        );
        assert_eq!(code(Action::Remove, Installed, ""), FailureCode::NoAnswer);
        assert_eq!(code(Action::Remove, Installed, "error: closed"), FailureCode::Other);
    }

    #[test]
    fn keeps_the_first_lines_of_a_long_answer() {
        let long = "Exception occurred\nline 2\nline 3\nline 4\nline 5\nline 6";
        assert_eq!(
            outcome(Action::Remove, Installed, long).unwrap_err().detail,
            "Exception occurred\nline 2\nline 3\nline 4"
        );
    }
}
