//! What the app holds while it runs: settings, the package knowledge, where adb is, the history,
//! and what it last learned about each phone.

use crate::settings::{Settings, Store};
use adb_core::device::Adb;
use adb_core::fetch::fetch;
use adb_core::history::History;
use adb_core::info::DeviceInfo;
use adb_core::knowledge::{parse_community, Knowledge, COMMUNITY_URL};
use adb_core::runner::Process;
use adb_core::tools::{self, AdbSource};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The community list is downloaded again when the copy is older than this.
const REFRESH_AFTER: Duration = Duration::from_secs(7 * 24 * 3600);
/// A complete list is about 2 MB.
const LIST_MAX_BYTES: u64 = 30_000_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdbTool {
    pub source: AdbSource,
    pub path: String,
    pub version: String,
}

pub struct AppState {
    pub store: Mutex<Store>,
    pub data_dir: PathBuf,
    pub knowledge: RwLock<Knowledge>,
    pub adb: Mutex<Option<AdbTool>>,
    pub history: History,
    /// A batch is running; a second one waits for it.
    pub busy: AtomicBool,
    pub stop: AtomicBool,
    /// The last information read from each phone, for the safety rules.
    pub devices: Mutex<HashMap<String, DeviceInfo>>,
    /// Something is downloading (the list or platform-tools).
    pub downloading: AtomicBool,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn community_file(data_dir: &Path) -> PathBuf {
    data_dir.join("community-packages.json")
}

impl AppState {
    pub fn new(config_dir: Option<PathBuf>, data_dir: PathBuf) -> Self {
        let store = Store::open(config_dir);
        let knowledge = if store.settings.community_list {
            load_community(&data_dir).unwrap_or_else(Knowledge::builtin)
        } else {
            Knowledge::builtin()
        };
        AppState {
            store: Mutex::new(store),
            history: History::new(&data_dir),
            data_dir,
            knowledge: RwLock::new(knowledge),
            adb: Mutex::new(None),
            busy: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            devices: Mutex::new(HashMap::new()),
            downloading: AtomicBool::new(false),
        }
    }

    pub fn settings(&self) -> Settings {
        self.store.lock().unwrap().settings.clone()
    }

    /// Finds adb and asks its version; `None` with the reason when there is none that works.
    pub fn resolve_adb(&self) -> Result<AdbTool, String> {
        if let Some(tool) = self.adb.lock().unwrap().clone() {
            return Ok(tool);
        }
        let custom = self.settings().adb_path;
        let custom = (!custom.is_empty()).then(|| PathBuf::from(custom));
        let (source, path) = tools::find(custom.as_deref(), &self.data_dir).ok_or_else(String::new)?;
        let runner = Process::new(&path);
        let version = Adb::new(&runner).version().map_err(|e| e.to_string())?;
        let tool = AdbTool {
            source,
            path: path.to_string_lossy().into_owned(),
            version,
        };
        *self.adb.lock().unwrap() = Some(tool.clone());
        Ok(tool)
    }

    /// Forgets the adb found, after the settings or platform-tools changed.
    pub fn forget_adb(&self) {
        *self.adb.lock().unwrap() = None;
    }

    pub fn runner(&self) -> Result<Process, String> {
        self.resolve_adb().map(|t| Process::new(t.path))
    }

    /// Whether the community list should be downloaded now: allowed, and missing or old.
    pub fn community_is_due(&self) -> bool {
        if !self.settings().community_list {
            return false;
        }
        let modified = std::fs::metadata(community_file(&self.data_dir))
            .and_then(|m| m.modified())
            .ok();
        modified.is_none_or(|m| m.elapsed().unwrap_or_default() > REFRESH_AFTER)
    }

    /// Downloads the community list, checks it, saves it and starts using it.
    pub fn refresh_community(&self) -> Result<(), String> {
        let bytes = fetch(COMMUNITY_URL, LIST_MAX_BYTES, &mut |_, _| {}).map_err(|e| e.to_string())?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let entries = parse_community(&text).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        let file = community_file(&self.data_dir);
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, &text)
            .and_then(|_| std::fs::rename(&tmp, &file))
            .map_err(|e| e.to_string())?;
        *self.knowledge.write().unwrap() = Knowledge::with_community(entries, Some(now_ms()));
        Ok(())
    }

    /// Goes back to the built-in list (the setting was switched off).
    pub fn use_builtin(&self) {
        *self.knowledge.write().unwrap() = Knowledge::builtin();
    }

    /// Uses the saved community list again (the setting was switched on).
    pub fn use_saved_community(&self) -> bool {
        match load_community(&self.data_dir) {
            Some(k) => {
                *self.knowledge.write().unwrap() = k;
                true
            }
            None => false,
        }
    }
}

fn load_community(data_dir: &Path) -> Option<Knowledge> {
    let file = community_file(data_dir);
    let text = std::fs::read_to_string(&file).ok()?;
    let entries = parse_community(&text).ok()?;
    let fetched = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    Some(Knowledge::with_community(entries, fetched))
}
