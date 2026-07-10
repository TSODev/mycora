use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use time::format_description::well_known::Rfc3339;

use crate::tree::Tree;
use crate::vault::Vault;

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
            count += 1;
        }

        tx.commit()?;
        Ok(count)
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
}
