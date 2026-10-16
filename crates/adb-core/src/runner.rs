//! Running adb. Everything else talks to a device through the `Runner` trait, so the rules can be
//! tested against a simulated phone (`crate::fake`) and the real program is only touched here.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// What a finished adb call printed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Output {
    /// The exit code; `None` when the process was killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn ok(stdout: impl Into<String>) -> Self {
        Output {
            code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// Both streams, as adb mixes what matters between them.
    pub fn text(&self) -> String {
        match (self.stdout.trim(), self.stderr.trim()) {
            (o, "") => o.to_string(),
            ("", e) => e.to_string(),
            (o, e) => format!("{o}\n{e}"),
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RunError {
    #[error("adb could not be started: {0}")]
    Start(String),
    #[error("adb did not answer within {0} seconds")]
    Timeout(u64),
}

pub trait Runner: Send + Sync {
    /// Runs adb with these arguments and waits at most `timeout`.
    fn run(&self, args: &[&str], timeout: Duration) -> Result<Output, RunError>;
}

/// Short calls (lists, properties, one package).
pub const SHORT: Duration = Duration::from_secs(20);
/// Calls that wait on the network or on a phone that is busy (pairing, connecting, a reboot).
pub const LONG: Duration = Duration::from_secs(60);

/// The real adb program.
#[derive(Debug, Clone)]
pub struct Process {
    pub path: PathBuf,
}

impl Process {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Process { path: path.into() }
    }
}

impl Runner for Process {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<Output, RunError> {
        run_program(&self.path, args, timeout)
    }
}

/// Starts a program without a console window, collects both streams on their own threads (so a
/// full pipe never blocks it) and kills it when it runs past `timeout`.
pub fn run_program(program: &Path, args: &[&str], timeout: Duration) -> Result<Output, RunError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| RunError::Start(e.to_string()))?;
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let out = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Timeout(timeout.as_secs()));
            }
            Ok(None) => thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(RunError::Start(e.to_string())),
        }
    };
    let text = |handle: thread::JoinHandle<Vec<u8>>| {
        let bytes = handle.join().unwrap_or_default();
        String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
    };
    Ok(Output {
        code: status.code(),
        stdout: text(out),
        stderr: text(err),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn shell() -> (PathBuf, Vec<&'static str>) {
        (PathBuf::from("/bin/sh"), vec!["-c"])
    }
    #[cfg(windows)]
    fn shell() -> (PathBuf, Vec<&'static str>) {
        (PathBuf::from("cmd.exe"), vec!["/C"])
    }

    #[test]
    fn collects_both_streams_and_the_exit_code() {
        let (sh, mut args) = shell();
        args.push("echo out&& echo err 1>&2&& exit 3");
        let out = run_program(&sh, &args, SHORT).unwrap();
        assert_eq!(out.stdout.trim(), "out");
        assert_eq!(out.stderr.trim(), "err");
        assert_eq!(out.code, Some(3));
        assert_eq!(out.text(), "out\nerr");
    }

    #[test]
    fn kills_a_call_that_runs_too_long() {
        let (sh, mut args) = shell();
        #[cfg(unix)]
        args.push("sleep 5");
        #[cfg(windows)]
        args.push("ping -n 6 127.0.0.1 > NUL");
        let started = Instant::now();
        let err = run_program(&sh, &args, Duration::from_millis(300)).unwrap_err();
        assert_eq!(err, RunError::Timeout(0));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn reports_a_missing_program() {
        let err = run_program(Path::new("/no/such/adb"), &["version"], SHORT).unwrap_err();
        assert!(matches!(err, RunError::Start(_)));
    }
}
