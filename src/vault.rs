use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::note::{Note, NoteId};
use crate::tree::Tree;

/// Warnings collected while loading a vault: malformed files, duplicate ids,
/// or notes whose parent couldn't be resolved. Nothing on disk is lost when
/// these happen — problem files are skipped, or self-healed and rewritten.
pub struct LoadReport {
    pub warnings: Vec<String>,
}

/// Loads/writes the Markdown vault directory. Markdown files are the only
/// source of truth; this type is the sole owner of the on-disk note <-> path
/// mapping needed to know which file to rewrite or delete for a given note.
pub struct Vault {
    root: PathBuf,
    paths: HashMap<NoteId, PathBuf>,
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)
            .with_context(|| format!("creating vault directory at {}", root.display()))?;
        Ok(Self {
            root,
            paths: HashMap::new(),
        })
    }

    pub fn load(&mut self) -> Result<(Tree, LoadReport)> {
        let mut tree = Tree::new();
        let mut warnings = Vec::new();
        let mut seen_ids = HashSet::new();

        let entries = fs::read_dir(&self.root)
            .with_context(|| format!("reading vault directory {}", self.root.display()))?;

        for entry in entries {
            let entry = entry.context("reading vault directory entry")?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }

            match load_note_file(&path) {
                Ok((mut id, note)) => {
                    if !seen_ids.insert(id) {
                        let new_id = NoteId::new();
                        warnings.push(format!(
                            "{}: duplicate id, reassigned to a new one",
                            path.display()
                        ));
                        if let Err(err) = write_note_file(&path, new_id, &note) {
                            warnings.push(format!(
                                "{}: failed to rewrite with new id: {err}",
                                path.display()
                            ));
                        }
                        id = new_id;
                    }
                    self.paths.insert(id, path);
                    tree.insert_loaded(id, note);
                }
                Err(err) => {
                    warnings.push(format!("{}: skipped, {err}", path.display()));
                }
            }
        }

        let orphaned = tree.rebuild_hierarchy();
        for id in orphaned {
            let where_ = self
                .paths
                .get(&id)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            warnings.push(format!("{where_}: parent not found, promoted to root"));
            if let Some(note) = tree.get(id)
                && let Some(path) = self.paths.get(&id)
                && let Err(err) = write_note_file(path, id, note)
            {
                warnings.push(format!("{where_}: failed to repair parent: {err}"));
            }
        }

        Ok((tree, LoadReport { warnings }))
    }

    /// The on-disk file backing `id`, if it's been loaded or saved this
    /// session — for callers (the SQLite indexer) that need the path rather
    /// than just the note's structural/content fields.
    pub fn path(&self, id: NoteId) -> Option<&Path> {
        self.paths.get(&id).map(PathBuf::as_path)
    }

    pub fn save_note(&mut self, id: NoteId, note: &Note) -> Result<()> {
        let path = match self.paths.get(&id) {
            Some(path) => path.clone(),
            None => {
                let path = self.allocate_path(&note.title);
                self.paths.insert(id, path.clone());
                path
            }
        };
        write_note_file(&path, id, note)
    }

    /// Moves a note's file into `<vault>/.trash/` rather than deleting it
    /// outright — the safety net behind confirmed deletes. Trash is never
    /// auto-emptied or scanned by `load`; restoring a note (e.g. via undo)
    /// writes a fresh file at the vault root instead of moving this one
    /// back, so entries here accumulate as a simple, inspectable history.
    pub fn trash_note(&mut self, id: NoteId) -> Result<()> {
        let Some(path) = self.paths.remove(&id) else {
            return Ok(());
        };
        let trash_dir = self.root.join(".trash");
        fs::create_dir_all(&trash_dir)
            .with_context(|| format!("creating trash directory at {}", trash_dir.display()))?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("note");
        let target = unique_path(&trash_dir, stem, "md");

        fs::rename(&path, &target)
            .with_context(|| format!("moving {} to trash", path.display()))?;
        Ok(())
    }

    fn allocate_path(&self, title: &str) -> PathBuf {
        unique_path(&self.root, &slugify(title), "md")
    }
}

fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base}-{n}.{ext}"));
        n += 1;
    }
    candidate
}

fn load_note_file(path: &Path) -> Result<(NoteId, Note)> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let (frontmatter, body) =
        split_frontmatter(&raw).with_context(|| "missing or malformed frontmatter")?;
    let fm: Frontmatter =
        serde_yaml::from_str(frontmatter).with_context(|| "invalid frontmatter YAML")?;
    let (title, body) = split_title(body);

    let note = Note {
        title,
        body,
        tags: fm.tags,
        parent: fm.parent.map(NoteId),
        children: Vec::new(),
        order: fm.order,
        created: fm.created,
        updated: fm.updated,
    };

    Ok((NoteId(fm.id), note))
}

fn write_note_file(path: &Path, id: NoteId, note: &Note) -> Result<()> {
    let fm = Frontmatter {
        id: id.0,
        parent: note.parent.map(|p| p.0),
        order: note.order,
        tags: note.tags.clone(),
        created: note.created,
        updated: note.updated,
    };
    let yaml = serde_yaml::to_string(&fm).context("serializing frontmatter")?;
    let contents = format!("---\n{yaml}---\n\n# {}\n\n{}\n", note.title, note.body.trim());

    let tmp_path = path.with_extension("md.tmp");
    fs::write(&tmp_path, contents).with_context(|| format!("writing {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("finalizing write to {}", path.display()))?;
    Ok(())
}

/// Splits a file's raw contents into its `---`-delimited YAML frontmatter and
/// the remaining body.
fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow::anyhow!("file must start with a '---' frontmatter block"))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| anyhow::anyhow!("frontmatter block has no closing '---'"))?;
    let frontmatter = &rest[..end];
    let body = rest[end + 5..].trim_start_matches('\n');
    Ok((frontmatter, body))
}

/// Splits a note body into its title (the first `# Heading`) and the rest.
/// Falls back to "Untitled" if the body doesn't start with one, keeping the
/// original content intact rather than losing it.
fn split_title(body: &str) -> (String, String) {
    let trimmed = body.trim_start();
    if let Some(rest) = trimmed.strip_prefix("# ") {
        let (title_line, remainder) = rest.split_once('\n').unwrap_or((rest, ""));
        return (
            title_line.trim().to_string(),
            remainder.trim_start_matches('\n').trim_end().to_string(),
        );
    }
    ("Untitled".to_string(), body.trim_end().to_string())
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Frontmatter {
    id: Uuid,
    #[serde(default)]
    parent: Option<Uuid>,
    #[serde(default)]
    order: i64,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mycora-vault-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slugify_handles_spaces_and_punctuation() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("Café ☕"), "caf");
    }

    #[test]
    fn split_frontmatter_rejects_missing_delimiters() {
        assert!(split_frontmatter("no frontmatter here").is_err());
    }

    #[test]
    fn split_title_extracts_leading_heading() {
        let (title, body) = split_title("# My Title\n\nSome body text.\n");
        assert_eq!(title, "My Title");
        assert_eq!(body, "Some body text.");
    }

    #[test]
    fn split_title_falls_back_to_untitled() {
        let (title, body) = split_title("Just a paragraph, no heading.");
        assert_eq!(title, "Untitled");
        assert_eq!(body, "Just a paragraph, no heading.");
    }

    #[test]
    fn save_then_load_round_trips_a_note() {
        let dir = temp_vault_dir();
        let mut vault = Vault::open(dir.clone()).unwrap();

        let mut note = Note::new("Round Trip", None);
        note.tags = vec!["alpha".to_string(), "beta".to_string()];
        note.body = "Some body content.".to_string();
        let id = NoteId::new();
        vault.save_note(id, &note).unwrap();

        let mut reloaded = Vault::open(dir.clone()).unwrap();
        let (tree, report) = reloaded.load().unwrap();

        assert!(report.warnings.is_empty());
        assert_eq!(tree.roots(), &[id]);
        let loaded_note = tree.get(id).unwrap();
        assert_eq!(loaded_note.title, "Round Trip");
        assert_eq!(loaded_note.body, "Some body content.");
        assert_eq!(loaded_note.tags, vec!["alpha", "beta"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reports_orphaned_parent_and_self_heals() {
        let dir = temp_vault_dir();
        let missing_parent = NoteId::new();
        let mut note = Note::new("Orphan", Some(missing_parent));
        note.order = 0;
        let id = NoteId::new();

        {
            let mut vault = Vault::open(dir.clone()).unwrap();
            vault.save_note(id, &note).unwrap();
        }

        let mut vault = Vault::open(dir.clone()).unwrap();
        let (tree, report) = vault.load().unwrap();

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(tree.roots(), &[id]);
        assert_eq!(tree.get(id).unwrap().parent, None);

        // Reloading again must not re-report the same note as orphaned: the
        // first load should have rewritten the file with `parent: null`.
        let mut vault_again = Vault::open(dir.clone()).unwrap();
        let (_, report_again) = vault_again.load().unwrap();
        assert!(report_again.warnings.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_self_heals_a_note_whose_parent_is_itself() {
        // Not producible by any in-app operation, but a hand-edited
        // frontmatter `parent:` field naming its own note's id is a real
        // possibility for a file on disk — same self-healing contract as
        // a missing parent, so it must not vanish from the tree silently.
        let dir = temp_vault_dir();
        let id = NoteId::new();
        let mut note = Note::new("Self-parented", Some(id));
        note.order = 0;

        {
            let mut vault = Vault::open(dir.clone()).unwrap();
            vault.save_note(id, &note).unwrap();
        }

        let mut vault = Vault::open(dir.clone()).unwrap();
        let (tree, report) = vault.load().unwrap();

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(tree.roots(), &[id]);
        assert_eq!(tree.get(id).unwrap().parent, None);
        assert!(tree.children(id).is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reassigns_duplicate_ids() {
        let dir = temp_vault_dir();
        let shared_id = NoteId::new();

        let mut first = Note::new("First", None);
        first.order = 0;
        let mut second = Note::new("Second", None);
        second.order = 1;

        write_note_file(&dir.join("first.md"), shared_id, &first).unwrap();
        write_note_file(&dir.join("second.md"), shared_id, &second).unwrap();

        let mut vault = Vault::open(dir.clone()).unwrap();
        let (tree, report) = vault.load().unwrap();

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(tree.roots().len(), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn trash_note_moves_file_out_of_vault_root() {
        let dir = temp_vault_dir();
        let mut vault = Vault::open(dir.clone()).unwrap();
        let note = Note::new("To Trash", None);
        let id = NoteId::new();
        vault.save_note(id, &note).unwrap();

        vault.trash_note(id).unwrap();

        assert!(!dir.join("to-trash.md").exists());
        assert!(dir.join(".trash/to-trash.md").exists());

        let mut reloaded = Vault::open(dir.clone()).unwrap();
        let (tree, report) = reloaded.load().unwrap();
        assert!(report.warnings.is_empty());
        assert!(tree.roots().is_empty());

        fs::remove_dir_all(&dir).ok();
    }
}
