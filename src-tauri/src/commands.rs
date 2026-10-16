//! The commands the window calls. adb calls block, so each runs on a worker thread.

use crate::settings::Settings;
use crate::state::{now_ms, AdbTool, AppState};
use adb_core::actions::Action;
use adb_core::device::{run_batch, Adb, AdbError, ItemResult};
use adb_core::devices::DeviceEntry;
use adb_core::fetch::fetch;
use adb_core::history::{Session, SessionDevice};
use adb_core::info::DeviceInfo;
use adb_core::knowledge::{KnowledgeStatus, Removal};
use adb_core::packages::PackageState;
use adb_core::safety::{assess, Protection, Verdict};
use adb_core::tools::{self, DOWNLOAD_URL};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// An error the window shows: a code it translates and the details.
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub code: &'static str,
    pub detail: String,
}

impl CmdError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        CmdError {
            code,
            detail: detail.into(),
        }
    }
}

impl From<AdbError> for CmdError {
    fn from(e: AdbError) -> Self {
        match e {
            AdbError::Invalid(d) => CmdError::new("invalid", d),
            other => CmdError::new("adb_failed", other.to_string()),
        }
    }
}

type Shared<'a> = State<'a, Arc<AppState>>;

/// Runs blocking work off the window's thread.
async fn blocking<T: Send + 'static>(
    state: &Shared<'_>,
    work: impl FnOnce(Arc<AppState>) -> Result<T, CmdError> + Send + 'static,
) -> Result<T, CmdError> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || work(state))
        .await
        .map_err(|e| CmdError::new("internal", e.to_string()))?
}

fn with_adb<T>(state: &AppState, work: impl FnOnce(&Adb) -> Result<T, CmdError>) -> Result<T, CmdError> {
    let runner = state.runner().map_err(|e| CmdError::new("no_adb", e))?;
    work(&Adb::new(&runner))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdbProblem {
    /// "missing" (no adb found) or "broken" (found, but it does not run).
    code: &'static str,
    detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    adb: Option<AdbTool>,
    adb_problem: Option<AdbProblem>,
    devices: Vec<DeviceEntry>,
    knowledge: KnowledgeStatus,
    busy: bool,
    downloading: bool,
}

/// adb, the phones it sees and the package list in use. The window asks every two seconds.
#[tauri::command]
pub async fn get_status(state: Shared<'_>) -> Result<Status, CmdError> {
    blocking(&state, |s| {
        let (adb, problem) = match s.resolve_adb() {
            Ok(tool) => (Some(tool), None),
            Err(detail) if detail.is_empty() => (
                None,
                Some(AdbProblem {
                    code: "missing",
                    detail,
                }),
            ),
            Err(detail) => (None, Some(AdbProblem { code: "broken", detail })),
        };
        let devices = match &adb {
            Some(tool) => {
                let runner = adb_core::runner::Process::new(&tool.path);
                Adb::new(&runner).devices().unwrap_or_default()
            }
            None => Vec::new(),
        };
        Ok(Status {
            adb,
            adb_problem: problem,
            devices,
            knowledge: s.knowledge.read().unwrap().status().clone(),
            busy: s.busy.load(Ordering::Relaxed),
            downloading: s.downloading.load(Ordering::Relaxed),
        })
    })
    .await
}

fn read_device(s: &AppState, serial: &str) -> Result<DeviceInfo, CmdError> {
    let info = with_adb(s, |adb| Ok(adb.info(serial)?))?;
    s.devices.lock().unwrap().insert(serial.to_string(), info.clone());
    Ok(info)
}

#[tauri::command]
pub async fn get_device(state: Shared<'_>, serial: String) -> Result<DeviceInfo, CmdError> {
    blocking(&state, move |s| read_device(&s, &serial)).await
}

/// A package as the window shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRow {
    name: String,
    label: Option<String>,
    path: Option<String>,
    system: bool,
    state: PackageState,
    removal: Option<Removal>,
    list: Option<String>,
    description: Option<String>,
    dependencies: Vec<String>,
    needed_by: Vec<String>,
    /// "allowed", "needsExpert" or "protected".
    verdict: &'static str,
    protection: Option<Protection>,
}

#[tauri::command]
pub async fn get_packages(state: Shared<'_>, serial: String) -> Result<Vec<AppRow>, CmdError> {
    blocking(&state, move |s| {
        let info = read_device(&s, &serial)?;
        let packages = with_adb(&s, |adb| Ok(adb.packages(&serial)?))?;
        let expert = s.settings().expert_mode;
        let knowledge = s.knowledge.read().unwrap();
        Ok(packages
            .into_iter()
            .map(|p| {
                let entry = knowledge.get(&p.name);
                let (verdict, protection) = match assess(&p.name, entry, &info, expert) {
                    Verdict::Allowed => ("allowed", None),
                    Verdict::NeedsExpert => ("needsExpert", None),
                    Verdict::Protected(why) => ("protected", Some(why)),
                };
                AppRow {
                    label: entry.and_then(|e| e.label.clone()),
                    removal: entry.map(|e| e.removal),
                    list: entry.map(|e| e.list.clone()),
                    description: entry.map(|e| e.description.clone()),
                    dependencies: entry.map(|e| e.dependencies.clone()).unwrap_or_default(),
                    needed_by: entry.map(|e| e.needed_by.clone()).unwrap_or_default(),
                    name: p.name,
                    path: p.path,
                    system: p.system,
                    state: p.state,
                    verdict,
                    protection,
                }
            })
            .collect())
    })
    .await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    index: usize,
    total: usize,
    item: ItemResult,
}

/// Runs an action on packages the owner reviewed, one at a time, telling the window about each,
/// and saves the batch in the history. The safety rules are checked again here.
#[tauri::command]
pub async fn apply(
    app: AppHandle,
    state: Shared<'_>,
    serial: String,
    action: Action,
    packages: Vec<String>,
) -> Result<Session, CmdError> {
    blocking(&state, move |s| {
        if s.busy.swap(true, Ordering::SeqCst) {
            return Err(CmdError::new("busy", ""));
        }
        s.stop.store(false, Ordering::SeqCst);
        let result = (|| {
            let info = read_device(&s, &serial)?;
            let expert = s.settings().expert_mode;
            let total = packages.len();
            let items = with_adb(&s, |adb| {
                let knowledge = s.knowledge.read().unwrap();
                Ok(run_batch(
                    adb,
                    &serial,
                    action,
                    &packages,
                    &knowledge,
                    &info,
                    expert,
                    &s.stop,
                    &mut |index, item| {
                        let _ = app.emit(
                            "batch-progress",
                            Progress {
                                index,
                                total,
                                item: item.clone(),
                            },
                        );
                    },
                )?)
            })?;
            let at = now_ms();
            let session = Session {
                id: at.to_string(),
                at,
                device: SessionDevice {
                    serial: serial.clone(),
                    name: info.name.clone(),
                    codename: info.codename.clone(),
                },
                action,
                items,
            };
            s.history
                .add(session.clone())
                .map_err(|e| CmdError::new("history_failed", e.to_string()))?;
            Ok(session)
        })();
        s.busy.store(false, Ordering::SeqCst);
        result
    })
    .await
}

/// Ends the running batch before its next package.
#[tauri::command]
pub fn stop_batch(state: Shared<'_>) {
    state.stop.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn get_history(state: Shared<'_>) -> Result<Vec<Session>, CmdError> {
    blocking(&state, |s| Ok(s.history.load())).await
}

#[tauri::command]
pub async fn pair(state: Shared<'_>, address: String, code: String) -> Result<String, CmdError> {
    blocking(&state, move |s| with_adb(&s, |adb| Ok(adb.pair(&address, &code)?))).await
}

#[tauri::command]
pub async fn connect(state: Shared<'_>, address: String) -> Result<String, CmdError> {
    blocking(&state, move |s| with_adb(&s, |adb| Ok(adb.connect(&address)?))).await
}

#[tauri::command]
pub async fn disconnect(state: Shared<'_>, serial: String) -> Result<(), CmdError> {
    blocking(&state, move |s| with_adb(&s, |adb| Ok(adb.disconnect(&serial)?))).await
}

#[tauri::command]
pub async fn switch_to_wifi(state: Shared<'_>, serial: String) -> Result<String, CmdError> {
    blocking(&state, move |s| with_adb(&s, |adb| Ok(adb.switch_to_wifi(&serial)?))).await
}

#[tauri::command]
pub async fn reboot(state: Shared<'_>, serial: String) -> Result<(), CmdError> {
    blocking(&state, move |s| {
        if s.busy.load(Ordering::SeqCst) {
            return Err(CmdError::new("busy", ""));
        }
        with_adb(&s, |adb| Ok(adb.reboot(&serial)?))
    })
    .await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    done: u64,
    total: Option<u64>,
}

/// Downloads Google's platform-tools into the app's data folder and uses its adb.
#[tauri::command]
pub async fn download_platform_tools(app: AppHandle, state: Shared<'_>) -> Result<AdbTool, CmdError> {
    blocking(&state, move |s| {
        if s.downloading.swap(true, Ordering::SeqCst) {
            return Err(CmdError::new("busy", ""));
        }
        let result = (|| {
            let zip = fetch(DOWNLOAD_URL, 200_000_000, &mut |done, total| {
                let _ = app.emit("download-progress", DownloadProgress { done, total });
            })
            .map_err(|e| CmdError::new("download_failed", e.to_string()))?;
            // A running adb server from the old copy would keep its files locked.
            if let Ok(runner) = s.runner() {
                let _ = Adb::new(&runner).kill_server();
            }
            tools::install(&zip, &s.data_dir).map_err(|e| CmdError::new("download_failed", e.to_string()))?;
            s.forget_adb();
            s.resolve_adb().map_err(|e| CmdError::new("no_adb", e))
        })();
        s.downloading.store(false, Ordering::SeqCst);
        result
    })
    .await
}

#[tauri::command]
pub async fn refresh_knowledge(state: Shared<'_>) -> Result<KnowledgeStatus, CmdError> {
    blocking(&state, |s| {
        s.refresh_community().map_err(|e| CmdError::new("list_failed", e))?;
        Ok(s.knowledge.read().unwrap().status().clone())
    })
    .await
}

#[tauri::command]
pub fn get_settings(state: Shared<'_>) -> Settings {
    state.settings()
}

#[tauri::command]
pub async fn set_settings(state: Shared<'_>, settings: Settings) -> Result<Settings, CmdError> {
    blocking(&state, move |s| {
        let before = s.settings();
        s.store
            .lock()
            .unwrap()
            .save(settings)
            .map_err(|e| CmdError::new("settings_not_saved", e))?;
        let after = s.settings();
        if after.adb_path != before.adb_path {
            s.forget_adb();
        }
        if after.community_list != before.community_list {
            if !after.community_list {
                s.use_builtin();
            } else if !s.use_saved_community() {
                let _ = s.refresh_community();
            }
        }
        Ok(after)
    })
    .await
}
