use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::note::NoteId;
use crate::tree::Tree;
use crate::vault::Vault;

/// A note found via the index — by full-text search or by tag filter.
pub struct IndexedNote {
    pub note_id: NoteId,
    pub title: String,
}

/// How to combine multiple tags in `Index::filter_by_tags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagFilterOp {
    /// Note must have every given tag.
    All,
    /// Note must have at least one of the given tags.
    Any,
}

/// A disposable SQLite index over one or more vaults' notes, rebuilt from
/// the Markdown source on demand (`reindex`) rather than treated as a
/// second source of truth. Deliberately not scoped to a single vault
/// directory: every table is keyed by `vault_id` so one index file can hold
/// every *mounted* vault (see ROADMAP.md's "Multiple vaults" entry) once
/// mounting more than one at a time is implemented — for now callers just
/// pass the active vault's registry name as `vault_id`.
pub struct Index {
    conn: Connection,
}

impl Index {
    /// `~/.local/share/mycora/index.sqlite3` — XDG data dir, not
    /// `~/.config`, since this file is generated and disposable rather than
    /// user-authored.
    pub fn default_path(home: &str) -> PathBuf {
        PathBuf::from(home).join(".local/share/mycora/index.sqlite3")
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating index directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening index at {}", path.display()))?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS notes (
                vault_id TEXT NOT NULL,
                id       TEXT NOT NULL,
                title    TEXT NOT NULL,
                path     TEXT NOT NULL,
                tags     TEXT NOT NULL,
                created  TEXT NOT NULL,
                updated  TEXT NOT NULL,
                PRIMARY KEY (vault_id, id)
            );
            CREATE TABLE IF NOT EXISTS tree_edges (
                vault_id  TEXT NOT NULL,
                id        TEXT NOT NULL,
                parent    TEXT,
                order_key INTEGER NOT NULL,
                PRIMARY KEY (vault_id, id)
            );
            CREATE TABLE IF NOT EXISTS links (
                vault_id TEXT NOT NULL,
                source   TEXT NOT NULL,
                target   TEXT NOT NULL,
                PRIMARY KEY (vault_id, source, target)
            );
            CREATE TABLE IF NOT EXISTS tags (
                vault_id TEXT NOT NULL,
                note_id  TEXT NOT NULL,
                tag      TEXT NOT NULL,
                PRIMARY KEY (vault_id, note_id, tag)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                title,
                body,
                tags,
                vault_id UNINDEXED,
                note_id UNINDEXED
            );
            ",
        )
        .context("creating index schema")?;
        Ok(())
    }

    /// Rebuilds every row belonging to `vault_id` from `tree`/`vault`'s
    /// current state: drops then reinserts, since the index is always
    /// disposable and cheaper to regenerate wholesale than to diff.
    pub fn reindex(&mut self, vault_id: &str, tree: &Tree, vault: &Vault) -> Result<usize> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM notes WHERE vault_id = ?1", params![vault_id])?;
        tx.execute(
            "DELETE FROM tree_edges WHERE vault_id = ?1",
            params![vault_id],
        )?;
        tx.execute(
            "DELETE FROM notes_fts WHERE vault_id = ?1",
            params![vault_id],
        )?;
        tx.execute("DELETE FROM tags WHERE vault_id = ?1", params![vault_id])?;

        let mut count = 0;
        for (id, note) in tree.iter() {
            let path = vault
                .path(id)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let tags = note.tags.join(",");
            let created = note.created.format(&Rfc3339)?;
            let updated = note.updated.format(&Rfc3339)?;

            tx.execute(
                "INSERT INTO notes (vault_id, id, title, path, tags, created, updated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    vault_id,
                    id.0.to_string(),
                    note.title,
                    path,
                    tags,
                    created,
                    updated
                ],
            )?;
            tx.execute(
                "INSERT INTO tree_edges (vault_id, id, parent, order_key)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    vault_id,
                    id.0.to_string(),
                    note.parent.map(|p| p.0.to_string()),
                    note.order
                ],
            )?;
            tx.execute(
                "INSERT INTO notes_fts (title, body, tags, vault_id, note_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![note.title, note.body, tags, vault_id, id.0.to_string()],
            )?;
            for tag in &note.tags {
                tx.execute(
                    "INSERT OR IGNORE INTO tags (vault_id, note_id, tag) VALUES (?1, ?2, ?3)",
                    params![vault_id, id.0.to_string(), tag],
                )?;
            }
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Full-text search over title + body (+ tags) within `vault_id`,
    /// best-match first. Baseline substring-ish matching for v0.4 — each
    /// whitespace-separated term becomes an FTS5 prefix match, ANDed
    /// together, rather than exposing raw FTS5 query syntax to the caller.
    /// Relevance ranking upgrades to tantivy/BM25 in v0.6.
    pub fn search(&self, vault_id: &str, query: &str) -> Result<Vec<IndexedNote>> {
        let match_query = Self::build_match_query(query);
        if match_query.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT note_id, title FROM notes_fts
             WHERE notes_fts MATCH ?1 AND vault_id = ?2
             ORDER BY rank
             LIMIT 50",
        )?;
        let rows = stmt.query_map(params![match_query, vault_id], |row| {
            let note_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            Ok((note_id, title))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(IndexedNote {
                note_id: NoteId(uuid),
                title,
            });
        }
        Ok(hits)
    }

    /// Turns free-text user input into an FTS5 MATCH expression: each term
    /// is quoted (so punctuation/FTS5 operators in the input can't be
    /// interpreted as query syntax) and suffixed with `*` for prefix
    /// matching, ANDed together via FTS5's default bareword-adjacency
    /// behavior.
    fn build_match_query(query: &str) -> String {
        query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Baseline set-filtering over the `tags` index: notes in `vault_id`
    /// that have all (`TagFilterOp::All`) or any (`TagFilterOp::Any`) of
    /// `tags`, ordered by title. No relevance ranking — that's v0.6's job,
    /// once tantivy's faceted filters land alongside this.
    pub fn filter_by_tags(
        &self,
        vault_id: &str,
        tags: &[String],
        op: TagFilterOp,
    ) -> Result<Vec<IndexedNote>> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = match op {
            TagFilterOp::Any => format!(
                "SELECT DISTINCT n.id, n.title
                 FROM notes n
                 JOIN tags t ON t.vault_id = n.vault_id AND t.note_id = n.id
                 WHERE n.vault_id = ? AND t.tag IN ({placeholders})
                 ORDER BY n.title"
            ),
            TagFilterOp::All => format!(
                "SELECT n.id, n.title
                 FROM notes n
                 JOIN tags t ON t.vault_id = n.vault_id AND t.note_id = n.id
                 WHERE n.vault_id = ? AND t.tag IN ({placeholders})
                 GROUP BY n.id, n.title
                 HAVING COUNT(DISTINCT t.tag) = ?
                 ORDER BY n.title"
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let mut query_params: Vec<&dyn rusqlite::ToSql> = vec![&vault_id];
        for tag in tags {
            query_params.push(tag);
        }
        let tag_count = tags.len() as i64;
        if op == TagFilterOp::All {
            query_params.push(&tag_count);
        }

        let rows = stmt.query_map(query_params.as_slice(), |row| {
            let note_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            Ok((note_id, title))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(IndexedNote {
                note_id: NoteId(uuid),
                title,
            });
        }
        Ok(hits)
    }

    pub fn note_count(&self, vault_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM notes WHERE vault_id = ?1",
                params![vault_id],
                |row| row.get(0),
            )
            .context("counting indexed notes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn scratch_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("mycora-index-test-{}.sqlite3", Uuid::new_v4()))
    }

    fn temp_vault_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mycora-index-vault-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reindex_populates_notes_and_tree_edges_for_the_given_vault() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        tree.create_note("Child", Some(root));

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();

        let count = index.reindex("default", &tree, &vault).unwrap();
        assert_eq!(count, 2);
        assert_eq!(index.note_count("default").unwrap(), 2);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_is_scoped_to_its_vault_id_and_replaces_prior_rows() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        tree_a.create_note("A", None);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        let mut tree_b = Tree::new();
        tree_b.create_note("B1", None);
        tree_b.create_note("B2", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();
        index.reindex("b", &tree_b, &vault_b).unwrap();

        assert_eq!(index.note_count("a").unwrap(), 1);
        assert_eq!(index.note_count("b").unwrap(), 2);

        // Reindexing "a" again with a now-empty tree should clear its rows
        // without touching "b"'s.
        let empty = Tree::new();
        index.reindex("a", &empty, &vault_a).unwrap();
        assert_eq!(index.note_count("a").unwrap(), 0);
        assert_eq!(index.note_count("b").unwrap(), 2);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn search_finds_notes_by_title_or_body_prefix() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let rust_note = tree.create_note("Rust ownership", None);
        tree.set_body(rust_note, "Notes about borrowing and lifetimes.");
        let other_note = tree.create_note("Grocery list", None);
        tree.set_body(other_note, "Milk, eggs, bread.");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let hits = index.search("default", "borrow").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, rust_note);
        assert_eq!(hits[0].title, "Rust ownership");

        let title_hits = index.search("default", "Rust").unwrap();
        assert_eq!(title_hits.len(), 1);
        assert_eq!(title_hits[0].note_id, rust_note);

        assert!(index.search("default", "xyzzy").unwrap().is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_is_scoped_to_its_vault_id() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        tree_a.create_note("Shared Topic", None);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        assert_eq!(index.search("a", "shared").unwrap().len(), 1);
        assert!(index.search("b", "shared").unwrap().is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
    }

    fn tagged_note(title: &str, tags: &[&str]) -> (NoteId, crate::note::Note) {
        let mut note = crate::note::Note::new(title, None);
        note.tags = tags.iter().map(|t| t.to_string()).collect();
        (NoteId::new(), note)
    }

    #[test]
    fn filter_by_tags_any_matches_notes_with_at_least_one_given_tag() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let (rust_note, note) = tagged_note("Rust ownership", &["rust", "lang"]);
        tree.insert_loaded(rust_note, note);
        let (go_note, note) = tagged_note("Go channels", &["go", "lang"]);
        tree.insert_loaded(go_note, note);
        let (_cooking_note, note) = tagged_note("Bread recipe", &["cooking"]);
        tree.insert_loaded(_cooking_note, note);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let tags = vec!["rust".to_string(), "go".to_string()];
        let mut hits = index
            .filter_by_tags("default", &tags, TagFilterOp::Any)
            .unwrap();
        hits.sort_by(|a, b| a.title.cmp(&b.title));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Go channels");
        assert_eq!(hits[1].title, "Rust ownership");

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn filter_by_tags_all_requires_every_given_tag() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let (rust_note, note) = tagged_note("Rust ownership", &["rust", "lang"]);
        tree.insert_loaded(rust_note, note);
        let (go_note, note) = tagged_note("Go channels", &["go", "lang"]);
        tree.insert_loaded(go_note, note);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let tags = vec!["rust".to_string(), "lang".to_string()];
        let hits = index
            .filter_by_tags("default", &tags, TagFilterOp::All)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, rust_note);

        let no_match = vec!["rust".to_string(), "go".to_string()];
        assert!(index
            .filter_by_tags("default", &no_match, TagFilterOp::All)
            .unwrap()
            .is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn filter_by_tags_is_scoped_to_its_vault_id() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let (note_a, note) = tagged_note("A", &["shared"]);
        tree_a.insert_loaded(note_a, note);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        let tags = vec!["shared".to_string()];
        assert_eq!(
            index
                .filter_by_tags("a", &tags, TagFilterOp::Any)
                .unwrap()
                .len(),
            1
        );
        assert!(index
            .filter_by_tags("b", &tags, TagFilterOp::Any)
            .unwrap()
            .is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
    }

    #[test]
    fn filter_by_tags_with_no_tags_returns_nothing() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        assert!(index
            .filter_by_tags("default", &[], TagFilterOp::Any)
            .unwrap()
            .is_empty());
        std::fs::remove_file(&db_path).ok();
    }
}
