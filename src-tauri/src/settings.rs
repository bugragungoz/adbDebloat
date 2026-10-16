//! The owner's settings, kept as JSON in the app's configuration folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "system", "dark" or "light".
    pub theme: String,
    /// "system", "en" or "tr".
    pub language: String,
    /// Allows packages marked Expert. Unsafe ones and the core list stay protected.
    pub expert_mode: bool,
    /// Download the community package list and refresh it once a week.
    pub community_list: bool,
    /// An adb chosen by hand; empty to find one.
    pub adb_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "system".into(),
            expert_mode: false,
            community_list: true,
            adb_path: String::new(),
        }
    }
}

impl Settings {
    /// Settings from the window, with unknown values set back to their defaults.
    pub fn sanitized(mut self) -> Self {
        let defaults = Self::default();
        if !["system", "dark", "light"].contains(&self.theme.as_str()) {
            self.theme = defaults.theme;
        }
        if !["system", "en", "tr"].contains(&self.language.as_str()) {
            self.language = defaults.language;
        }
        // "Copy as path" in Explorer adds quotes.
        self.adb_path = self.adb_path.trim().trim_matches('"').trim().to_string();
        self
    }
}

pub struct Store {
    file: Option<PathBuf>,
    pub settings: Settings,
}

impl Store {
    pub fn open(config_dir: Option<PathBuf>) -> Self {
        let file = config_dir.map(|d| d.join("settings.json"));
        let settings = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .map(Settings::sanitized)
            .unwrap_or_default();
        Self { file, settings }
    }

    pub fn save(&mut self, settings: Settings) -> Result<(), String> {
        let settings = settings.sanitized();
        if let Some(file) = &self.file {
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
            std::fs::write(file, text).map_err(|e| e.to_string())?;
        }
        self.settings = settings;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_fall_back_and_paths_lose_their_quotes() {
        let s = Settings {
            theme: "neon".into(),
            language: "fr".into(),
            adb_path: "  \"C:\\platform-tools\\adb.exe\" ".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.theme, "system");
        assert_eq!(s.language, "system");
        assert_eq!(s.adb_path, "C:\\platform-tools\\adb.exe");
    }

    #[test]
    fn settings_survive_a_restart_and_missing_fields_take_defaults() {
        let dir = std::env::temp_dir().join(format!("adbdebloat-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Store::open(Some(dir.clone()));
        assert_eq!(store.settings, Settings::default());
        store
            .save(Settings {
                expert_mode: true,
                ..Settings::default()
            })
            .unwrap();
        assert!(Store::open(Some(dir.clone())).settings.expert_mode);

        std::fs::write(dir.join("settings.json"), r#"{"theme":"dark"}"#).unwrap();
        let partial = Store::open(Some(dir.clone()));
        assert_eq!(partial.settings.theme, "dark");
        assert!(partial.settings.community_list, "a missing field takes its default");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
