//! The simulated phone as a program that answers like adb, for the tests that start a real
//! process (and so check how arguments reach it, on Windows too). The phone's state lives in the
//! JSON file named by FAKE_ADB_STATE, so it survives from one call to the next.

use adb_core::runner::Runner;
use adb_core::simulator::{Phone, Simulator};
use std::time::Duration;

fn main() {
    let state = std::env::var("FAKE_ADB_STATE").expect("FAKE_ADB_STATE names the phone's state file");
    let phone: Phone = std::fs::read_to_string(&state)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(Phone::redmi);
    let sim = Simulator::new(phone);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = sim
        .run(&args, Duration::from_secs(5))
        .expect("the simulator always answers");
    let phone = sim.phone.into_inner().unwrap();
    std::fs::write(&state, serde_json::to_string(&phone).unwrap()).expect("state saved");
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    std::process::exit(out.code.unwrap_or(1));
}
