//! Which packages adbDebloat may remove or disable. The backend checks this again before every
//! command, whatever the interface allowed.

use crate::info::DeviceInfo;
use crate::knowledge::{Entry, Removal};
use serde::Serialize;

/// Packages that are never removed or disabled: without them the phone does not start, cannot
/// call, cannot install, or adb itself stops working, which would leave no way back.
pub const CORE: &[&str] = &[
    "android",
    "com.android.systemui",
    "com.android.settings",
    "com.android.phone",
    "com.android.shell",
    "com.android.server.telecom",
    "com.android.providers.settings",
    "com.android.providers.telephony",
    "com.android.packageinstaller",
    "com.google.android.packageinstaller",
    "com.android.permissioncontroller",
    "com.google.android.permissioncontroller",
    "com.android.networkstack",
    "com.google.android.networkstack",
    "com.miui.securitycenter",
    "com.miui.system",
    "com.miui.rom",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Protection {
    /// On the `CORE` list.
    Core,
    /// Marked Unsafe: removing it can stop the phone from starting.
    Unsafe,
    /// The home screen in use (or one of them, when none is the default).
    Launcher,
    /// The keyboard in use: without it nothing can be typed, not even a PIN.
    Keyboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Allowed,
    /// Marked Expert; allowed only in expert mode.
    NeedsExpert,
    Protected(Protection),
}

/// Whether a package may be removed or disabled on this phone. Restoring is always allowed.
pub fn assess(package: &str, entry: Option<&Entry>, device: &DeviceInfo, expert: bool) -> Verdict {
    if CORE.contains(&package) {
        return Verdict::Protected(Protection::Core);
    }
    if device.launchers.iter().any(|l| l == package) {
        return Verdict::Protected(Protection::Launcher);
    }
    if device.keyboard.as_deref() == Some(package) {
        return Verdict::Protected(Protection::Keyboard);
    }
    match entry.map(|e| e.removal) {
        Some(Removal::Unsafe) => Verdict::Protected(Protection::Unsafe),
        Some(Removal::Expert) if !expert => Verdict::NeedsExpert,
        _ => Verdict::Allowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(removal: Removal) -> Entry {
        Entry {
            label: None,
            list: "oem".into(),
            removal,
            description: String::new(),
            dependencies: vec![],
            needed_by: vec![],
        }
    }

    fn phone() -> DeviceInfo {
        DeviceInfo {
            launchers: vec!["com.miui.home".into()],
            keyboard: Some("com.google.android.inputmethod.latin".into()),
            ..Default::default()
        }
    }

    #[test]
    fn keeps_what_the_phone_cannot_do_without() {
        let p = phone();
        assert_eq!(
            assess("com.android.systemui", None, &p, true),
            Verdict::Protected(Protection::Core)
        );
        assert_eq!(
            assess("com.android.shell", None, &p, true),
            Verdict::Protected(Protection::Core)
        );
        assert_eq!(
            assess("com.miui.home", Some(&entry(Removal::Expert)), &p, true),
            Verdict::Protected(Protection::Launcher)
        );
        assert_eq!(
            assess("com.google.android.inputmethod.latin", None, &p, true),
            Verdict::Protected(Protection::Keyboard)
        );
        assert_eq!(
            assess("com.miui.system", Some(&entry(Removal::Unsafe)), &p, true),
            Verdict::Protected(Protection::Core)
        );
        assert_eq!(
            assess("com.example.boot", Some(&entry(Removal::Unsafe)), &p, true),
            Verdict::Protected(Protection::Unsafe)
        );
    }

    #[test]
    fn expert_packages_need_expert_mode() {
        let p = phone();
        let e = entry(Removal::Expert);
        assert_eq!(
            assess("com.miui.powerkeeper", Some(&e), &p, false),
            Verdict::NeedsExpert
        );
        assert_eq!(assess("com.miui.powerkeeper", Some(&e), &p, true), Verdict::Allowed);
    }

    #[test]
    fn everything_else_is_allowed_including_unknown_packages() {
        let p = phone();
        assert_eq!(
            assess("com.miui.msa.global", Some(&entry(Removal::Recommended)), &p, false),
            Verdict::Allowed
        );
        assert_eq!(
            assess("com.miui.gallery", Some(&entry(Removal::Advanced)), &p, false),
            Verdict::Allowed
        );
        assert_eq!(assess("com.unknown.app", None, &p, false), Verdict::Allowed);
        // Another phone with another launcher: MIUI's is not protected as the launcher there.
        let other = DeviceInfo {
            launchers: vec!["bitpit.launcher".into()],
            ..Default::default()
        };
        assert_eq!(
            assess("com.miui.home", Some(&entry(Removal::Expert)), &other, true),
            Verdict::Allowed
        );
    }
}
