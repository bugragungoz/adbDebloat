//! What a phone is: its properties, its home screen and keyboard (which must never be removed),
//! and its battery. One `adb shell` call collects all of it.

#[cfg(test)]
use crate::sections::marker;
use crate::sections::{script, split_sections};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub serial: String,
    /// The name people know ("Redmi 5 Plus"), from the market name when the phone has one.
    pub name: String,
    pub brand: String,
    pub manufacturer: String,
    pub model: String,
    pub codename: String,
    /// "8.1.0"
    pub android: String,
    pub sdk: u32,
    /// The maker's skin and its version: "MIUI 12", "One UI 6.1", "ColorOS V13"; empty for others.
    pub skin: String,
    pub build: String,
    pub battery: Option<u8>,
    /// The home screen app in use; all of them when the phone has no default.
    pub launchers: Vec<String>,
    /// The keyboard in use.
    pub keyboard: Option<String>,
}

/// The shell script `info` runs. Commands a phone lacks print nothing, and the rest still works.
pub fn info_script() -> String {
    script(&[
        ("props", "getprop"),
        ("ime", "settings get secure default_input_method 2>/dev/null"),
        ("home", "cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME 2>/dev/null"),
        ("homes", "cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.HOME 2>/dev/null"),
        ("battery", "dumpsys battery 2>/dev/null"),
    ])
}

/// `[ro.product.model]: [Redmi 5 Plus]` lines.
pub fn parse_props(text: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix('[') else {
            continue;
        };
        let Some((key, rest)) = rest.split_once("]: [") else {
            continue;
        };
        let value = rest.strip_suffix(']').unwrap_or(rest);
        props.insert(key.to_string(), value.to_string());
    }
    props
}

/// The package of a component line ("com.miui.home/.launcher.Launcher"), skipping the lines
/// `--brief` prints around it.
fn component_packages(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains(' ') && l.contains('/'))
        .filter_map(|l| l.split_once('/').map(|(p, _)| p.to_string()))
        .collect()
}

fn skin(props: &HashMap<String, String>) -> String {
    let get = |k: &str| props.get(k).map(String::as_str).unwrap_or("").trim();
    if !get("ro.mi.os.version.name").is_empty() {
        return format!("HyperOS {}", get("ro.mi.os.version.name").trim_start_matches("OS"));
    }
    if !get("ro.miui.ui.version.name").is_empty() {
        // "V12", and from 12.5 on three digits: "V125" is 12.5, "V140" is 14.
        let v = get("ro.miui.ui.version.name").trim_start_matches('V');
        return match v.len() {
            3 if v.ends_with('0') => format!("MIUI {}", &v[..2]),
            3 => format!("MIUI {}.{}", &v[..2], &v[2..]),
            _ => format!("MIUI {v}"),
        };
    }
    if let Ok(v) = get("ro.build.version.oneui").parse::<u32>() {
        // 60101 is One UI 6.1.1; a zero patch is left out.
        let (major, minor, patch) = (v / 10000, v / 100 % 100, v % 100);
        return if patch == 0 {
            format!("One UI {major}.{minor}")
        } else {
            format!("One UI {major}.{minor}.{patch}")
        };
    }
    if !get("ro.build.version.opporom").is_empty() {
        return format!("ColorOS {}", get("ro.build.version.opporom"));
    }
    if !get("ro.oxygen.version").is_empty() {
        return format!("OxygenOS {}", get("ro.oxygen.version"));
    }
    String::new()
}

pub fn parse_info(serial: &str, text: &str) -> DeviceInfo {
    let parts = split_sections(text);
    let section = |name: &str| parts.get(name).map(String::as_str).unwrap_or("");
    let props = parse_props(section("props"));
    let get = |k: &str| props.get(k).cloned().unwrap_or_default();

    let model = get("ro.product.model");
    let market = [
        "ro.product.marketname",
        "ro.product.vendor.marketname",
        "ro.config.marketing_name",
    ]
    .iter()
    .map(|k| get(k))
    .find(|v| !v.trim().is_empty());
    let name = market.unwrap_or_else(|| model.clone());

    let keyboard = section("ime")
        .lines()
        .map(str::trim)
        .find(|l| l.contains('/'))
        .and_then(|l| l.split_once('/'))
        .map(|(p, _)| p.to_string());

    // With no default home app, resolve-activity names the chooser ("android/...ResolverActivity");
    // then every home app is protected, so the phone keeps at least one.
    let default_home = component_packages(section("home"))
        .into_iter()
        .next()
        .filter(|p| p != "android");
    let launchers = match default_home {
        Some(p) => vec![p],
        None => {
            let mut all = component_packages(section("homes"));
            all.retain(|p| p != "android");
            all.dedup();
            all
        }
    };

    let battery = section("battery")
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("level:"))
        .and_then(|v| v.trim().parse::<u8>().ok());

    DeviceInfo {
        serial: serial.to_string(),
        name,
        brand: get("ro.product.brand"),
        manufacturer: get("ro.product.manufacturer"),
        model,
        codename: get("ro.product.device"),
        android: get("ro.build.version.release"),
        sdk: get("ro.build.version.sdk").parse().unwrap_or(0),
        skin: skin(&props),
        build: get("ro.build.display.id"),
        battery,
        launchers,
        keyboard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(props: &str, ime: &str, home: &str, homes: &str) -> String {
        format!(
            "{}\n{props}\n{}\n{ime}\n{}\n{home}\n{}\n{homes}\n{}\nCurrent Battery Service state:\n  AC powered: false\n  level: 84\n  scale: 100\n",
            marker("props"),
            marker("ime"),
            marker("home"),
            marker("homes"),
            marker("battery")
        )
    }

    const VINCE: &str = "[ro.build.display.id]: [OPM1.171019.019]\n\
                         [ro.build.version.release]: [8.1.0]\n\
                         [ro.build.version.sdk]: [27]\n\
                         [ro.miui.ui.version.name]: [V12]\n\
                         [ro.product.brand]: [xiaomi]\n\
                         [ro.product.device]: [vince]\n\
                         [ro.product.manufacturer]: [Xiaomi]\n\
                         [ro.product.model]: [Redmi 5 Plus]\n\
                         [persist.sys.note]: [a value with ]: [ inside]\n";

    #[test]
    fn reads_a_xiaomi_phone() {
        let out = text(
            VINCE,
            "com.google.android.inputmethod.latin/com.android.inputmethod.latin.LatinIME",
            "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\ncom.miui.home/.launcher.Launcher",
            "",
        );
        let info = parse_info("4b2a1c3d", &out);
        assert_eq!(info.name, "Redmi 5 Plus");
        assert_eq!(info.brand, "xiaomi");
        assert_eq!(info.codename, "vince");
        assert_eq!(info.android, "8.1.0");
        assert_eq!(info.sdk, 27);
        assert_eq!(info.skin, "MIUI 12");
        assert_eq!(info.battery, Some(84));
        assert_eq!(info.launchers, vec!["com.miui.home"]);
        assert_eq!(info.keyboard.as_deref(), Some("com.google.android.inputmethod.latin"));
    }

    #[test]
    fn protects_every_home_app_when_none_is_the_default() {
        let out = text(
            "[ro.product.model]: [SM-A525F]\n[ro.product.marketname]: [Galaxy A52]\n[ro.build.version.oneui]: [60101]\n",
            "null",
            "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false\nandroid/com.android.internal.app.ResolverActivity",
            "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false\ncom.sec.android.app.launcher/.activities.LauncherActivity\npriority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false\nbitpit.launcher/.ui.HomeActivity\n",
        );
        let info = parse_info("R58M", &out);
        assert_eq!(info.name, "Galaxy A52");
        assert_eq!(info.skin, "One UI 6.1.1");
        assert_eq!(info.launchers, vec!["com.sec.android.app.launcher", "bitpit.launcher"]);
        assert_eq!(info.keyboard, None);
    }

    #[test]
    fn an_old_phone_without_cmd_still_has_its_properties() {
        let info = parse_info(
            "x",
            &text(
                "[ro.product.model]: [Nexus 5]\n[ro.build.version.sdk]: [23]\n",
                "",
                "",
                "",
            ),
        );
        assert_eq!(info.name, "Nexus 5");
        assert_eq!(info.sdk, 23);
        assert!(info.launchers.is_empty());
        assert_eq!(info.skin, "");
    }

    #[test]
    fn reads_the_newer_skins() {
        let props = |s: &str| parse_props(s);
        assert_eq!(
            skin(&props(
                "[ro.mi.os.version.name]: [OS2.0]\n[ro.miui.ui.version.name]: [V816]\n"
            )),
            "HyperOS 2.0"
        );
        assert_eq!(skin(&props("[ro.build.version.oneui]: [50000]\n")), "One UI 5.0");
        assert_eq!(skin(&props("[ro.miui.ui.version.name]: [V125]\n")), "MIUI 12.5");
        assert_eq!(skin(&props("[ro.miui.ui.version.name]: [V140]\n")), "MIUI 14");
        assert_eq!(skin(&props("[ro.build.version.opporom]: [V13.1]\n")), "ColorOS V13.1");
    }

    #[test]
    fn a_value_may_hold_brackets() {
        let p = parse_props(VINCE);
        assert_eq!(p["persist.sys.note"], "a value with ]: [ inside");
    }
}
