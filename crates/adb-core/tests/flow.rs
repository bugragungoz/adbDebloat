//! The whole flow against a simulated phone: find it, read it, list its packages, change them
//! within the safety rules, and undo.

use adb_core::actions::{Action, FailureCode};
use adb_core::device::{run_batch, Adb, AdbError};
use adb_core::devices::DeviceState;
use adb_core::knowledge::Knowledge;
use adb_core::packages::PackageState;
use adb_core::simulator::{Phone, Simulator};
use std::sync::atomic::AtomicBool;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn finds_and_reads_the_phone() {
    let sim = Simulator::new(Phone::redmi());
    let adb = Adb::new(&sim);
    assert_eq!(adb.version().unwrap(), "35.0.2");
    let devices = adb.devices().unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].state, DeviceState::Ready);

    let info = adb.info("4b2a1c3d").unwrap();
    assert_eq!(info.name, "Redmi 5 Plus");
    assert_eq!(info.skin, "MIUI 12");
    assert_eq!(info.battery, Some(76));
    assert_eq!(info.launchers, vec!["com.miui.home"]);
    assert_eq!(info.keyboard.as_deref(), Some("com.google.android.inputmethod.latin"));

    let packages = adb.packages("4b2a1c3d").unwrap();
    assert_eq!(packages.len(), 12);
    let spotify = packages.iter().find(|p| p.name == "com.spotify.music").unwrap();
    assert!(!spotify.system);
    assert_eq!(spotify.path.as_deref(), Some("/data/app/com.spotify.music/base.apk"));
}

#[test]
fn removes_within_the_rules_and_undoes() {
    let sim = Simulator::new(Phone::redmi());
    let adb = Adb::new(&sim);
    let info = adb.info("4b2a1c3d").unwrap();
    let knowledge = Knowledge::builtin();
    let stop = AtomicBool::new(false);
    let mut seen = Vec::new();

    let asked = names(&[
        "com.miui.msa.global",
        "com.miui.analytics",
        "com.miui.home",                        // the launcher in use
        "com.google.android.inputmethod.latin", // the keyboard in use
        "com.android.systemui",                 // core
        "com.miui.powerkeeper",                 // Expert, expert mode off
        "com.xiaomi.finddevice",                // a device administrator (and Expert)
        "com.not.on.this.phone",
    ]);
    let results = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Remove,
        &asked,
        &knowledge,
        &info,
        false,
        &stop,
        &mut |i, r| seen.push((i, r.package.clone())),
    )
    .unwrap();
    assert_eq!(seen.len(), asked.len());

    let code = |p: &str| {
        results
            .iter()
            .find(|r| r.package == p)
            .unwrap()
            .failure
            .as_ref()
            .map(|f| f.code)
    };
    assert_eq!(code("com.miui.msa.global"), None);
    assert_eq!(code("com.miui.analytics"), None);
    assert_eq!(code("com.miui.home"), Some(FailureCode::Protected));
    assert_eq!(
        code("com.google.android.inputmethod.latin"),
        Some(FailureCode::Protected)
    );
    assert_eq!(code("com.android.systemui"), Some(FailureCode::Protected));
    assert_eq!(code("com.miui.powerkeeper"), Some(FailureCode::NeedsExpert));
    assert_eq!(code("com.xiaomi.finddevice"), Some(FailureCode::NeedsExpert));
    assert_eq!(code("com.not.on.this.phone"), Some(FailureCode::Unknown));
    let msa = results.iter().find(|r| r.package == "com.miui.msa.global").unwrap();
    assert_eq!(msa.label.as_deref(), Some("MIUI System Ads"));
    assert_eq!(msa.before, Some(PackageState::Installed));

    assert_eq!(sim.state("com.miui.msa.global"), Some(PackageState::Removed));
    assert_eq!(sim.state("com.miui.home"), Some(PackageState::Installed));
    assert_eq!(sim.state("com.android.systemui"), Some(PackageState::Installed));
    // Protected packages never reached the phone.
    assert!(!sim
        .log
        .lock()
        .unwrap()
        .iter()
        .any(|c| c.contains("uninstall -k --user 0 com.android.systemui")));

    // Expert mode lets the Expert one through; the device administrator is refused by Android.
    let results = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Remove,
        &names(&["com.miui.powerkeeper", "com.xiaomi.finddevice"]),
        &knowledge,
        &info,
        true,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(results[0].ok);
    assert_eq!(results[1].failure.as_ref().unwrap().code, FailureCode::DeviceAdmin);

    // Undo: restore what the first batch removed.
    let restore = names(&["com.miui.msa.global", "com.miui.analytics", "com.miui.powerkeeper"]);
    let results = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Restore,
        &restore,
        &knowledge,
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(results.iter().all(|r| r.ok), "{results:?}");
    assert_eq!(sim.state("com.miui.msa.global"), Some(PackageState::Installed));
    assert_eq!(sim.state("com.miui.powerkeeper"), Some(PackageState::Installed));
}

#[test]
fn disable_where_the_maker_allows_it() {
    let sim = Simulator::new(Phone::redmi());
    let adb = Adb::new(&sim);
    let info = adb.info("4b2a1c3d").unwrap();
    let stop = AtomicBool::new(false);
    let results = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Disable,
        &names(&["com.google.android.youtube", "com.miui.weather2"]),
        &Knowledge::builtin(),
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(results[0].ok);
    assert_eq!(sim.state("com.google.android.youtube"), Some(PackageState::Disabled));
    assert_eq!(results[1].failure.as_ref().unwrap().code, FailureCode::DisableRefused);

    // A disabled package is switched back on, and disabling it again has nothing to do.
    let restore = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Restore,
        &names(&["com.google.android.youtube"]),
        &Knowledge::builtin(),
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(restore[0].ok);
    assert_eq!(sim.state("com.google.android.youtube"), Some(PackageState::Installed));
    let again = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Restore,
        &names(&["com.google.android.youtube"]),
        &Knowledge::builtin(),
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(again[0].failure.as_ref().unwrap().code, FailureCode::NothingToDo);
}

#[test]
fn stops_when_the_phone_goes_away_and_when_asked() {
    let sim = Simulator::new(Phone::redmi());
    let adb = Adb::new(&sim);
    let info = adb.info("4b2a1c3d").unwrap();
    let knowledge = Knowledge::builtin();

    // Asked to stop: nothing more is sent.
    let stop = AtomicBool::new(true);
    let r = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Remove,
        &names(&["com.miui.analytics"]),
        &knowledge,
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(r[0].failure.as_ref().unwrap().code, FailureCode::NoAnswer);
    assert_eq!(sim.state("com.miui.analytics"), Some(PackageState::Installed));

    // The cable comes out after the list was read: the first two commands get no answer, and
    // the batch does not try the rest.
    sim.phone.lock().unwrap().unplug_after_list = true;
    let asked = names(&[
        "com.miui.analytics",
        "com.miui.msa.global",
        "com.google.android.youtube",
        "com.spotify.music",
    ]);
    let r = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Remove,
        &asked,
        &knowledge,
        &info,
        false,
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .unwrap();
    assert!(
        r.iter()
            .all(|i| i.failure.as_ref().unwrap().code == FailureCode::NoAnswer),
        "{r:?}"
    );
    let sent = sim
        .log
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c.contains("pm uninstall"))
        .count();
    assert_eq!(sent, 2);
    assert!(matches!(adb.packages("4b2a1c3d"), Err(AdbError::Failed(_))));
}

#[test]
fn wireless_debugging() {
    let sim = Simulator::new(Phone::redmi());
    let adb = Adb::new(&sim);
    assert!(adb.pair("192.168.1.23:37145", "123456").is_ok());
    assert!(matches!(
        adb.pair("192.168.1.23:37145", "000000"),
        Err(AdbError::Failed(_))
    ));
    assert!(matches!(
        adb.pair("not an address", "123456"),
        Err(AdbError::Invalid(_))
    ));
    assert!(matches!(
        adb.pair("192.168.1.23:37145", "12345"),
        Err(AdbError::Invalid(_))
    ));
    assert!(adb.connect("192.168.1.23:5555").is_ok());
    assert_eq!(adb.switch_to_wifi("4b2a1c3d").unwrap(), "192.168.1.23:5555");
    let log = sim.log.lock().unwrap();
    assert!(log.contains(&"-s 4b2a1c3d tcpip 5555".to_string()));
    assert!(log.contains(&"connect 192.168.1.23:5555".to_string()));
}
