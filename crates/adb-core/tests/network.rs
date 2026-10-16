//! Downloads from the real servers. Ignored by default (they need the internet); CI runs them with
//! --include-ignored.

use adb_core::device::Adb;
use adb_core::fetch::fetch;
use adb_core::knowledge::{parse_community, Knowledge, Removal, COMMUNITY_URL};
use adb_core::runner::Process;
use adb_core::tools::{install, DOWNLOAD_URL};

#[test]
#[ignore = "downloads the community list"]
fn the_community_list_downloads_and_reads() {
    let bytes = fetch(COMMUNITY_URL, 30_000_000, &mut |_, _| {}).unwrap();
    let entries = parse_community(&String::from_utf8(bytes).unwrap()).unwrap();
    assert!(entries.len() > 4000, "{}", entries.len());
    let k = Knowledge::with_community(entries, None);
    assert_eq!(k.get("com.android.systemui").unwrap().removal, Removal::Unsafe);
    assert!(k.get("com.miui.msa.global").unwrap().label.is_some());
}

#[test]
#[ignore = "downloads platform-tools from Google"]
fn platform_tools_download_install_and_run() {
    let mut last = (0, None);
    let zip = fetch(DOWNLOAD_URL, 200_000_000, &mut |done, total| last = (done, total)).unwrap();
    assert_eq!(last.0, zip.len() as u64);
    let dir = std::env::temp_dir().join(format!("adbdebloat-net-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let adb_path = install(&zip, &dir).unwrap();
    let runner = Process::new(&adb_path);
    let adb = Adb::new(&runner);
    let version = adb.version().unwrap();
    assert!(version.split('.').count() >= 3, "{version}");
    // No phone is attached to a CI runner, but listing must work.
    assert!(adb.devices().unwrap().iter().all(|d| !d.serial.is_empty()));
    let _ = adb.kill_server();
    let _ = std::fs::remove_dir_all(&dir);
}
