use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::note::NoteId;

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawSession {
    #[serde(default)]
    vaults: HashMap<String, VaultSession>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VaultSession {
    selected: Option<Uuid>,
    #[serde(default)]
    expanded: Vec<Uuid>,
}

/// The last-known selection and expand/collapse state per mounted vault —
/// "remember last open note, expanded/collapsed branches" across restarts.
/// Keyed by vault name (not just the active one) so switching which vault
/// is `default` in the config doesn't clobber another vault's remembered
/// position, even though only one vault is ever navigable at a time today.
///
/// Read once at startup (`App::new`) and written once at shutdown — not
/// write-through on every expand/collapse or selection change, unlike note
/// edits. This is ephemeral navigation state, not user content; writing it
/// to disk on every keystroke would be wasted I/O for no benefit over
/// saving once when the app is closing.
pub struct Session {
    path: PathBuf,
    raw: RawSession,
}

impl Session {
    /// `~/.local/share/mycora/session.toml` — XDG data dir alongside the
    /// SQLite index, since this is generated state, not user-authored
    /// config.
    pub fn default_path(home: &str) -> PathBuf {
        PathBuf::from(home).join(".local/share/mycora/session.toml")
    }

    /// Missing or unparseable files are treated the same as an empty
    /// session (self-heal rather than fail to start) — losing remembered
    /// position is a papercut, not data loss.
    pub fn load(path: &Path) -> Self {
        let raw = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            raw,
        }
    }

    /// The previously saved `(selected, expanded)` for `vault_name`, if
    /// any was ever saved for it.
    pub fn for_vault(&self, vault_name: &str) -> Option<(Option<NoteId>, HashSet<NoteId>)> {
        let saved = self.raw.vaults.get(vault_name)?;
        let selected = saved.selected.map(NoteId);
        let expanded = saved.expanded.iter().copied().map(NoteId).collect();
        Some((selected, expanded))
    }

    /// Records `vault_name`'s current selection/expand state and writes
    /// the whole session file — other vaults' entries are preserved
    /// unchanged (this re-reads the file first rather than assuming
    /// in-memory state from `load` is still current, since another
    /// mycora process could have written its own vault's entry meanwhile).
    pub fn save(
        &mut self,
        vault_name: &str,
        selected: Option<NoteId>,
        expanded: &HashSet<NoteId>,
    ) -> anyhow::Result<()> {
        let mut fresh = Self::load(&self.path);
        fresh.raw.vaults.insert(
            vault_name.to_string(),
            VaultSession {
                selected: selected.map(|id| id.0),
                expanded: expanded.iter().map(|id| id.0).collect(),
            },
        );
        self.raw = fresh.raw;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(&self.raw)?;
        std::fs::write(&self.path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_path() -> PathBuf {
        std::env::temp_dir().join(format!("mycora-session-test-{}.toml", Uuid::new_v4()))
    }

    #[test]
    fn missing_file_loads_as_an_empty_session() {
        let path = scratch_path();
        let session = Session::load(&path);
        assert!(session.for_vault("default").is_none());
    }

    #[test]
    fn save_then_load_round_trips_selection_and_expanded_set() {
        let path = scratch_path();
        let mut session = Session::load(&path);

        let selected = NoteId::new();
        let a = NoteId::new();
        let b = NoteId::new();
        let expanded: HashSet<NoteId> = [a, b].into_iter().collect();
        session.save("default", Some(selected), &expanded).unwrap();

        let reloaded = Session::load(&path);
        let (saved_selected, saved_expanded) = reloaded.for_vault("default").unwrap();
        assert_eq!(saved_selected, Some(selected));
        assert_eq!(saved_expanded, expanded);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn saving_one_vault_does_not_clobber_another_vaults_entry() {
        let path = scratch_path();
        let mut session = Session::load(&path);

        let a_selected = NoteId::new();
        session
            .save("a", Some(a_selected), &HashSet::new())
            .unwrap();

        let b_selected = NoteId::new();
        session
            .save("b", Some(b_selected), &HashSet::new())
            .unwrap();

        let reloaded = Session::load(&path);
        assert_eq!(reloaded.for_vault("a").unwrap().0, Some(a_selected));
        assert_eq!(reloaded.for_vault("b").unwrap().0, Some(b_selected));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unparseable_file_is_treated_as_an_empty_session() {
        let path = scratch_path();
        std::fs::write(&path, "not valid toml {{{").unwrap();
        let session = Session::load(&path);
        assert!(session.for_vault("default").is_none());
        std::fs::remove_file(&path).ok();
    }
}
