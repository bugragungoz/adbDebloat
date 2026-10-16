//! Every batch adbDebloat ran, newest first, so it can be read back and undone.

use crate::actions::Action;
use crate::device::ItemResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// How many batches are kept.
pub const KEEP: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDevice {
    pub serial: String,
    pub name: String,
    pub codename: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    /// Milliseconds since 1970.
    pub at: u64,
    pub device: SessionDevice,
    pub action: Action,
    pub items: Vec<ItemResult>,
}

impl Session {
    /// The packages to restore to undo this batch: those it removed or disabled. Restores are not
    /// undone this way; remove the package again instead.
    pub fn undo(&self) -> Option<Vec<String>> {
        if self.action == Action::Restore {
            return None;
        }
        let packages: Vec<String> = self.items.iter().filter(|i| i.ok).map(|i| i.package.clone()).collect();
        (!packages.is_empty()).then_some(packages)
    }
}

pub struct History {
    path: PathBuf,
}

impl History {
    pub fn new(dir: &Path) -> Self {
        History {
            path: dir.join("history.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved batches; an unreadable file is kept aside as history.json.broken and the history
    /// starts again, rather than failing every time.
    pub fn load(&self) -> Vec<Session> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        match serde_json::from_str(&text) {
            Ok(sessions) => sessions,
            Err(_) => {
                let _ = fs::rename(&self.path, self.path.with_extension("json.broken"));
                Vec::new()
            }
        }
    }

    /// Adds a batch at the top and saves, through a temporary file so a crash never leaves half a
    /// file behind.
    pub fn add(&self, session: Session) -> io::Result<Vec<Session>> {
        let mut sessions = self.load();
        sessions.insert(0, session);
        sessions.truncate(KEEP);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&sessions).map_err(io::Error::other)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::PackageState;

    fn item(package: &str, ok: bool) -> ItemResult {
        ItemResult {
            package: package.into(),
            label: None,
            before: Some(PackageState::Installed),
            ok,
            failure: None,
        }
    }

    fn session(id: &str, action: Action, items: Vec<ItemResult>) -> Session {
        Session {
            id: id.into(),
            at: 1,
            device: SessionDevice {
                serial: "s".into(),
                name: "Redmi 5 Plus".into(),
                codename: "vince".into(),
            },
            action,
            items,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("adbdebloat-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn undo_restores_what_worked() {
        let s = session(
            "1",
            Action::Remove,
            vec![item("a.b", true), item("c.d", false), item("e.f", true)],
        );
        assert_eq!(s.undo(), Some(vec!["a.b".to_string(), "e.f".to_string()]));
        assert_eq!(session("2", Action::Remove, vec![item("a.b", false)]).undo(), None);
        assert_eq!(session("3", Action::Restore, vec![item("a.b", true)]).undo(), None);
    }

    #[test]
    fn keeps_the_newest_first_and_survives_a_broken_file() {
        let dir = temp_dir("history");
        let h = History::new(&dir);
        assert!(h.load().is_empty());
        h.add(session("1", Action::Remove, vec![item("a.b", true)])).unwrap();
        let all = h.add(session("2", Action::Disable, vec![item("c.d", true)])).unwrap();
        assert_eq!(all.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["2", "1"]);
        assert_eq!(h.load(), all);

        fs::write(h.path(), "{ not json").unwrap();
        assert!(h.load().is_empty());
        assert!(dir.join("history.json.broken").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
