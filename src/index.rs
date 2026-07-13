use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::link::extract_wikilink_titles;
use crate::note::NoteId;
use crate::tree::Tree;
use crate::vault::Vault;

/// A note found via the index — by tag filter or backlinks lookup, neither
/// of which has anything snippet-worthy to show (see `SearchHit` for the
/// full-text-search equivalent, which does).
pub struct IndexedNote {
    pub note_id: NoteId,
    pub title: String,
    /// Which vault this note actually lives in — `filter_by_tags` can
    /// span every mounted vault at once (see its own doc comment), and
    /// `backlinks`' sources can live in a different vault than the
    /// target they're pointing at, so a caller can't assume "the vault I
    /// asked about" the way it safely could when both were always
    /// single-vault-scoped.
    pub vault_id: String,
}

/// One full-text search hit: a resolved note plus an FTS5-generated
/// snippet — body text around the match, with every matched term wrapped
/// in `\u{1}`...`\u{2}` sentinels (never shown to the user directly; a
/// renderer splits on them to style the match distinctly, the way
/// `ui.rs`'s `spans_from_snippet` does).
pub struct SearchHit {
    pub note_id: NoteId,
    pub title: String,
    pub snippet: String,
}

/// How to combine multiple tags in `Index::filter_by_tags`/`SearchFacets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagFilterOp {
    /// Note must have every given tag.
    All,
    /// Note must have at least one of the given tags.
    Any,
}

/// Optional facets narrowing `Index::search_faceted` beyond its free-text
/// query — ANDed together with the text match and with each other: a
/// result must satisfy the query and every facet that's `Some`.
#[derive(Default)]
pub struct SearchFacets<'a> {
    /// Tag membership, reusing `filter_by_tags`'s AND/OR semantics.
    pub tags: Option<(&'a [String], TagFilterOp)>,
    /// Inclusive range on `updated`.
    pub date_range: Option<(OffsetDateTime, OffsetDateTime)>,
    /// Restrict to these note ids — typically `Tree::subtree_ids(branch_root)`,
    /// to search "within this branch" rather than the whole vault.
    pub branch: Option<&'a [NoteId]>,
}

/// A `[[title]]` reference found in `source`'s body that didn't resolve to
/// any note during the last `reindex` — a broken link. Reported, not
/// treated as an error: a note referencing a not-yet-written or
/// since-renamed/deleted title is expected, not exceptional.
pub struct BrokenLink {
    pub source: NoteId,
    pub title: String,
}

/// The result of a `reindex` call.
pub struct ReindexReport {
    pub note_count: usize,
    pub broken_links: Vec<BrokenLink>,
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

    /// How long a write waits on a `SQLITE_BUSY` lock (another process
    /// mid-transaction) before giving up — SQLite's own default is `0`,
    /// meaning an immediate error rather than any wait at all. Generous
    /// but still finite: this index is disposable (see the type's own
    /// doc comment), so a caller that genuinely deadlocked is better off
    /// erroring out and letting the user retry than hanging indefinitely.
    const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating index directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening index at {}", path.display()))?;
        // Neither of these makes concurrent writers safe from each other
        // — two processes can still each think they "won" a write, same
        // as the vault's own Markdown files (see ROADMAP.md's
        // "Concurrent-write safety" entry) — but both are strict
        // improvements over the previous defaults at effectively zero
        // cost: WAL lets readers proceed during a writer's transaction
        // instead of blocking on it, and a real timeout means a second
        // process racing a reindex waits and retries instead of failing
        // instantly with "database is locked".
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("enabling WAL journal mode")?;
        conn.busy_timeout(Self::BUSY_TIMEOUT)
            .context("setting busy timeout")?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<()> {
        // `links`' shape changed (a single `vault_id` -> `source_vault` +
        // `target_vault`, to represent a link whose two ends live in
        // different mounted vaults). The whole index is disposable and
        // safe to rebuild from scratch, so an old-shape table left over
        // from before this change is just dropped rather than migrated —
        // its rows regenerate for free on the next reindex.
        let old_shape: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('links') WHERE name = 'vault_id'")?
            .exists([])?;
        if old_shape {
            conn.execute("DROP TABLE links", [])?;
        }

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
                source_vault TEXT NOT NULL,
                source       TEXT NOT NULL,
                target_vault TEXT NOT NULL,
                target       TEXT NOT NULL,
                PRIMARY KEY (source_vault, source, target_vault, target)
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
            CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title);
            ",
        )
        .context("creating index schema")?;
        Ok(())
    }

    /// Rebuilds `vault_id`'s rows alone, resolving its wikilinks only
    /// against its own notes — equivalent to `reindex_mounted(&[(vault_id,
    /// tree, vault)])`. Fine for a single-vault setup, or for refreshing
    /// just the one vault that changed (search/backlinks do this): the
    /// `notes` table still has every other mounted vault's rows from a
    /// previous `reindex_mounted` call, but this call's link resolution
    /// won't reach them, since it only trusts `vault_id` as "known good".
    pub fn reindex(&mut self, vault_id: &str, tree: &Tree, vault: &Vault) -> Result<ReindexReport> {
        let mut reports = self.reindex_mounted(&[(vault_id, tree, vault)])?;
        Ok(reports.remove(0))
    }

    /// Rebuilds every given vault's rows together, so a `[[title]]` in one
    /// can resolve to a note in *any* of them — this is what cross-vault
    /// linking means (see ROADMAP.md's v0.5 "Cross-vault links" entry).
    /// Two phases, because link resolution needs every vault's notes
    /// already written before any of them can be looked up: first every
    /// vault's `notes`/`tree_edges`/`notes_fts`/`tags` rows, then every
    /// vault's `links` rows, resolved against the now-complete set.
    /// Deliberately scoped to just the vaults passed in, not "every vault
    /// ever indexed" — a vault that was mounted in a past session but
    /// isn't part of this call doesn't get to resolve as a link target,
    /// so its stale rows (still on disk until something reindexes over
    /// them) can't silently leak into a fresh session's link results.
    pub fn reindex_mounted(
        &mut self,
        vaults: &[(&str, &Tree, &Vault)],
    ) -> Result<Vec<ReindexReport>> {
        let mut note_counts = Vec::with_capacity(vaults.len());
        for (vault_id, tree, vault) in vaults {
            note_counts.push(self.write_notes(vault_id, tree, vault)?);
        }

        let vault_ids: Vec<&str> = vaults.iter().map(|(id, _, _)| *id).collect();
        let mut reports = Vec::with_capacity(vaults.len());
        for (i, (vault_id, tree, _)) in vaults.iter().enumerate() {
            let broken_links = self.write_links(vault_id, tree, &vault_ids)?;
            reports.push(ReindexReport {
                note_count: note_counts[i],
                broken_links,
            });
        }
        Ok(reports)
    }

    /// Phase 1 of a reindex: `notes`/`tree_edges`/`notes_fts`/`tags` for
    /// `vault_id` alone, from `tree`/`vault`'s current state. Drops then
    /// reinserts, since the index is always disposable and cheaper to
    /// regenerate wholesale than to diff. Does not touch `links` — that's
    /// `write_links`, run separately once every vault in a batch has had
    /// this phase done.
    fn write_notes(&mut self, vault_id: &str, tree: &Tree, vault: &Vault) -> Result<usize> {
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

    /// Phase 2 of a reindex: resolves `vault_id`'s wikilinks and (re)writes
    /// its `links` rows — only its *outgoing* ones (`source_vault =
    /// vault_id`), never touching another vault's rows even if this call
    /// creates a link pointing into it. Resolution is a `notes` lookup
    /// scoped to `known_vault_ids`, not every vault ever indexed (see
    /// `reindex_mounted`'s doc comment on why that scoping matters). Titles
    /// aren't required to be unique, so a match in more than one note (in
    /// the same or a different vault) fans out to a link per match, rather
    /// than silently picking one (see ROADMAP.md's v0.5 entry).
    fn write_links(
        &mut self,
        vault_id: &str,
        tree: &Tree,
        known_vault_ids: &[&str],
    ) -> Result<Vec<BrokenLink>> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM links WHERE source_vault = ?1",
            params![vault_id],
        )?;

        let placeholders = known_vault_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let lookup_sql =
            format!("SELECT vault_id, id FROM notes WHERE title = ?1 AND vault_id IN ({placeholders})");

        let mut broken_links = Vec::new();
        for (id, note) in tree.iter() {
            for title in extract_wikilink_titles(&note.body) {
                let targets: Vec<(String, String)> = {
                    // `lookup_sql`'s text is identical on every iteration
                    // (it only depends on `known_vault_ids`, fixed for the
                    // whole call) — `prepare_cached` compiles it once and
                    // reuses that plan for every wikilink instead of
                    // recompiling per lookup.
                    let mut stmt = tx.prepare_cached(&lookup_sql)?;
                    let mut lookup_params: Vec<&dyn rusqlite::ToSql> = vec![&title];
                    for vid in known_vault_ids {
                        lookup_params.push(vid);
                    }
                    stmt.query_map(lookup_params.as_slice(), |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })?
                    .collect::<rusqlite::Result<_>>()?
                };

                if targets.is_empty() {
                    broken_links.push(BrokenLink { source: id, title });
                    continue;
                }
                for (target_vault, target_id) in targets {
                    if target_vault == vault_id && target_id == id.0.to_string() {
                        continue; // skip self-links
                    }
                    tx.execute(
                        "INSERT OR IGNORE INTO links (source_vault, source, target_vault, target)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![vault_id, id.0.to_string(), target_vault, target_id],
                    )?;
                }
            }
        }

        tx.commit()?;
        Ok(broken_links)
    }

    /// Full-text search over title + body (+ tags) within `vault_id`,
    /// best-match first. Baseline substring-ish matching for v0.4 — each
    /// whitespace-separated term becomes an FTS5 prefix match, ANDed
    /// together, rather than exposing raw FTS5 query syntax to the caller.
    /// Relevance ranking upgrades to tantivy/BM25 in v0.6.
    pub fn search(&self, vault_id: &str, query: &str) -> Result<Vec<SearchHit>> {
        self.search_faceted(vault_id, query, &SearchFacets::default())
    }

    /// Full-text search like `search`, plus optional facets (tag, date
    /// range, tree branch — see `SearchFacets`) ANDed onto the match.
    /// `search(vault_id, query)` is exactly `search_faceted(vault_id,
    /// query, &SearchFacets::default())`.
    pub fn search_faceted(
        &self,
        vault_id: &str,
        query: &str,
        facets: &SearchFacets,
    ) -> Result<Vec<SearchHit>> {
        let match_query = Self::build_match_query(query);
        if match_query.is_empty() {
            return Ok(Vec::new());
        }
        // An empty branch or empty tag list can never match anything —
        // short-circuit rather than let the SQL below degrade into "no
        // restriction at all" (an empty `IN ()` list) or a stray error.
        if matches!(facets.branch, Some(ids) if ids.is_empty())
            || matches!(&facets.tags, Some((tags, _)) if tags.is_empty())
        {
            return Ok(Vec::new());
        }

        let branch_ids: Vec<String> = facets
            .branch
            .map(|ids| ids.iter().map(|id| id.0.to_string()).collect())
            .unwrap_or_default();
        let date_strings: Option<(String, String)> = facets
            .date_range
            .map(|(start, end)| -> Result<(String, String)> {
                Ok((start.format(&Rfc3339)?, end.format(&Rfc3339)?))
            })
            .transpose()?;
        let tag_count: Option<i64> = facets
            .tags
            .as_ref()
            .and_then(|(tags, op)| (*op == TagFilterOp::All).then_some(tags.len() as i64));

        // Column 1 is `body` (see the CREATE VIRTUAL TABLE order: title=0,
        // body=1, tags=2). `\u{1}`/`\u{2}` are ASCII control characters
        // used purely as sentinels around each matched term — never
        // rendered directly, a UI splits on them to style the match (see
        // `SearchHit`'s doc comment). `notes_fts` stays unaliased (rather
        // than e.g. `AS nf`) because FTS5's whole-row `MATCH` needs to
        // name the table directly.
        let mut sql = String::from(
            "SELECT notes_fts.note_id, notes_fts.title,
                    snippet(notes_fts, 1, '\u{1}', '\u{2}', '…', 16)
             FROM notes_fts
             JOIN notes AS n ON n.vault_id = notes_fts.vault_id AND n.id = notes_fts.note_id
             WHERE notes_fts MATCH ? AND notes_fts.vault_id = ?",
        );
        let mut query_params: Vec<&dyn rusqlite::ToSql> = vec![&match_query, &vault_id];

        if !branch_ids.is_empty() {
            let placeholders = branch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            sql.push_str(&format!(" AND notes_fts.note_id IN ({placeholders})"));
            for id in &branch_ids {
                query_params.push(id);
            }
        }

        if let Some((start, end)) = &date_strings {
            sql.push_str(" AND n.updated BETWEEN ? AND ?");
            query_params.push(start);
            query_params.push(end);
        }

        if let Some((tags, op)) = &facets.tags {
            let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            sql.push_str(&format!(
                " AND n.id IN (SELECT note_id FROM tags WHERE vault_id = ? AND tag IN ({placeholders})"
            ));
            query_params.push(&vault_id);
            for tag in *tags {
                query_params.push(tag);
            }
            if *op == TagFilterOp::All {
                sql.push_str(" GROUP BY note_id HAVING COUNT(DISTINCT tag) = ?)");
                query_params.push(tag_count.as_ref().unwrap());
            } else {
                sql.push(')');
            }
        }

        sql.push_str(" ORDER BY rank LIMIT 50");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(query_params.as_slice(), |row| {
            let note_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let snippet: String = row.get(2)?;
            Ok((note_id, title, snippet))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title, snippet) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(SearchHit {
                note_id: NoteId(uuid),
                title,
                snippet,
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

    /// Baseline set-filtering over the `tags` index: notes in any of
    /// `vault_ids` that have all (`TagFilterOp::All`) or any
    /// (`TagFilterOp::Any`) of `tags`, ordered by title. Deliberately
    /// spans every mounted vault at once rather than being scoped to one
    /// — unlike full-text search (`search`/`search_faceted`, scoped to
    /// wherever the current selection is), a tag is a deliberate,
    /// low-noise signal a user applies the same way across every vault
    /// they keep, so "everything tagged X, anywhere" is more useful here
    /// than "only in the one vault I happen to be looking at." No
    /// relevance ranking — that's v0.6's job, once tantivy's faceted
    /// filters land alongside this.
    pub fn filter_by_tags(
        &self,
        vault_ids: &[&str],
        tags: &[String],
        op: TagFilterOp,
    ) -> Result<Vec<IndexedNote>> {
        if tags.is_empty() || vault_ids.is_empty() {
            return Ok(Vec::new());
        }

        let vault_placeholders = vault_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let tag_placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = match op {
            TagFilterOp::Any => format!(
                "SELECT DISTINCT n.id, n.title, n.vault_id
                 FROM notes n
                 JOIN tags t ON t.vault_id = n.vault_id AND t.note_id = n.id
                 WHERE n.vault_id IN ({vault_placeholders}) AND t.tag IN ({tag_placeholders})
                 ORDER BY n.title"
            ),
            TagFilterOp::All => format!(
                "SELECT n.id, n.title, n.vault_id
                 FROM notes n
                 JOIN tags t ON t.vault_id = n.vault_id AND t.note_id = n.id
                 WHERE n.vault_id IN ({vault_placeholders}) AND t.tag IN ({tag_placeholders})
                 GROUP BY n.id, n.title, n.vault_id
                 HAVING COUNT(DISTINCT t.tag) = ?
                 ORDER BY n.title"
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let mut query_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for vault_id in vault_ids {
            query_params.push(vault_id);
        }
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
            let vault_id: String = row.get(2)?;
            Ok((note_id, title, vault_id))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title, vault_id) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(IndexedNote {
                note_id: NoteId(uuid),
                title,
                vault_id,
            });
        }
        Ok(hits)
    }

    /// Every distinct tag used across any of `vault_ids`, alphabetical,
    /// each with how many notes carry it *in total* across all of them
    /// (not broken down per vault — same "tags are transversal" instinct
    /// as `filter_by_tags`) — backs the `:tags list` command.
    pub fn all_tags(&self, vault_ids: &[&str]) -> Result<Vec<(String, i64)>> {
        if vault_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vault_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT tag, COUNT(*) FROM tags WHERE vault_id IN ({placeholders}) \
             GROUP BY tag ORDER BY tag"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(vault_ids), |row| {
            let tag: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((tag, count))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Notes (in any mounted vault) that link to `target` in `vault_id` —
    /// i.e. `target`'s backlinks, ordered by title. Reads whatever `links`
    /// rows `reindex`/`reindex_mounted` last resolved; does not itself
    /// trigger a reindex.
    pub fn backlinks(&self, vault_id: &str, target: NoteId) -> Result<Vec<IndexedNote>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, n.vault_id
             FROM links l
             JOIN notes n ON n.vault_id = l.source_vault AND n.id = l.source
             WHERE l.target_vault = ?1 AND l.target = ?2
             ORDER BY n.title",
        )?;
        let rows = stmt.query_map(params![vault_id, target.0.to_string()], |row| {
            let note_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let vault_id: String = row.get(2)?;
            Ok((note_id, title, vault_id))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title, vault_id) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(IndexedNote {
                note_id: NoteId(uuid),
                title,
                vault_id,
            });
        }
        Ok(hits)
    }

    /// Notes (in any mounted vault) that `source` in `vault_id` links to —
    /// `backlinks`' mirror image, for following a `[[wikilink]]` forward
    /// instead of seeing who points back. Ordered by title; a title that
    /// fans out to more than one note (see `write_links`'s doc comment)
    /// lists each one separately, same as `backlinks` would for the
    /// reverse direction. Broken links (title matches no note) and
    /// self-links are already absent from `links` by construction — see
    /// `write_links` — so neither needs filtering out here.
    pub fn outgoing_links(&self, vault_id: &str, source: NoteId) -> Result<Vec<IndexedNote>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, n.vault_id
             FROM links l
             JOIN notes n ON n.vault_id = l.target_vault AND n.id = l.target
             WHERE l.source_vault = ?1 AND l.source = ?2
             ORDER BY n.title",
        )?;
        let rows = stmt.query_map(params![vault_id, source.0.to_string()], |row| {
            let note_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let vault_id: String = row.get(2)?;
            Ok((note_id, title, vault_id))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (note_id, title, vault_id) = row?;
            let uuid = Uuid::parse_str(&note_id)
                .with_context(|| format!("indexed note id {note_id} is not a valid UUID"))?;
            hits.push(IndexedNote {
                note_id: NoteId(uuid),
                title,
                vault_id,
            });
        }
        Ok(hits)
    }

    /// Total distinct `links` rows touching any note in `subtree` (source
    /// or target, counted once even for a link between two notes both
    /// inside the subtree) — the aggregate badge for a collapsed tree
    /// branch, e.g. "▸ Research (12 links)". Computed fresh from indexed
    /// lookups on every call rather than cached, per ROADMAP.md's v0.5
    /// entry: expected to stay well under the search-latency budget even
    /// at thousands of notes.
    pub fn link_count_for_subtree(&self, vault_id: &str, subtree: &[NoteId]) -> Result<i64> {
        if subtree.is_empty() {
            return Ok(0);
        }
        let ids: Vec<String> = subtree.iter().map(|id| id.0.to_string()).collect();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT COUNT(*) FROM links
             WHERE (source_vault = ? AND source IN ({placeholders}))
                OR (target_vault = ? AND target IN ({placeholders}))"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let mut query_params: Vec<&dyn rusqlite::ToSql> = vec![&vault_id];
        for id in &ids {
            query_params.push(id);
        }
        query_params.push(&vault_id);
        for id in &ids {
            query_params.push(id);
        }
        stmt.query_row(query_params.as_slice(), |row| row.get(0))
            .context("counting links for subtree")
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
    fn open_enables_wal_mode_and_a_nonzero_busy_timeout() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();

        let journal_mode: String = index
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let busy_timeout_ms: i64 = index
            .conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout_ms, Index::BUSY_TIMEOUT.as_millis() as i64);

        std::fs::remove_file(&db_path).ok();
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

        let report = index.reindex("default", &tree, &vault).unwrap();
        assert_eq!(report.note_count, 2);
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
    fn search_wraps_the_matched_term_in_the_snippet_with_sentinels() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let note = tree.create_note("Rust ownership", None);
        tree.set_body(note, "Notes about borrowing and lifetimes in Rust.");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let hits = index.search("default", "borrow").unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains('\u{1}'));
        assert!(hits[0].snippet.contains('\u{2}'));
        // The matched term itself (case as stored) appears between the
        // sentinels, not just somewhere in the snippet.
        let start = hits[0].snippet.find('\u{1}').unwrap();
        let end = hits[0].snippet.find('\u{2}').unwrap();
        assert!(end > start);
        assert_eq!(&hits[0].snippet[start + 1..end], "borrowing");

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_restricts_results_to_a_given_branch() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let branch_root = tree.create_note("Branch", None);
        let in_branch = tree.create_note("Inside", Some(branch_root));
        tree.set_body(in_branch, "shared keyword here");
        let outside_branch = tree.create_note("Outside", None);
        tree.set_body(outside_branch, "shared keyword too");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let branch_ids = tree.subtree_ids(branch_root);
        let facets = SearchFacets {
            branch: Some(&branch_ids),
            ..Default::default()
        };
        let hits = index
            .search_faceted("default", "keyword", &facets)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, in_branch);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_with_an_empty_branch_returns_nothing() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let note = tree.create_note("Note", None);
        tree.set_body(note, "keyword");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let facets = SearchFacets {
            branch: Some(&[]),
            ..Default::default()
        };
        assert!(index
            .search_faceted("default", "keyword", &facets)
            .unwrap()
            .is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_combines_with_tag_any() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let (rust_note, mut note) = tagged_note("Rust ownership", &["rust"]);
        note.body = "shared keyword".to_string();
        tree.insert_loaded(rust_note, note);
        let (other_note, mut note) = tagged_note("Go channels", &["go"]);
        note.body = "shared keyword".to_string();
        tree.insert_loaded(other_note, note);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let tags = vec!["rust".to_string()];
        let facets = SearchFacets {
            tags: Some((&tags, TagFilterOp::Any)),
            ..Default::default()
        };
        let hits = index
            .search_faceted("default", "keyword", &facets)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, rust_note);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_combines_with_tag_all() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let (both_tags, mut note) = tagged_note("Has Both", &["rust", "lang"]);
        note.body = "shared keyword".to_string();
        tree.insert_loaded(both_tags, note);
        let (one_tag, mut note) = tagged_note("Has One", &["rust"]);
        note.body = "shared keyword".to_string();
        tree.insert_loaded(one_tag, note);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let tags = vec!["rust".to_string(), "lang".to_string()];
        let facets = SearchFacets {
            tags: Some((&tags, TagFilterOp::All)),
            ..Default::default()
        };
        let hits = index
            .search_faceted("default", "keyword", &facets)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, both_tags);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_with_empty_tags_returns_nothing() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let note = tree.create_note("Note", None);
        tree.set_body(note, "keyword");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let tags: Vec<String> = vec![];
        let facets = SearchFacets {
            tags: Some((&tags, TagFilterOp::Any)),
            ..Default::default()
        };
        assert!(index
            .search_faceted("default", "keyword", &facets)
            .unwrap()
            .is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn search_faceted_restricts_by_date_range() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let mut old_note = crate::note::Note::new("Old", None);
        old_note.body = "shared keyword".to_string();
        old_note.updated = OffsetDateTime::parse("2020-01-01T00:00:00Z", &Rfc3339).unwrap();
        let old_id = NoteId::new();
        tree.insert_loaded(old_id, old_note);

        let mut recent_note = crate::note::Note::new("Recent", None);
        recent_note.body = "shared keyword".to_string();
        recent_note.updated = OffsetDateTime::parse("2026-06-01T00:00:00Z", &Rfc3339).unwrap();
        let recent_id = NoteId::new();
        tree.insert_loaded(recent_id, recent_note);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let facets = SearchFacets {
            date_range: Some((
                OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339).unwrap(),
                OffsetDateTime::parse("2026-12-31T00:00:00Z", &Rfc3339).unwrap(),
            )),
            ..Default::default()
        };
        let hits = index
            .search_faceted("default", "keyword", &facets)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, recent_id);

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
            .filter_by_tags(&["default"], &tags, TagFilterOp::Any)
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
            .filter_by_tags(&["default"], &tags, TagFilterOp::All)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, rust_note);

        let no_match = vec!["rust".to_string(), "go".to_string()];
        assert!(index
            .filter_by_tags(&["default"], &no_match, TagFilterOp::All)
            .unwrap()
            .is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn filter_by_tags_spans_exactly_the_given_vault_ids() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let (note_a, note) = tagged_note("A", &["shared"]);
        tree_a.insert_loaded(note_a, note);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        let mut tree_b = Tree::new();
        let (note_b, note) = tagged_note("B", &["shared"]);
        tree_b.insert_loaded(note_b, note);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();
        index.reindex("b", &tree_b, &vault_b).unwrap();

        let tags = vec!["shared".to_string()];
        // A vault_ids list of just "a" doesn't reach into "b" — tags
        // spanning every mounted vault is the caller's choice (passing
        // every mounted vault's id), not something filter_by_tags
        // assumes on its own.
        assert_eq!(
            index
                .filter_by_tags(&["a"], &tags, TagFilterOp::Any)
                .unwrap()
                .len(),
            1
        );
        assert!(index
            .filter_by_tags(&["c"], &tags, TagFilterOp::Any)
            .unwrap()
            .is_empty());

        // Passing both ids is what "tags span every mounted vault" (see
        // App::mounted_vault_ids) actually looks like: both notes come
        // back, each correctly labeled with its own vault_id.
        let mut both = index
            .filter_by_tags(&["a", "b"], &tags, TagFilterOp::Any)
            .unwrap();
        both.sort_by(|x, y| x.title.cmp(&y.title));
        assert_eq!(both.len(), 2);
        assert_eq!(both[0].vault_id, "a");
        assert_eq!(both[1].vault_id, "b");

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn filter_by_tags_with_no_tags_returns_nothing() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        assert!(index
            .filter_by_tags(&["default"], &[], TagFilterOp::Any)
            .unwrap()
            .is_empty());
        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn filter_by_tags_with_no_vault_ids_returns_nothing() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        let tags = vec!["shared".to_string()];
        assert!(index
            .filter_by_tags(&[], &tags, TagFilterOp::Any)
            .unwrap()
            .is_empty());
        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn all_tags_lists_each_distinct_tag_alphabetically_with_its_note_count() {
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

        let tags = index.all_tags(&["default"]).unwrap();
        assert_eq!(
            tags,
            vec![
                ("go".to_string(), 1),
                ("lang".to_string(), 2),
                ("rust".to_string(), 1),
            ]
        );

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn all_tags_spans_exactly_the_given_vault_ids_and_sums_shared_tag_counts() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let (id, note) = tagged_note("A note", &["only-in-a", "shared"]);
        tree_a.insert_loaded(id, note);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        let mut tree_b = Tree::new();
        let (id, note) = tagged_note("B note", &["shared"]);
        tree_b.insert_loaded(id, note);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();
        index.reindex("b", &tree_b, &vault_b).unwrap();

        assert!(index.all_tags(&["c"]).unwrap().is_empty());
        assert_eq!(
            index.all_tags(&["a"]).unwrap(),
            vec![
                ("only-in-a".to_string(), 1),
                ("shared".to_string(), 1),
            ]
        );
        // "shared" appears in both vaults — spanning both sums the count
        // rather than reporting it once per vault, same "tags are
        // transversal" instinct the whole feature is built on.
        assert_eq!(
            index.all_tags(&["a", "b"]).unwrap(),
            vec![
                ("only-in-a".to_string(), 1),
                ("shared".to_string(), 2),
            ]
        );

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn all_tags_is_empty_for_a_vault_with_no_tags() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        assert!(index.all_tags(&["default"]).unwrap().is_empty());
        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn all_tags_with_no_vault_ids_returns_nothing() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        assert!(index.all_tags(&[]).unwrap().is_empty());
        std::fs::remove_file(&db_path).ok();
    }

    fn links_for(index: &Index, vault_id: &str) -> Vec<(String, String)> {
        let mut stmt = index
            .conn
            .prepare(
                "SELECT source, target FROM links
                 WHERE source_vault = ?1 ORDER BY source, target",
            )
            .unwrap();
        stmt.query_map(params![vault_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn reindex_resolves_a_wikilink_to_the_matching_note() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let target = tree.create_note("Target Note", None);
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Target Note]] for details.");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        assert_eq!(
            links_for(&index, "default"),
            vec![(source.0.to_string(), target.0.to_string())]
        );

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_fans_out_a_wikilink_to_every_note_sharing_that_title() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let dup_a = tree.create_note("Debugging", None);
        let dup_b = tree.create_note("Debugging", None);
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Debugging]].");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let mut expected = vec![
            (source.0.to_string(), dup_a.0.to_string()),
            (source.0.to_string(), dup_b.0.to_string()),
        ];
        expected.sort();
        assert_eq!(links_for(&index, "default"), expected);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_skips_a_wikilink_whose_title_matches_no_note() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Nonexistent Title]].");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        assert!(links_for(&index, "default").is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_skips_self_links() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "Refers to [[Source Note]] itself.");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        assert!(links_for(&index, "default").is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_scopes_links_to_their_vault_id() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let target_a = tree_a.create_note("Target", None);
        let source_a = tree_a.create_note("Source", None);
        tree_a.set_body(source_a, "[[Target]]");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        assert_eq!(
            links_for(&index, "a"),
            vec![(source_a.0.to_string(), target_a.0.to_string())]
        );
        assert!(links_for(&index, "b").is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
    }

    #[test]
    fn backlinks_returns_every_note_linking_to_the_target() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let target = tree.create_note("Target Note", None);
        let source_a = tree.create_note("Source A", None);
        tree.set_body(source_a, "[[Target Note]]");
        let source_b = tree.create_note("Source B", None);
        tree.set_body(source_b, "also [[Target Note]]");
        let unrelated = tree.create_note("Unrelated", None);
        tree.set_body(unrelated, "no link here");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let hits = index.backlinks("default", target).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Source A");
        assert_eq!(hits[1].title, "Source B");
        assert!(!hits.iter().any(|h| h.note_id == unrelated));

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn backlinks_is_empty_for_a_note_nothing_links_to() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let lonely = tree.create_note("Lonely Note", None);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        assert!(index.backlinks("default", lonely).unwrap().is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn backlinks_is_scoped_to_its_vault_id() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let target_a = tree_a.create_note("Target", None);
        let source_a = tree_a.create_note("Source", None);
        tree_a.set_body(source_a, "[[Target]]");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        index.reindex("a", &tree_a, &vault_a).unwrap();

        assert_eq!(index.backlinks("a", target_a).unwrap().len(), 1);
        assert!(index.backlinks("b", target_a).unwrap().is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
    }

    #[test]
    fn reindex_reports_a_wikilink_whose_title_matches_no_note_as_broken() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Nonexistent Title]].");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        let report = index.reindex("default", &tree, &vault).unwrap();

        assert_eq!(report.broken_links.len(), 1);
        assert_eq!(report.broken_links[0].source, source);
        assert_eq!(report.broken_links[0].title, "Nonexistent Title");

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn reindex_does_not_report_a_resolved_wikilink_as_broken() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        tree.create_note("Target Note", None);
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Target Note]].");

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        let report = index.reindex("default", &tree, &vault).unwrap();

        assert!(report.broken_links.is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn link_count_for_subtree_counts_internal_incoming_and_outgoing_links_once_each() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let a = tree.create_note("A", None);
        let _b = tree.create_note("B", Some(a));
        let _c = tree.create_note("C", None);
        let d = tree.create_note("D", None);
        let e = tree.create_note("E", None);
        let _f = tree.create_note("F", None);
        tree.set_body(a, "[[C]] and [[B]]"); // A -> C (outgoing), A -> B (internal)
        tree.set_body(d, "[[B]]"); // D -> B (incoming)
        tree.set_body(e, "[[F]]"); // unrelated to the subtree

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let subtree = tree.subtree_ids(a);
        assert_eq!(subtree.len(), 2); // A and B

        let count = index
            .link_count_for_subtree("default", &subtree)
            .unwrap();
        assert_eq!(count, 3); // A->C, A->B, D->B

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn link_count_for_subtree_is_zero_for_an_unconnected_note() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let lonely = tree.create_note("Lonely", None);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let subtree = tree.subtree_ids(lonely);
        assert_eq!(index.link_count_for_subtree("default", &subtree).unwrap(), 0);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn link_count_for_subtree_with_no_ids_is_zero() {
        let db_path = scratch_db_path();
        let index = Index::open(&db_path).unwrap();
        assert_eq!(index.link_count_for_subtree("default", &[]).unwrap(), 0);
        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn reindex_mounted_resolves_wikilinks_across_vaults() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Target In B]].");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();

        let mut tree_b = Tree::new();
        let target = tree_b.create_note("Target In B", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();

        let reports = index
            .reindex_mounted(&[("a", &tree_a, &vault_a), ("b", &tree_b, &vault_b)])
            .unwrap();
        assert!(reports.iter().all(|r| r.broken_links.is_empty()));

        // The link is recorded once, under its source vault ("a"); "b"
        // never gets an outgoing-link row for it.
        assert_eq!(links_for(&index, "a"), vec![(source.0.to_string(), target.0.to_string())]);
        assert!(links_for(&index, "b").is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn reindex_mounted_fans_out_across_vaults_when_ambiguous() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Shared]].");
        let dup_in_a = tree_a.create_note("Shared", None);
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();

        let mut tree_b = Tree::new();
        let dup_in_b = tree_b.create_note("Shared", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();

        index
            .reindex_mounted(&[("a", &tree_a, &vault_a), ("b", &tree_b, &vault_b)])
            .unwrap();

        let mut hits = links_for(&index, "a");
        hits.sort();
        let mut expected = vec![
            (source.0.to_string(), dup_in_a.0.to_string()),
            (source.0.to_string(), dup_in_b.0.to_string()),
        ];
        expected.sort();
        assert_eq!(hits, expected);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn reindex_does_not_resolve_against_a_vault_left_out_of_the_batch() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        // "b" gets indexed once, on its own.
        let mut tree_b = Tree::new();
        tree_b.create_note("Target In B", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();
        index.reindex("b", &tree_b, &vault_b).unwrap();

        // Later, "a" is reindexed alone (not batched with "b") and links to
        // a title that only exists in "b". Even though "b"'s notes are
        // still sitting in the table, they must not count as known-good
        // for this narrower reindex.
        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Target In B]].");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();
        let report = index.reindex("a", &tree_a, &vault_a).unwrap();

        assert_eq!(report.broken_links.len(), 1);
        assert_eq!(report.broken_links[0].title, "Target In B");
        assert!(links_for(&index, "a").is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn backlinks_finds_a_source_note_in_a_different_vault() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Target]].");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();

        let mut tree_b = Tree::new();
        let target = tree_b.create_note("Target", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();

        index
            .reindex_mounted(&[("a", &tree_a, &vault_a), ("b", &tree_b, &vault_b)])
            .unwrap();

        let hits = index.backlinks("b", target).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, source);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn outgoing_links_returns_every_note_a_source_links_to() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let source = tree.create_note("Source Note", None);
        tree.set_body(source, "See [[Target A]] and also [[Target B]].");
        let target_a = tree.create_note("Target A", None);
        let target_b = tree.create_note("Target B", None);
        let unrelated = tree.create_note("Unrelated", None);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        let hits = index.outgoing_links("default", source).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].note_id, target_a);
        assert_eq!(hits[1].note_id, target_b);
        assert!(!hits.iter().any(|h| h.note_id == unrelated));

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn outgoing_links_is_empty_for_a_note_with_no_links() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree = Tree::new();
        let lonely = tree.create_note("Lonely Note", None);

        let vault_dir = temp_vault_dir();
        let vault = Vault::open(vault_dir.clone()).unwrap();
        index.reindex("default", &tree, &vault).unwrap();

        assert!(index.outgoing_links("default", lonely).unwrap().is_empty());

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_dir).ok();
    }

    #[test]
    fn outgoing_links_finds_a_target_note_in_a_different_vault() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Target]].");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();

        let mut tree_b = Tree::new();
        let target = tree_b.create_note("Target", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();

        index
            .reindex_mounted(&[("a", &tree_a, &vault_a), ("b", &tree_b, &vault_b)])
            .unwrap();

        let hits = index.outgoing_links("a", source).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, target);
        assert_eq!(hits[0].vault_id, "b");

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }

    #[test]
    fn link_count_for_subtree_counts_a_cross_vault_link() {
        let db_path = scratch_db_path();
        let mut index = Index::open(&db_path).unwrap();

        let mut tree_a = Tree::new();
        let source = tree_a.create_note("Source", None);
        tree_a.set_body(source, "See [[Target]].");
        let vault_a_dir = temp_vault_dir();
        let vault_a = Vault::open(vault_a_dir.clone()).unwrap();

        let mut tree_b = Tree::new();
        let target = tree_b.create_note("Target", None);
        let vault_b_dir = temp_vault_dir();
        let vault_b = Vault::open(vault_b_dir.clone()).unwrap();

        index
            .reindex_mounted(&[("a", &tree_a, &vault_a), ("b", &tree_b, &vault_b)])
            .unwrap();

        assert_eq!(
            index
                .link_count_for_subtree("a", &tree_a.subtree_ids(source))
                .unwrap(),
            1
        );
        assert_eq!(
            index
                .link_count_for_subtree("b", &tree_b.subtree_ids(target))
                .unwrap(),
            1
        );

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir_all(&vault_a_dir).ok();
        std::fs::remove_dir_all(&vault_b_dir).ok();
    }
}
