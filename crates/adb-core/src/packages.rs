//! The packages on a phone and what state each is in for the main user (user 0).
//!
//! Without root nothing can be deleted from the system partition. "Removed" here means what
//! `pm uninstall -k --user 0` does: the package is uninstalled for the main user, its APK stays in
//! the system image and its data is kept, so `pm install-existing` brings it back as it was. A
//! factory reset brings every removed system app back too.

use crate::sections::{script, split_sections};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageState {
    /// Installed and enabled for the main user.
    Installed,
    /// Installed but switched off (`pm disable-user`): it does not run and its icon is gone.
    Disabled,
    /// Uninstalled for the main user; still in the system image (or kept with its data).
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    /// Where the APK is: `/system/app/...` for a preinstalled app, `/data/app/...` for one the
    /// owner installed or that was updated.
    pub path: Option<String>,
    /// Came with the phone (lives in a system partition).
    pub system: bool,
    pub state: PackageState,
}

/// The shell script `list` runs: every package including removed ones with its path, then the
/// installed, disabled and system ones.
pub fn list_script() -> String {
    script(&[
        ("all", "pm list packages -f -u --user 0"),
        ("installed", "pm list packages --user 0"),
        ("disabled", "pm list packages -d --user 0"),
        ("system", "pm list packages -s -u --user 0"),
    ])
}

/// `package:/system/app/Foo/Foo.apk=com.foo` (with -f) or `package:com.foo`. A path may itself
/// contain `=` (`/data/app/~~a1b2==/...`), so the name is what follows the last one.
fn parse_line(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.trim().strip_prefix("package:")?;
    match rest.rsplit_once('=') {
        Some((path, name)) if path.starts_with('/') => Some((name.trim().to_string(), Some(path.to_string()))),
        _ => Some((rest.trim().to_string(), None)),
    }
    .filter(|(name, _)| !name.is_empty() && !name.contains(char::is_whitespace))
}

fn names(text: &str) -> HashSet<String> {
    text.lines().filter_map(parse_line).map(|(n, _)| n).collect()
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ListError {
    /// The phone answered something else: the package manager is not running yet (a phone that is
    /// still starting) or the shell failed.
    #[error("the phone did not list its packages: {0}")]
    Unreadable(String),
}

/// The packages from the output of `list_script`, sorted by name.
pub fn parse_list(text: &str) -> Result<Vec<Package>, ListError> {
    let parts = split_sections(text);
    let all_text = parts.get("all").map(String::as_str).unwrap_or("");
    let all: BTreeMap<String, Option<String>> = all_text.lines().filter_map(parse_line).collect();
    if all.is_empty() {
        let shown = text
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with("@@"))
            .unwrap_or("")
            .trim();
        return Err(ListError::Unreadable(shown.chars().take(200).collect()));
    }
    let installed = names(parts.get("installed").map(String::as_str).unwrap_or(""));
    let disabled = names(parts.get("disabled").map(String::as_str).unwrap_or(""));
    let system = names(parts.get("system").map(String::as_str).unwrap_or(""));

    Ok(all
        .into_iter()
        .map(|(name, path)| {
            let state = if !installed.contains(&name) {
                PackageState::Removed
            } else if disabled.contains(&name) {
                PackageState::Disabled
            } else {
                PackageState::Installed
            };
            let system = system.contains(&name);
            Package {
                name,
                path,
                system,
                state,
            }
        })
        .collect())
}

/// Package names are Java-style identifiers joined by dots. Anything else never reaches a shell.
pub fn is_package_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && s.split('.').all(|part| {
            let mut chars = part.chars();
            chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::marker;

    fn output(all: &str, installed: &str, disabled: &str, system: &str) -> String {
        format!(
            "{}\n{all}\n{}\n{installed}\n{}\n{disabled}\n{}\n{system}\n",
            marker("all"),
            marker("installed"),
            marker("disabled"),
            marker("system")
        )
    }

    #[test]
    fn works_out_each_state() {
        let text = output(
            "package:/system/app/MSA/MSA.apk=com.miui.msa.global\n\
             package:/system/priv-app/MiuiHome/MiuiHome.apk=com.miui.home\n\
             package:/data/app/~~Zx1==/com.spotify.music-Qw2==/base.apk=com.spotify.music\n\
             package:/system/app/Weather/Weather.apk=com.miui.weather2\n",
            "package:com.miui.home\npackage:com.spotify.music\npackage:com.miui.weather2\n",
            "package:com.miui.weather2\n",
            "package:com.miui.msa.global\npackage:com.miui.home\npackage:com.miui.weather2\n",
        );
        let p = parse_list(&text).unwrap();
        let by = |n: &str| p.iter().find(|x| x.name == n).unwrap().clone();
        assert_eq!(p.len(), 4);
        assert_eq!(by("com.miui.msa.global").state, PackageState::Removed);
        assert!(by("com.miui.msa.global").system);
        assert_eq!(by("com.miui.home").state, PackageState::Installed);
        assert_eq!(by("com.miui.weather2").state, PackageState::Disabled);
        let spotify = by("com.spotify.music");
        assert!(!spotify.system);
        assert_eq!(
            spotify.path.as_deref(),
            Some("/data/app/~~Zx1==/com.spotify.music-Qw2==/base.apk")
        );
        // Sorted by name.
        assert_eq!(p[0].name, "com.miui.home");
    }

    #[test]
    fn a_phone_that_is_still_starting_is_an_error() {
        let text = format!(
            "{}\nError: Could not access the Package Manager.  Is the system running?\n",
            marker("all")
        );
        assert_eq!(
            parse_list(&text),
            Err(ListError::Unreadable(
                "Error: Could not access the Package Manager.  Is the system running?".into()
            ))
        );
    }

    #[test]
    fn checks_package_names() {
        assert!(is_package_name("com.miui.msa.global"));
        assert!(is_package_name("com.google.android.apps.tachyon"));
        assert!(is_package_name("android"));
        assert!(is_package_name("com.sec.android.app.sbrowser_beta"));
        assert!(!is_package_name(""));
        assert!(!is_package_name("com.foo; reboot"));
        assert!(!is_package_name("com..foo"));
        assert!(!is_package_name("com.1foo"));
        assert!(!is_package_name("-k"));
        assert!(!is_package_name("com.foo bar"));
    }
}
