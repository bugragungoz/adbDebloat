//! The part of adbDebloat that knows about adb and Android, kept apart from the window so every
//! rule can be tested against a simulated phone.

pub mod actions;
pub mod device;
pub mod devices;
pub mod fetch;
pub mod history;
pub mod info;
pub mod knowledge;
pub mod packages;
pub mod runner;
pub mod safety;
mod sections;
pub mod simulator;
pub mod tools;
