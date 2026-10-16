//! adbDebloat against a separate program that answers like adb (src/bin/fake-adb.rs), started the
//! way the app starts the real one: the shell scripts must reach it as one argument, on Windows too.

use adb_core::actions::{Action, FailureCode};
use adb_core::device::{run_batch, Adb};
use adb_core::knowledge::Knowledge;
use adb_core::packages::PackageState;
use adb_core::runner::Process;
use adb_core::simulator::Phone;
use std::sync::atomic::AtomicBool;

#[test]
fn the_whole_flow_through_a_real_process() {
    let state = std::env::temp_dir().join(format!("adbdebloat-fake-adb-{}.json", std::process::id()));
    std::fs::write(&state, serde_json::to_string(&Phone::redmi()).unwrap()).unwrap();
    // SAFETY: set before any other thread of this test binary reads the environment.
    unsafe { std::env::set_var("FAKE_ADB_STATE", &state) };
    let runner = Process::new(env!("CARGO_BIN_EXE_fake-adb"));
    let adb = Adb::new(&runner);

    assert_eq!(adb.version().unwrap(), "35.0.2");
    assert_eq!(adb.devices().unwrap()[0].serial, "4b2a1c3d");
    let info = adb.info("4b2a1c3d").unwrap();
    assert_eq!(info.name, "Redmi 5 Plus");
    assert_eq!(info.launchers, vec!["com.miui.home"]);
    assert_eq!(adb.packages("4b2a1c3d").unwrap().len(), 12);

    let stop = AtomicBool::new(false);
    let asked = vec!["com.miui.msa.global".to_string(), "com.miui.home".to_string()];
    let r = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Remove,
        &asked,
        &Knowledge::builtin(),
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(r[0].ok, "{r:?}");
    assert_eq!(r[1].failure.as_ref().unwrap().code, FailureCode::Protected);
    let after = adb.packages("4b2a1c3d").unwrap();
    assert_eq!(
        after.iter().find(|p| p.name == "com.miui.msa.global").unwrap().state,
        PackageState::Removed
    );

    let r = run_batch(
        &adb,
        "4b2a1c3d",
        Action::Restore,
        &asked[..1],
        &Knowledge::builtin(),
        &info,
        false,
        &stop,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(r[0].ok, "{r:?}");

    // A phone that is not there: adb's complaint, not a parse failure.
    assert!(adb.info("ZZZ").is_err());
    let _ = std::fs::remove_file(&state);
}
