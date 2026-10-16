//! What is known about a package: a name people recognise, what it does and how safe it is to
//! remove.
//!
//! Two sources. adbDebloat's own list (`data/packages.json`, MIT) covers the common Google,
//! Xiaomi, Samsung and OPPO apps and works offline. The community list of Universal Android
//! Debloater Next Generation covers several thousand packages; it is GPL-3.0 and is downloaded
//! from its repository when the owner allows it, never shipped with the app. Where both know a
//! package, the community's description and level win and the label comes from ours.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How safe removing a package is. The levels and their meaning follow the community list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Removal {
    /// Safe to remove for almost everyone: ads, trackers, apps nobody needs.
    Recommended,
    /// Safe, but it removes something a person may use (a gallery, a keyboard, a store).
    Advanced,
    /// Can break important features or other apps. Needs expert mode.
    Expert,
    /// Can stop the phone from starting. adbDebloat never removes these.
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Who ships it: google, oem, aosp, carrier or misc.
    pub list: String,
    pub removal: Removal,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needed_by: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// Only adbDebloat's own list.
    Builtin,
    /// Ours merged with the community list.
    Community,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeStatus {
    pub source: SourceKind,
    pub packages: usize,
    /// When the community list was downloaded (milliseconds since 1970).
    pub fetched_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Knowledge {
    entries: HashMap<String, Entry>,
    status: KnowledgeStatus,
}

/// Where the community list is downloaded from.
pub const COMMUNITY_URL: &str = "https://raw.githubusercontent.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation/main/resources/assets/uad_lists.json";
/// Its project page, for the credit in the interface.
pub const COMMUNITY_PAGE: &str =
    "https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation";

const BUILTIN: &str = include_str!("../data/packages.json");

/// One entry of the community list as it is stored there.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommunityEntry {
    list: String,
    description: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    needed_by: Vec<String>,
    removal: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KnowledgeError {
    #[error("the community list could not be read: {0}")]
    Format(String),
    #[error("the community list has only {0} packages; a complete one has thousands")]
    TooSmall(usize),
}

fn removal_of(word: &str) -> Option<Removal> {
    match word.to_ascii_lowercase().as_str() {
        "recommended" => Some(Removal::Recommended),
        "advanced" => Some(Removal::Advanced),
        "expert" => Some(Removal::Expert),
        "unsafe" => Some(Removal::Unsafe),
        _ => None,
    }
}

/// Reads the community list. Entries with a level this version does not know are left out; a
/// list too small to be the real one is refused, so a broken download never replaces a good one.
pub fn parse_community(text: &str) -> Result<HashMap<String, Entry>, KnowledgeError> {
    let raw: HashMap<String, CommunityEntry> =
        serde_json::from_str(text).map_err(|e| KnowledgeError::Format(e.to_string()))?;
    let entries: HashMap<String, Entry> = raw
        .into_iter()
        .filter_map(|(name, e)| {
            let removal = removal_of(&e.removal)?;
            Some((
                name,
                Entry {
                    label: None,
                    list: e.list.to_ascii_lowercase(),
                    removal,
                    description: e.description.trim().to_string(),
                    dependencies: e.dependencies,
                    needed_by: e.needed_by,
                },
            ))
        })
        .collect();
    if entries.len() < 1000 {
        return Err(KnowledgeError::TooSmall(entries.len()));
    }
    Ok(entries)
}

impl Knowledge {
    /// adbDebloat's own list only.
    pub fn builtin() -> Self {
        let entries: HashMap<String, Entry> = serde_json::from_str(BUILTIN).expect("data/packages.json is valid");
        let status = KnowledgeStatus {
            source: SourceKind::Builtin,
            packages: entries.len(),
            fetched_at: None,
        };
        Knowledge { entries, status }
    }

    /// Ours with the community list on top.
    pub fn with_community(community: HashMap<String, Entry>, fetched_at: Option<u64>) -> Self {
        let mut merged = community;
        let ours: HashMap<String, Entry> = serde_json::from_str(BUILTIN).expect("data/packages.json is valid");
        for (name, entry) in ours {
            match merged.get_mut(&name) {
                Some(known) => known.label = entry.label,
                None => {
                    merged.insert(name, entry);
                }
            }
        }
        let status = KnowledgeStatus {
            source: SourceKind::Community,
            packages: merged.len(),
            fetched_at,
        };
        Knowledge {
            entries: merged,
            status,
        }
    }

    pub fn get(&self, package: &str) -> Option<&Entry> {
        self.entries.get(package)
    }

    pub fn status(&self) -> &KnowledgeStatus {
        &self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_list_is_complete_and_consistent() {
        let k = Knowledge::builtin();
        assert!(k.status().packages >= 100);
        for (name, e) in &k.entries {
            assert!(crate::packages::is_package_name(name), "{name}");
            assert!(
                e.label.as_deref().is_some_and(|l| !l.trim().is_empty()),
                "{name} has no label"
            );
            assert!(
                ["google", "oem", "aosp", "carrier", "misc"].contains(&e.list.as_str()),
                "{name}: {}",
                e.list
            );
            assert!(
                e.description.ends_with('.'),
                "{name}: write the description as a sentence"
            );
            assert!(
                !e.description.contains('\u{2014}') && !e.description.contains('\u{2026}'),
                "{name}"
            );
        }
        let msa = k.get("com.miui.msa.global").unwrap();
        assert_eq!(msa.removal, Removal::Recommended);
        assert_eq!(msa.label.as_deref(), Some("MIUI System Ads"));
    }

    fn community(n: usize) -> String {
        let mut items: Vec<String> = (0..n)
            .map(|i| format!(r#""org.example.p{i}": {{"list": "Misc", "description": "Package {i}", "dependencies": [], "neededBy": [], "labels": [], "removal": "Recommended"}}"#))
            .collect();
        items.push(r#""com.miui.msa.global": {"list": "Oem", "description": "Community words.\n", "dependencies": [], "neededBy": ["com.miui.other"], "labels": [], "removal": "Advanced"}"#.into());
        items.push(r#""com.future.level": {"list": "Oem", "description": "x", "removal": "Maybe"}"#.into());
        format!("{{{}}}", items.join(","))
    }

    #[test]
    fn merges_the_community_list_with_ours() {
        let entries = parse_community(&community(1200)).unwrap();
        assert!(!entries.contains_key("com.future.level"));
        let k = Knowledge::with_community(entries, Some(1_700_000_000_000));
        let msa = k.get("com.miui.msa.global").unwrap();
        assert_eq!(msa.description, "Community words.");
        assert_eq!(msa.removal, Removal::Advanced);
        assert_eq!(msa.list, "oem");
        assert_eq!(msa.needed_by, vec!["com.miui.other"]);
        assert_eq!(msa.label.as_deref(), Some("MIUI System Ads"));
        // Ours fills what the community list lacks.
        assert!(k.get("com.miui.mms").is_some());
        assert_eq!(k.status().source, SourceKind::Community);
        assert_eq!(k.status().fetched_at, Some(1_700_000_000_000));
    }

    #[test]
    fn refuses_a_broken_or_partial_download() {
        assert!(matches!(
            parse_community("<html>rate limited</html>"),
            Err(KnowledgeError::Format(_))
        ));
        assert_eq!(parse_community(&community(10)), Err(KnowledgeError::TooSmall(11)));
    }
}
