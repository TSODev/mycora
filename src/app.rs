use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;

use crate::config::{Config, VaultEntry};
use crate::index::{Index, IndexedNote, SearchHit, TagFilterOp};
use crate::lang::Lang;
use crate::note::{Note, NoteId};
use crate::session::Session;
use crate::tree::Tree;
use crate::vault::Vault;

/// Expands every ancestor of `id` in `tree` so it's reachable in a
/// depth-first, collapse-respecting traversal — shared by `App::reveal`
/// (used after a search/backlinks jump) and `App::new`'s session restore
/// (used before `App` itself exists, hence a free function rather than a
/// method).
fn reveal_ancestors(tree: &Tree, expanded: &mut HashSet<NoteId>, id: NoteId) {
    let mut current = tree.get(id).and_then(|note| note.parent);
    while let Some(ancestor) = current {
        expanded.insert(ancestor);
        current = tree.get(ancestor).and_then(|note| note.parent);
    }
}

/// A vault mounted alongside the primary one: loaded and indexed (so its
/// notes count toward search/backlinks/link-count badges under its own
/// `vault_id`, and its wikilinks can resolve cross-vault against the
/// primary one's), but not navigable or editable yet. Full multi-vault
/// editing needs every mutating `App` method to first resolve which vault a
/// given `NoteId` belongs to — deferred to a later pass (see ROADMAP.md's
/// "Multiple vaults" entry). Only its top-level roots are ever shown, and
/// always collapsed: there's no expand/collapse interaction to offer when
/// nothing here can become `selected`. `vault` is kept (not just `tree`)
/// because reindexing needs it for note-path lookups, even though nothing
/// ever writes through it.
struct ReadOnlyVault {
    id: String,
    tree: Tree,
    vault: Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    /// Awaiting y/n confirmation for `pending_delete`.
    ConfirmDelete,
    /// Typing a full-text query; `search_results` updates on every keystroke.
    Search,
    /// Browsing notes that link to the note selected when `b` was pressed.
    Backlinks,
    /// Editing the selected note's Markdown body in a full-pane overlay
    /// (see `App::body_editor`'s doc comment for why full-pane rather than
    /// a split layout). `Esc` saves and returns to Normal — there's no
    /// separate discard-without-saving path; `u` in Normal mode afterward
    /// undoes the whole edit session as one step if you change your mind.
    EditBody,
    /// Typing a `:` command; the input replaces the status bar's hint row
    /// only (see `ui.rs`'s `draw_hint_row`), same as `ConfirmDelete`'s
    /// prompt — the split-pane layout stays visible underneath, unlike
    /// `Search`/`EditBody`'s full-pane overlays. See `App::execute_command`
    /// for the command set.
    Command,
    /// Browsing the notes a `:tags` command matched — full-pane overlay,
    /// same interaction shape as `Search` (`Up`/`Down` move, `Enter`
    /// jumps, `Esc` cancels) but over a fixed result set rather than a
    /// live-as-you-type query.
    TagResults,
    /// Browsing every distinct tag in the active vault (`:tags list`) —
    /// same full-pane shape as `TagResults`, but `Enter` doesn't jump to a
    /// note: it filters by the selected tag, transitioning into
    /// `TagResults` for that tag (see `App::confirm_tag_list`). Exists so
    /// you can pick a tag to filter by without having to already know and
    /// type its exact spelling.
    TagList,
}

/// An action that can be pushed onto `undo_stack`/`redo_stack`. Applying one
/// always returns its own inverse, built from the *current* live tree state
/// rather than a value frozen at record time — so a chain of undo/redo stays
/// correct even if the note was edited again in between.
enum UndoAction {
    Rename { id: NoteId, title: String },
    Move { id: NoteId, parent: Option<NoteId> },
    Reorder { id: NoteId, move_down: bool },
    /// Applying this deletes `root_id`'s subtree (to trash) and produces a
    /// `Restore` holding what was removed.
    Remove { root_id: NoteId },
    /// Applying this reinserts a previously removed subtree and produces a
    /// `Remove` pointing back at its (now live again) root.
    Restore { snapshot: Vec<(NoteId, Note)> },
    /// Applying this replaces `id`'s body with `body` and produces another
    /// `EditBody` holding what the body was before — one entry per whole
    /// edit session, not per keystroke.
    EditBody { id: NoteId, body: String },
    /// Applying this replaces `id`'s whole tag list and produces another
    /// `SetTags` holding what it was before — `:tag add`/`:tag del` each
    /// record one of these per invocation, not per tag, same "one entry
    /// per user action" shape as `EditBody`.
    SetTags { id: NoteId, tags: Vec<String> },
}

pub struct App {
    pub tree: Tree,
    pub vault: Vault,
    pub expanded: HashSet<NoteId>,
    pub selected: Option<NoteId>,
    pub mode: Mode,
    /// Interface language, from `config.toml`'s `language` key — every
    /// user-facing label/hint/message renders through this (see
    /// `crate::lang::Lang`). Read directly by `ui.rs` too.
    pub lang: Lang,
    pub input: String,
    pub should_quit: bool,
    pub last_error: Option<String>,
    /// Set by a successful `:` command (e.g. `:reindex`'s note count) —
    /// shown in the status bar like `last_error`, just not in red. Cleared
    /// whenever the other one is set, so only the most recent outcome of
    /// the two shows.
    pub last_message: Option<String>,
    /// Set by a first `q` press; a second press actually quits, any other
    /// key resets it. Mirrors Terapi's q/q confirm dance.
    pub confirm_quit: bool,
    /// Note pending a delete confirmation (`Mode::ConfirmDelete`).
    pending_delete: Option<NoteId>,
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
    index: Index,
    /// The active vault's registry name — used as the index's `vault_id`.
    vault_id: String,
    search_query: String,
    search_results: Vec<SearchHit>,
    search_selected: usize,
    /// Index into `live_backlinks()` while `mode == Mode::Backlinks` — see
    /// `focus_backlinks`'s doc comment for why there's no cached results
    /// field alongside it.
    backlinks_selected: usize,
    /// Every other mounted vault (see `Config::mounted_vaults`), read-only.
    other_vaults: Vec<ReadOnlyVault>,
    /// Registered vaults that are *not* currently mounted and not
    /// archived — nothing is loaded for them, but each still gets a
    /// single unexpandable placeholder row in the tree (see
    /// `TreeRow::UnmountedVault`) so their existence isn't invisible
    /// until a restart after `mycora vault mount`.
    unmounted_vaults: Vec<VaultEntry>,
    /// Registered vaults that are archived (`mycora vault archive`) —
    /// like `unmounted_vaults` but with nothing at `path` to load or
    /// mount at all, just a compressed file elsewhere (see
    /// `TreeRow::ArchivedVault`).
    archived_vaults: Vec<VaultEntry>,
    /// Set instead of `selected` when the current row is an unmounted
    /// vault's placeholder rather than a note — mutually exclusive with
    /// `selected` and `selected_archived_vault` (see
    /// `set_selected`/`set_selected_unmounted_vault`/
    /// `set_selected_archived_vault`), exactly one of the three is
    /// `Some` whenever anything is highlighted in the tree pane at all.
    selected_unmounted_vault: Option<String>,
    /// The `selected_archived_vault` counterpart to
    /// `selected_unmounted_vault` — see that field's doc comment.
    selected_archived_vault: Option<String>,
    /// Whether `TreeRow::UnmountedVault`/`TreeRow::ArchivedVault` rows
    /// show in the tree at all (`:config unmount show/hide`, `:config
    /// archive show/hide`) — persisted in `Session`, vault-agnostic like
    /// `pane_widths`. `true` by default (nothing hidden until the user
    /// asks).
    show_unmounted: bool,
    show_archived: bool,
    /// When `Some`, `:tags`/`:tags list` are restricted to just this one
    /// mounted vault instead of spanning all of them (`:tags limit
    /// <name>` / `:tags unlimit`) — a temporary working focus, not a
    /// display preference like `show_unmounted`/`show_archived`, so
    /// deliberately *not* persisted in `Session`: it always starts
    /// `None` (global) on a fresh launch rather than leaving a limit
    /// active from days ago as a surprise.
    tags_limit: Option<String>,
    /// The active `ratatui-textarea` widget state while `mode ==
    /// Mode::EditBody`; `None` otherwise. Full-pane overlay rather than a
    /// split layout (tree + body pane) — the latter is its own separate
    /// v0.7 roadmap item (resizable panes, etc.); this gets editing
    /// working without waiting on that. Which note is being edited is
    /// just `selected` — navigation is disabled in `EditBody` mode so it
    /// can't change out from under the editor.
    body_editor: Option<TextArea<'static>>,
    /// Where `save_session` writes on exit — computed once from
    /// `config.home` so later saves don't need a `Config` around.
    session_path: PathBuf,
    /// Where `:lang` persists a language switch (`Config::set_language`)
    /// — kept for the same reason as `session_path`.
    config_path: PathBuf,
    /// Percent widths of the split layout's three columns (tree, body,
    /// backlinks — see `ui.rs`'s `draw_main`), always summing to 100.
    /// Persisted in `Session` (vault-agnostic, unlike `selected`/
    /// `expanded`) and restored in `App::new`, falling back to the
    /// default (40/40/20) if nothing was saved or the saved widths fail
    /// validation (don't sum to 100, or a pane is below `PANE_MIN_PCT`).
    pane_widths: [u16; 3],
    /// Text typed after `:` while `mode == Mode::Command`.
    command_input: String,
    tag_results: Vec<IndexedNote>,
    tag_results_selected: usize,
    /// `(tag, note count)` pairs while `mode == Mode::TagList`.
    tag_list: Vec<(String, i64)>,
    tag_list_selected: usize,
    /// Vertical scroll offset (rendered lines) into the body preview pane
    /// — see `App::scroll_body_down`/`scroll_body_up`. Reset to 0 by
    /// `set_selected` every time the selection changes, so a freshly
    /// selected note always starts at the top.
    body_scroll: u16,
}

/// One row in the tree pane, as returned by `App::visible_rows`: a note
/// (possibly in a read-only mounted vault), a `── vault name ──`
/// separator marking where a read-only vault's section begins, or a
/// registered-but-unmounted vault's placeholder row. Separators aren't
/// navigable — `App::move_selection` skips them.
pub enum TreeRow {
    Note {
        id: NoteId,
        depth: usize,
        title: String,
        has_children: bool,
        expanded: bool,
        link_count: i64,
        /// `false` for anything outside the active vault.
        editable: bool,
    },
    VaultSeparator(String),
    /// A registered vault that isn't currently mounted — nothing is
    /// loaded for it (no `Tree`, no `Vault`), so unlike `Note` it can
    /// never expand, and selecting it sets `App::selected_unmounted_vault`
    /// instead of `App::selected` (there's no `NoteId` to hold). The body
    /// preview shows how to mount it instead of a note body — see
    /// `App::selected_unmounted_vault_info`. Hidden entirely when
    /// `App::show_unmounted` is `false` (`:config unmount hide`).
    UnmountedVault { name: String, path: PathBuf },
    /// A registered vault that's been compressed via `mycora vault
    /// archive` — like `UnmountedVault` but nothing exists at `path` to
    /// mount at all (it's compressed at `archive_path` instead), so the
    /// body preview points at `mycora vault unarchive` rather than
    /// `vault mount`. Selecting it sets `App::selected_archived_vault`.
    /// Hidden entirely when `App::show_archived` is `false` (`:config
    /// archive hide`).
    ArchivedVault { name: String, archive_path: PathBuf },
}

impl App {
    /// Loads config + vault from disk and returns the ready-to-run app along
    /// with any load warnings (malformed files, orphaned/duplicate ids) for
    /// the caller to print before the TUI takes over the terminal.
    pub fn new() -> anyhow::Result<(Self, Vec<String>)> {
        let config = Config::load()?;
        let lang = config.language;
        let active = config.active_vault().clone();
        // Excludes `active` itself even if its own `mounted` flag is
        // `false` — that can happen via `Config::active_vault`'s
        // self-heal (see below), and it's actively loaded regardless, so
        // showing it *again* as an unmounted placeholder would be wrong.
        // Archived vaults get their own separate list/row type
        // (`TreeRow::ArchivedVault`) rather than showing up here too —
        // `TreeRow::UnmountedVault`'s body preview tells you to `mycora
        // vault mount <name>`, which would be wrong for one (nothing
        // exists at `path` to mount — it's compressed elsewhere).
        let unmounted_vaults: Vec<VaultEntry> = config
            .vaults
            .iter()
            .filter(|v| !v.mounted && v.archived.is_none() && v.name != active.name)
            .cloned()
            .collect();
        let archived_vaults: Vec<VaultEntry> = config
            .vaults
            .iter()
            .filter(|v| v.archived.is_some() && v.name != active.name)
            .cloned()
            .collect();

        // Load every mounted vault (primary included) before indexing any
        // of them — cross-vault wikilink resolution needs every vault's
        // notes visible to the index together, not one at a time (see
        // `Index::reindex_mounted`'s doc comment). `active` is loaded even
        // if it isn't itself in `mounted_vaults()`: `Config::active_vault`
        // self-heals by returning *some* vault even when every registry
        // entry has `mounted = false` (e.g. after `vault unmount`ing all
        // of them), and that self-healed pick needs to actually be
        // loadable, not just named.
        let mut to_load: Vec<&VaultEntry> = config.mounted_vaults().collect();
        if !to_load.iter().any(|entry| entry.name == active.name) {
            to_load.push(&active);
        }
        let mut loaded: Vec<(String, Tree, Vault)> = Vec::new();
        let mut warnings = Vec::new();
        for entry in to_load {
            let mut v = Vault::open(entry.path.clone())?;
            let (t, r) = v.load()?;
            for warning in &r.warnings {
                if entry.name == active.name {
                    warnings.push(warning.clone());
                } else {
                    warnings.push(format!("[{}] {warning}", entry.name));
                }
            }
            loaded.push((entry.name.clone(), t, v));
        }

        let primary_idx = loaded
            .iter()
            .position(|(name, _, _)| *name == active.name)
            .expect("active is always pushed onto to_load above if not already in it");
        let (_, mut tree, mut vault) = loaded.remove(primary_idx);

        let mut selected = if tree.roots().is_empty() {
            let welcome = tree.create_note(lang.welcome_title(), None);
            tree.set_body(welcome, lang.welcome_body());
            if let Some(note) = tree.get(welcome) {
                vault.save_note(welcome, note)?;
            }
            Some(welcome)
        } else {
            tree.roots().first().copied()
        };

        let mut expanded = HashSet::new();
        if let Some(id) = selected {
            expanded.insert(id);
        }

        // Restore last session's selection/expand state for this vault, if
        // any was saved — ids that no longer resolve (the note was deleted
        // or the vault changed since) are dropped rather than kept
        // dangling. Falls back to the defaults just computed above when
        // nothing was saved, or when the saved selection no longer exists.
        let session_path = Session::default_path(&config.home);
        let config_path = Config::default_path(&config.home);
        let session = Session::load(&session_path);
        if let Some((saved_selected, saved_expanded)) = session.for_vault(&active.name) {
            expanded = saved_expanded
                .into_iter()
                .filter(|id| tree.get(*id).is_some())
                .collect();
            if let Some(id) = saved_selected.filter(|id| tree.get(*id).is_some()) {
                selected = Some(id);
                // Guarantee the restored selection is actually visible,
                // regardless of what the saved expanded set had.
                reveal_ancestors(&tree, &mut expanded, id);
            }
        }

        // A hand-edited or stale session file could hold widths that no
        // longer sum to 100 or dip below the resize floor — fall back to
        // the default rather than handing `ui.rs` a layout it can't
        // render sanely.
        let pane_widths = session
            .pane_widths()
            .filter(|widths| {
                widths.iter().sum::<u16>() == 100
                    && widths.iter().all(|w| *w >= Self::PANE_MIN_PCT)
            })
            .unwrap_or(Self::DEFAULT_PANE_WIDTHS);
        let show_unmounted = session.show_unmounted().unwrap_or(true);
        let show_archived = session.show_archived().unwrap_or(true);

        let index_path = Index::default_path(&config.home);
        let mut index = Index::open(&index_path)?;
        // Reindex every mounted vault together at startup, so search and
        // cross-vault links reflect them as loaded without requiring
        // `mycora reindex` to have been run separately — the index is
        // disposable, so rebuilding it here is just as valid a source as
        // an on-disk one from a previous session.
        let mut batch: Vec<(&str, &Tree, &Vault)> = vec![(active.name.as_str(), &tree, &vault)];
        for (name, t, v) in &loaded {
            batch.push((name.as_str(), t, v));
        }
        let reports = index.reindex_mounted(&batch)?;
        for ((vault_name, _, _), r) in batch.iter().zip(reports.iter()) {
            for broken in &r.broken_links {
                let source_title = if *vault_name == active.name {
                    tree.get(broken.source).map(|n| n.title.as_str())
                } else {
                    loaded
                        .iter()
                        .find(|(name, _, _)| name == vault_name)
                        .and_then(|(_, t, _)| t.get(broken.source))
                        .map(|n| n.title.as_str())
                }
                .unwrap_or("?");
                let prefix = if *vault_name == active.name {
                    String::new()
                } else {
                    format!("[{vault_name}] ")
                };
                warnings.push(format!(
                    "{prefix}broken link in \"{source_title}\": [[{}]] matches no note",
                    broken.title
                ));
            }
        }

        // Every other mounted vault stays loaded, just not wired into
        // `tree`/`vault`/`selected` — read-only for now (see
        // `ReadOnlyVault`'s doc comment).
        let other_vaults = loaded
            .into_iter()
            .map(|(id, tree, vault)| ReadOnlyVault { id, tree, vault })
            .collect();

        let app = Self {
            tree,
            vault,
            expanded,
            selected,
            mode: Mode::Normal,
            lang,
            input: String::new(),
            should_quit: false,
            last_error: None,
            last_message: None,
            confirm_quit: false,
            pending_delete: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            index,
            vault_id: active.name,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            backlinks_selected: 0,
            other_vaults,
            unmounted_vaults,
            archived_vaults,
            selected_unmounted_vault: None,
            selected_archived_vault: None,
            show_unmounted,
            show_archived,
            tags_limit: None,
            body_editor: None,
            session_path,
            config_path,
            pane_widths,
            command_input: String::new(),
            tag_results: Vec::new(),
            tag_results_selected: 0,
            tag_list: Vec::new(),
            tag_list_selected: 0,
            body_scroll: 0,
        };

        Ok((app, warnings))
    }

    /// Saves this vault's current selection/expand state, for the next
    /// `App::new()` to restore. Called once at shutdown (see `main.rs`),
    /// not write-through — see `Session`'s doc comment for why.
    pub fn save_session(&self) -> anyhow::Result<()> {
        let mut session = Session::load(&self.session_path);
        session.save(
            &self.vault_id,
            self.selected,
            &self.expanded,
            self.pane_widths,
            self.show_unmounted,
            self.show_archived,
        )
    }

    /// Depth-first (id, depth) pairs for the *active* vault only,
    /// respecting collapse state. Private — only `neighbor_after` (picking
    /// where selection lands after a delete, which only ever happens in
    /// the active vault) still needs an active-only list; rendering and
    /// `move_selection` use the cross-vault `visible_rows` below.
    fn visible_active_notes(&self) -> Vec<(NoteId, usize)> {
        let mut out = Vec::new();
        for &root in self.tree.roots() {
            self.push_visible_active(root, 0, &mut out);
        }
        out
    }

    fn push_visible_active(&self, id: NoteId, depth: usize, out: &mut Vec<(NoteId, usize)>) {
        out.push((id, depth));
        if self.expanded.contains(&id) {
            for &child in self.tree.children(id) {
                self.push_visible_active(child, depth + 1, out);
            }
        }
    }

    /// The `Tree` that owns `id` and that tree's vault id — the active
    /// vault first, else whichever read-only mounted vault's tree
    /// contains it, else `None` if `id` isn't loaded anywhere right now.
    /// Backbone for every read accessor that must work regardless of
    /// which vault the current selection happens to be in.
    fn resolve(&self, id: NoteId) -> Option<(&Tree, &str)> {
        if self.tree.get(id).is_some() {
            return Some((&self.tree, self.vault_id.as_str()));
        }
        self.other_vaults
            .iter()
            .find(|v| v.tree.get(id).is_some())
            .map(|v| (&v.tree, v.id.as_str()))
    }

    /// Every currently-loaded vault's id — the active one plus every
    /// read-only mounted one — for read operations that deliberately
    /// span all of them at once (`:tags`/`:tags list`, see
    /// `Index::filter_by_tags`'s doc comment for why tags work this way
    /// while `/` search doesn't).
    fn mounted_vault_ids(&self) -> Vec<&str> {
        let mut ids = vec![self.vault_id.as_str()];
        ids.extend(self.other_vaults.iter().map(|v| v.id.as_str()));
        ids
    }

    /// What `:tags`/`:tags list` actually query: every mounted vault, or
    /// just the one `:tags limit <name>` narrowed to, if any. `ui.rs`
    /// reads `tags_limit()` directly to show which in the overlay title.
    fn tags_scope(&self) -> Vec<&str> {
        match &self.tags_limit {
            Some(name) => vec![name.as_str()],
            None => self.mounted_vault_ids(),
        }
    }

    /// The vault `:tags`/`:tags list` are currently limited to, if any
    /// (`:tags limit <name>` / `:tags unlimit`) — `None` means every
    /// mounted vault.
    pub fn tags_limit(&self) -> Option<&str> {
        self.tags_limit.as_deref()
    }

    /// `true` iff `id` belongs to the active (editable) vault. Every
    /// mutating command checks this first and reports a clear error
    /// rather than silently no-oping or, worse, acting on the wrong
    /// vault — e.g. `create_child` would otherwise happily create a new
    /// note in the *active* vault, wrongly parented under a read-only
    /// vault's id, since `Tree::create_note` doesn't itself validate that
    /// a given parent id exists in `self.tree`.
    fn require_editable(&mut self, id: NoteId) -> bool {
        if self.tree.get(id).is_some() {
            true
        } else {
            self.last_message = None;
            self.last_error = Some(self.lang.read_only_vault().to_string());
            false
        }
    }

    /// Depth-first rows across *every* mounted vault — the active one
    /// first, then each read-only one behind its own separator, then one
    /// placeholder row per unmounted registry entry — used by both
    /// `ui.rs`'s tree rendering and `move_selection`. Read-only branches
    /// respect `self.expanded` exactly like the active tree does (ids are
    /// globally unique UUIDs, so the same set works across vaults); this
    /// replaces the old roots-only, always-collapsed `other_vault_sections`
    /// view with real navigation.
    pub fn visible_rows(&self) -> Vec<TreeRow> {
        let mut out = Vec::new();
        for &root in self.tree.roots() {
            self.push_visible_row(&self.tree, &self.vault_id, root, 0, true, &mut out);
        }
        for v in &self.other_vaults {
            out.push(TreeRow::VaultSeparator(v.id.clone()));
            for &root in v.tree.roots() {
                self.push_visible_row(&v.tree, &v.id, root, 0, false, &mut out);
            }
        }
        if self.show_unmounted {
            for entry in &self.unmounted_vaults {
                out.push(TreeRow::UnmountedVault {
                    name: entry.name.clone(),
                    path: entry.path.clone(),
                });
            }
        }
        if self.show_archived {
            for entry in &self.archived_vaults {
                // `archived` is always `Some` for anything in
                // `archived_vaults` — that's exactly how it was filtered
                // into this list in `App::new`.
                if let Some(archive_path) = &entry.archived {
                    out.push(TreeRow::ArchivedVault {
                        name: entry.name.clone(),
                        archive_path: archive_path.clone(),
                    });
                }
            }
        }
        out
    }

    fn push_visible_row(
        &self,
        tree: &Tree,
        vault_id: &str,
        id: NoteId,
        depth: usize,
        editable: bool,
        out: &mut Vec<TreeRow>,
    ) {
        let note = tree
            .get(id)
            .expect("visible row ids always resolve in their own tree");
        let has_children = !tree.children(id).is_empty();
        let is_expanded = self.expanded.contains(&id);
        let link_count = if has_children && !is_expanded {
            let subtree = tree.subtree_ids(id);
            self.index
                .link_count_for_subtree(vault_id, &subtree)
                .unwrap_or(0)
        } else {
            0
        };
        out.push(TreeRow::Note {
            id,
            depth,
            title: note.title.clone(),
            has_children,
            expanded: is_expanded,
            link_count,
            editable,
        });
        if is_expanded {
            for &child in tree.children(id) {
                self.push_visible_row(tree, vault_id, child, depth + 1, editable, out);
            }
        }
    }

    /// The single place `self.selected` is ever written — also clears
    /// `selected_unmounted_vault`/`selected_archived_vault` (all three
    /// are mutually exclusive) and resets `body_scroll` to 0, so a
    /// freshly selected note (or a fresh search/backlinks/tag-list jump)
    /// always starts at the top of the body preview rather than wherever
    /// a previous note happened to be scrolled to.
    fn set_selected(&mut self, id: Option<NoteId>) {
        self.selected = id;
        self.selected_unmounted_vault = None;
        self.selected_archived_vault = None;
        self.body_scroll = 0;
    }

    /// The `selected_unmounted_vault` counterpart to `set_selected` —
    /// clears `selected`/`selected_archived_vault` (mutually exclusive)
    /// and resets `body_scroll` the same way.
    fn set_selected_unmounted_vault(&mut self, name: Option<String>) {
        self.selected = None;
        self.selected_unmounted_vault = name;
        self.selected_archived_vault = None;
        self.body_scroll = 0;
    }

    /// The `selected_archived_vault` counterpart to `set_selected` — see
    /// `set_selected_unmounted_vault`.
    fn set_selected_archived_vault(&mut self, name: Option<String>) {
        self.selected = None;
        self.selected_unmounted_vault = None;
        self.selected_archived_vault = name;
        self.body_scroll = 0;
    }

    pub fn move_selection(&mut self, delta: isize) {
        enum Stop {
            Note(NoteId),
            Unmounted(String),
            Archived(String),
        }

        let stops: Vec<Stop> = self
            .visible_rows()
            .into_iter()
            .filter_map(|row| match row {
                TreeRow::Note { id, .. } => Some(Stop::Note(id)),
                TreeRow::VaultSeparator(_) => None,
                TreeRow::UnmountedVault { name, .. } => Some(Stop::Unmounted(name)),
                TreeRow::ArchivedVault { name, .. } => Some(Stop::Archived(name)),
            })
            .collect();
        if stops.is_empty() {
            self.set_selected(None);
            return;
        }

        let current_pos = stops
            .iter()
            .position(|stop| match stop {
                Stop::Note(id) => self.selected == Some(*id),
                Stop::Unmounted(name) => {
                    self.selected_unmounted_vault.as_deref() == Some(name.as_str())
                }
                Stop::Archived(name) => {
                    self.selected_archived_vault.as_deref() == Some(name.as_str())
                }
            })
            .unwrap_or(0);

        let len = stops.len() as isize;
        let new_pos = (current_pos as isize + delta).rem_euclid(len) as usize;
        match &stops[new_pos] {
            Stop::Note(id) => self.set_selected(Some(*id)),
            Stop::Unmounted(name) => self.set_selected_unmounted_vault(Some(name.clone())),
            Stop::Archived(name) => self.set_selected_archived_vault(Some(name.clone())),
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some(id) = self.selected {
            let has_children = self
                .resolve(id)
                .map(|(tree, _)| !tree.children(id).is_empty())
                .unwrap_or(false);
            if !has_children {
                return;
            }
            if !self.expanded.insert(id) {
                self.expanded.remove(&id);
            }
        }
    }

    pub fn expand_selected(&mut self) {
        if let Some(id) = self.selected {
            self.expanded.insert(id);
        }
    }

    pub fn collapse_selected(&mut self) {
        if let Some(id) = self.selected {
            self.expanded.remove(&id);
        }
    }

    pub fn create_sibling(&mut self) {
        // `let Some(id) = ... else { return }`, not `if let ... && !...`:
        // the latter only returns early when something selected turns out
        // not to be editable, but falls straight through — treating a
        // `None` selection (nothing selected, or an unmounted vault's
        // placeholder row) as "create at root" instead of a no-op. That
        // was reachable even before unmounted-vault rows existed (delete
        // the very last note and `selected` goes to `None` too), just
        // rare enough not to have been noticed.
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let parent = self.tree.get(id).and_then(|note| note.parent);
        let new_id = self.tree.create_note(self.lang.new_note_title(), parent);
        if let Some(parent) = parent {
            self.expanded.insert(parent);
        }
        self.set_selected(Some(new_id));
        self.persist(new_id);
        self.record(UndoAction::Remove { root_id: new_id });
        self.begin_naming();
    }

    pub fn create_child(&mut self) {
        if let Some(parent) = self.selected {
            if !self.require_editable(parent) {
                return;
            }
            let new_id = self.tree.create_note(self.lang.new_note_title(), Some(parent));
            self.expanded.insert(parent);
            self.set_selected(Some(new_id));
            self.persist(new_id);
            self.record(UndoAction::Remove { root_id: new_id });
            self.begin_naming();
        }
    }

    /// Deep-copies the selected note (and its subtree) as a new sibling
    /// right after it. Undoing removes the whole copy in one step.
    pub fn copy_selected(&mut self) {
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let Some(note) = self.tree.get(id) else {
            return;
        };
        let parent = note.parent;

        let Some(new_root) = self.tree.deep_copy(id, parent) else {
            return;
        };
        for copied_id in self.tree.subtree_ids(new_root) {
            self.persist(copied_id);
        }
        self.set_selected(Some(new_root));
        self.record(UndoAction::Remove {
            root_id: new_root,
        });
    }

    /// Indents the selected note: reparents it under its immediately
    /// preceding sibling (becoming that sibling's last child).
    pub fn indent_selected(&mut self) {
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let Some(previous_parent) = self.tree.get(id).map(|note| note.parent) else {
            return;
        };

        let siblings: &[NoteId] = match previous_parent {
            Some(p) => self.tree.children(p),
            None => self.tree.roots(),
        };
        let Some(pos) = siblings.iter().position(|&s| s == id) else {
            return;
        };
        let Some(&new_parent) = pos.checked_sub(1).and_then(|i| siblings.get(i)) else {
            return; // already first among siblings, nothing to indent under
        };

        self.reparent(id, Some(new_parent), previous_parent);
    }

    /// Outdents the selected note: reparents it to be a sibling of its
    /// current parent (its grandparent's children). Appended after the
    /// grandparent's current last child — not necessarily right after the
    /// former parent if it already had later siblings.
    pub fn outdent_selected(&mut self) {
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let Some(previous_parent) = self.tree.get(id).map(|note| note.parent) else {
            return;
        };
        let Some(current_parent) = previous_parent else {
            return; // already a root
        };
        let grandparent = self.tree.get(current_parent).and_then(|note| note.parent);

        self.reparent(id, grandparent, previous_parent);
    }

    fn reparent(&mut self, id: NoteId, new_parent: Option<NoteId>, previous_parent: Option<NoteId>) {
        if self.tree.move_note(id, new_parent).is_err() {
            return;
        }
        if let Some(p) = new_parent {
            self.expanded.insert(p);
        }
        self.persist(id);
        self.record(UndoAction::Move {
            id,
            parent: previous_parent,
        });
    }

    pub fn reorder_up(&mut self) {
        self.reorder(true);
    }

    pub fn reorder_down(&mut self) {
        self.reorder(false);
    }

    fn reorder(&mut self, up: bool) {
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let moved = if up {
            self.tree.move_up(id)
        } else {
            self.tree.move_down(id)
        };
        if !moved {
            return;
        }
        self.persist_siblings(id);
        self.record(UndoAction::Reorder { id, move_down: up });
    }

    fn persist_siblings(&mut self, id: NoteId) {
        let parent = self.tree.get(id).and_then(|note| note.parent);
        let siblings: Vec<NoteId> = match parent {
            Some(p) => self.tree.children(p).to_vec(),
            None => self.tree.roots().to_vec(),
        };
        for sibling_id in siblings {
            self.persist(sibling_id);
        }
    }

    /// Starts insert mode with an empty input, for naming a freshly created
    /// note. Unlike `begin_rename`, doesn't prefill the placeholder title —
    /// the user types the name outright instead of editing "New note" away.
    fn begin_naming(&mut self) {
        self.input.clear();
        self.mode = Mode::Insert;
    }

    /// Opens the delete confirmation prompt for the selected note.
    pub fn request_delete(&mut self) {
        if let Some(id) = self.selected {
            if !self.require_editable(id) {
                return;
            }
            self.pending_delete = Some(id);
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete = None;
        self.mode = Mode::Normal;
    }

    /// First `q` arms the confirmation; a second press actually quits.
    pub fn request_quit(&mut self) {
        if self.confirm_quit {
            self.should_quit = true;
        } else {
            self.confirm_quit = true;
        }
    }

    pub fn reset_quit_confirmation(&mut self) {
        self.confirm_quit = false;
    }

    /// Clears `last_error`/`last_message` — called once per keypress,
    /// before dispatch, so a status message from a previous action (e.g.
    /// `:export`'s "exported to ...") doesn't linger in the hint row
    /// forever once you've moved on to something else. Whatever the
    /// keypress itself does can still set a fresh one right afterward in
    /// the same call, overwriting this.
    pub fn clear_transient_status(&mut self) {
        self.last_error = None;
        self.last_message = None;
    }

    /// Number of descendants under the pending note, for the confirmation
    /// prompt ("delete this and its N descendants?").
    pub fn pending_delete_descendant_count(&self) -> Option<usize> {
        let id = self.pending_delete?;
        Some(self.tree.subtree_ids(id).len() - 1)
    }

    pub fn pending_delete_title(&self) -> Option<&str> {
        let id = self.pending_delete?;
        self.tree.get(id).map(|note| note.title.as_str())
    }

    /// Deletes the pending note and its whole subtree (moved to trash, not
    /// permanently erased) after the user confirmed.
    pub fn confirm_delete(&mut self) {
        let Some(id) = self.pending_delete.take() else {
            return;
        };
        self.mode = Mode::Normal;

        let next = self.neighbor_after(id);
        let Some(removed) = self.tree.delete_subtree(id) else {
            return;
        };

        for &(note_id, _) in &removed {
            self.expanded.remove(&note_id);
            if let Err(err) = self.vault.trash_note(note_id) {
                self.last_error = Some(self.lang.trash_failed(&err));
            }
        }
        self.set_selected(next);
        self.record(UndoAction::Restore { snapshot: removed });
    }

    fn neighbor_after(&self, id: NoteId) -> Option<NoteId> {
        let visible = self.visible_active_notes();
        let pos = visible.iter().position(|&(v, _)| v == id)?;
        visible
            .get(pos + 1)
            .or_else(|| pos.checked_sub(1).and_then(|p| visible.get(p)))
            .map(|&(v, _)| v)
    }

    pub fn begin_rename(&mut self) {
        if let Some(id) = self.selected {
            if !self.require_editable(id) {
                return;
            }
            self.input = self
                .tree
                .get(id)
                .map(|note| note.title.clone())
                .unwrap_or_default();
            self.mode = Mode::Insert;
        }
    }

    pub fn commit_rename(&mut self) {
        if !self.input.trim().is_empty()
            && let Some(id) = self.selected
            && let Some(previous_title) = self.tree.get(id).map(|note| note.title.clone())
        {
            self.tree.rename(id, self.input.clone());
            self.persist(id);
            self.record(UndoAction::Rename {
                id,
                title: previous_title,
            });
        }
        self.input.clear();
        self.mode = Mode::Normal;
    }

    pub fn cancel_rename(&mut self) {
        self.input.clear();
        self.mode = Mode::Normal;
    }

    fn persist(&mut self, id: NoteId) {
        let Some(note) = self.tree.get(id) else {
            return;
        };
        match self.vault.save_note(id, note) {
            Ok(()) => self.last_error = None,
            Err(err) => self.last_error = Some(self.lang.save_failed(&err)),
        }
    }

    fn record(&mut self, action: UndoAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        let Some(action) = self.undo_stack.pop() else {
            return;
        };
        if let Some(inverse) = self.apply_undo_action(action) {
            self.redo_stack.push(inverse);
        }
    }

    pub fn redo(&mut self) {
        let Some(action) = self.redo_stack.pop() else {
            return;
        };
        if let Some(inverse) = self.apply_undo_action(action) {
            self.undo_stack.push(inverse);
        }
    }

    /// Applies `action` against the *current* live tree and returns its
    /// inverse (to push onto the opposite stack), or `None` if the action
    /// no longer applies (e.g. the note was already removed by something
    /// else) — dropped silently rather than corrupting either stack.
    fn apply_undo_action(&mut self, action: UndoAction) -> Option<UndoAction> {
        match action {
            UndoAction::Rename { id, title } => {
                let previous = self.tree.get(id)?.title.clone();
                self.tree.rename(id, title);
                self.persist(id);
                self.set_selected(Some(id));
                Some(UndoAction::Rename {
                    id,
                    title: previous,
                })
            }
            UndoAction::Move { id, parent } => {
                let previous = self.tree.get(id)?.parent;
                self.tree.move_note(id, parent).ok()?;
                if let Some(p) = parent {
                    self.expanded.insert(p);
                }
                self.persist(id);
                self.set_selected(Some(id));
                Some(UndoAction::Move {
                    id,
                    parent: previous,
                })
            }
            UndoAction::Reorder { id, move_down } => {
                let moved = if move_down {
                    self.tree.move_down(id)
                } else {
                    self.tree.move_up(id)
                };
                if !moved {
                    return None;
                }
                self.persist_siblings(id);
                self.set_selected(Some(id));
                Some(UndoAction::Reorder {
                    id,
                    move_down: !move_down,
                })
            }
            UndoAction::Remove { root_id } => {
                let next = self.neighbor_after(root_id);
                let removed = self.tree.delete_subtree(root_id)?;
                for &(note_id, _) in &removed {
                    self.expanded.remove(&note_id);
                    if let Err(err) = self.vault.trash_note(note_id) {
                        self.last_error = Some(self.lang.trash_failed(&err));
                    }
                }
                self.set_selected(next);
                Some(UndoAction::Restore { snapshot: removed })
            }
            UndoAction::Restore { snapshot } => {
                let root_id = snapshot.first()?.0;
                let ids: Vec<NoteId> = snapshot.iter().map(|(id, _)| *id).collect();
                for (id, note) in snapshot {
                    self.tree.insert_loaded(id, note);
                }
                self.tree.rebuild_hierarchy();
                for id in ids {
                    self.persist(id);
                }
                self.set_selected(Some(root_id));
                Some(UndoAction::Remove { root_id })
            }
            UndoAction::EditBody { id, body } => {
                let previous = self.tree.get(id)?.body.clone();
                self.tree.set_body(id, body);
                self.persist(id);
                self.set_selected(Some(id));
                Some(UndoAction::EditBody {
                    id,
                    body: previous,
                })
            }
            UndoAction::SetTags { id, tags } => {
                let previous = self.tree.get(id)?.tags.clone();
                self.tree.set_tags(id, tags);
                self.persist(id);
                self.set_selected(Some(id));
                Some(UndoAction::SetTags {
                    id,
                    tags: previous,
                })
            }
        }
    }

    /// Enters search mode. Reindexes first so results reflect the live
    /// in-memory tree (including edits made this session that a prior
    /// `mycora reindex` run on disk wouldn't know about), not a stale
    /// copy. Every mounted vault gets reindexed (`reindex_mounted`), even
    /// though a single search only ever queries one of them (see
    /// `search_scope`) — cheap to keep them all fresh together, and
    /// avoids a second reindex if the vault being searched changes
    /// before the next search session.
    pub fn begin_search(&mut self) {
        if let Err(err) = self.reindex_mounted() {
            self.last_error = Some(self.lang.reindex_failed(&err));
        }
        self.search_query.clear();
        self.search_results.clear();
        self.search_selected = 0;
        self.mode = Mode::Search;
    }

    pub fn search_input(&mut self, c: char) {
        self.search_query.push(c);
        self.update_search_results();
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.update_search_results();
    }

    fn update_search_results(&mut self) {
        let vault_id = self.search_scope().to_string();
        self.search_results = match self.index.search(&vault_id, &self.search_query) {
            Ok(hits) => hits,
            Err(err) => {
                self.last_error = Some(self.lang.search_failed(&err));
                Vec::new()
            }
        };
        self.search_selected = 0;
    }

    pub fn move_search_selection(&mut self, delta: isize) {
        if self.search_results.is_empty() {
            return;
        }
        let len = self.search_results.len() as isize;
        let new_pos = (self.search_selected as isize + delta).rem_euclid(len) as usize;
        self.search_selected = new_pos;
    }

    /// Jumps to the selected search hit (expanding its ancestors so it's
    /// visible) and returns to normal mode.
    pub fn confirm_search(&mut self) {
        if let Some(hit) = self.search_results.get(self.search_selected) {
            let id = hit.note_id;
            self.reveal(id);
            self.set_selected(Some(id));
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_search(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Expands every ancestor of `id` so it's visible in `visible_notes()`.
    /// Direct field access rather than `self.resolve(id)`: needs a live
    /// `&Tree` reference at the same time as `&mut self.expanded`, which
    /// a `&self` method handing back borrowed data can't provide alongside
    /// a mutable borrow of a different field — same reason
    /// `reveal_ancestors` is a free function taking disjoint refs rather
    /// than a method in the first place.
    fn reveal(&mut self, id: NoteId) {
        if self.tree.get(id).is_some() {
            reveal_ancestors(&self.tree, &mut self.expanded, id);
            return;
        }
        if let Some(v) = self.other_vaults.iter().find(|v| v.tree.get(id).is_some()) {
            reveal_ancestors(&v.tree, &mut self.expanded, id);
        }
    }

    /// The vault `/` search actually queries: wherever the current
    /// selection lives (via `resolve`), so searching while browsing a
    /// read-only mounted vault searches *that* vault rather than
    /// silently falling back to the active one. Falls back to the active
    /// vault when nothing's selected, or the selection is an unmounted/
    /// archived vault's placeholder row (nothing loaded there to
    /// search). Selection can't change while `Mode::Search` is open (no
    /// tree navigation happens in that mode), so this stays stable for
    /// as long as a search session lasts.
    pub fn search_scope(&self) -> &str {
        self.selected
            .and_then(|id| self.resolve(id))
            .map(|(_, vault_id)| vault_id)
            .unwrap_or(self.vault_id.as_str())
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn search_results(&self) -> &[SearchHit] {
        &self.search_results
    }

    pub fn search_selected(&self) -> usize {
        self.search_selected
    }

    /// Shifts keyboard focus onto the backlinks pane already visible in the
    /// split layout, rather than opening a separate overlay — `j`/`k`
    /// (or `Up`/`Down`) move within it, `Enter` jumps, `Esc` or `b` again
    /// returns focus to the tree. Doesn't reindex first: it reads
    /// `live_backlinks()` exactly like the passive pane already did before
    /// this had its own focus state, so results reflect whatever the last
    /// reindex resolved rather than forcing a fresh one on every `b` press.
    pub fn focus_backlinks(&mut self) {
        if self.selected.is_none() {
            return;
        }
        self.backlinks_selected = 0;
        self.mode = Mode::Backlinks;
    }

    pub fn move_backlinks_selection(&mut self, delta: isize) {
        let len = self.live_backlinks().len();
        if len == 0 {
            return;
        }
        let len = len as isize;
        let new_pos = (self.backlinks_selected as isize + delta).rem_euclid(len) as usize;
        self.backlinks_selected = new_pos;
    }

    /// Jumps to the focused backlink (expanding its ancestors so it's
    /// visible) and returns focus to the tree.
    pub fn confirm_backlinks(&mut self) {
        if let Some(hit) = self.live_backlinks().get(self.backlinks_selected) {
            let id = hit.note_id;
            self.reveal(id);
            self.set_selected(Some(id));
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_backlinks(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn backlinks_selected(&self) -> usize {
        self.backlinks_selected
    }

    /// Reindexes the primary vault together with every read-only mounted
    /// vault, so cross-vault wikilinks stay resolved against each other —
    /// same reasoning as `App::new()`'s startup reindex. Called from `/`
    /// and `b`, the only two places that need fresh results mid-session.
    /// Errors fold into `last_error` rather than propagating, since these
    /// are UI-triggered entry points that can't fail the whole app.
    /// Returns the total note count across every mounted vault on success,
    /// for callers (`:reindex`) that want to report it — `begin_search`
    /// just ignores it.
    fn reindex_mounted(&mut self) -> anyhow::Result<usize> {
        let mut batch: Vec<(&str, &Tree, &Vault)> =
            Vec::with_capacity(1 + self.other_vaults.len());
        batch.push((self.vault_id.as_str(), &self.tree, &self.vault));
        for v in &self.other_vaults {
            batch.push((v.id.as_str(), &v.tree, &v.vault));
        }
        let reports = self.index.reindex_mounted(&batch)?;
        Ok(reports.iter().map(|r| r.note_count).sum())
    }

    /// Notes linking to the currently selected note — the split layout's
    /// passive backlinks pane. Best-effort and read-only like
    /// `link_count_for`: doesn't reindex first, just reflects whatever the
    /// last reindex resolved, since this also runs during rendering on an
    /// immutable `&App`. The interactive `b` overlay (`Mode::Backlinks`)
    /// is still the way to actually jump to one of these.
    pub fn live_backlinks(&self) -> Vec<IndexedNote> {
        let Some(id) = self.selected else {
            return Vec::new();
        };
        let Some((_, vault_id)) = self.resolve(id) else {
            return Vec::new();
        };
        self.index.backlinks(vault_id, id).unwrap_or_default()
    }

    /// The selected note, wherever it lives — the active vault or a
    /// read-only mounted one. `ui.rs`'s body preview uses this instead of
    /// reaching into the active `tree` directly, so a read-only note's
    /// body is actually readable (the whole point of read-only vaults
    /// being navigable at all).
    pub fn selected_note(&self) -> Option<&Note> {
        let id = self.selected?;
        self.resolve(id).and_then(|(tree, _)| tree.get(id))
    }

    /// The registry name of whichever vault `selected` is actually in —
    /// the first segment of the status bar's breadcrumb. Falls back to
    /// the active vault's name when nothing's selected. This is what
    /// makes the breadcrumb honestly show, e.g., `archive › Some Note`
    /// rather than always claiming `default` while browsing a read-only
    /// vault.
    pub fn vault_name(&self) -> &str {
        if let Some(name) = &self.selected_unmounted_vault {
            return name;
        }
        if let Some(name) = &self.selected_archived_vault {
            return name;
        }
        self.selected
            .and_then(|id| self.resolve(id))
            .map(|(_, vault_id)| vault_id)
            .unwrap_or(self.vault_id.as_str())
    }

    /// `true` if the current selection is in a read-only mounted vault —
    /// drives the breadcrumb row's "READ-ONLY" marker in `ui.rs`.
    pub fn selected_is_read_only(&self) -> bool {
        self.selected.is_some_and(|id| self.tree.get(id).is_none())
    }

    /// `true` if the current row is an unmounted vault's placeholder
    /// rather than a note — drives the breadcrumb row's "UNMOUNTED"
    /// marker, the hint row's full mutation lockout (nothing is loaded to
    /// act on), and `draw_body_preview`'s "how to mount" message.
    pub fn selected_is_unmounted_vault(&self) -> bool {
        self.selected_unmounted_vault.is_some()
    }

    /// `(name, path)` of the currently selected row's unmounted vault, if
    /// that's what's selected — the whole reason the row exists is to
    /// tell the user how to bring it back, so the body preview needs
    /// both the display name and the exact path to put in that message.
    pub fn selected_unmounted_vault_info(&self) -> Option<(&str, &Path)> {
        let name = self.selected_unmounted_vault.as_deref()?;
        self.unmounted_vaults
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| (entry.name.as_str(), entry.path.as_path()))
    }

    /// `true` if the current row is an archived vault's placeholder
    /// rather than a note — drives the breadcrumb row's "ARCHIVED"
    /// marker, the hint row's full mutation lockout, and
    /// `draw_body_preview`'s "how to unarchive" message.
    pub fn selected_is_archived_vault(&self) -> bool {
        self.selected_archived_vault.is_some()
    }

    /// `(name, archive_path)` of the currently selected row's archived
    /// vault, if that's what's selected — the body preview's "how to
    /// unarchive" message needs both.
    pub fn selected_archived_vault_info(&self) -> Option<(&str, &Path)> {
        let name = self.selected_archived_vault.as_deref()?;
        self.archived_vaults
            .iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| entry.archived.as_deref())
            .map(|archive_path| (name, archive_path))
    }

    /// Current percent widths of the split layout's tree/body/backlinks
    /// columns, always summing to 100.
    pub fn pane_widths(&self) -> [u16; 3] {
        self.pane_widths
    }

    /// Floor no pane may shrink past.
    const PANE_MIN_PCT: u16 = 10;
    /// How much one `[`/`]`/`{`/`}` press adjusts a pane by.
    const PANE_STEP_PCT: u16 = 5;
    /// Starting layout, and what `:panes reset` restores.
    const DEFAULT_PANE_WIDTHS: [u16; 3] = [40, 40, 20];

    /// Grows or shrinks `pane_widths[target]` (0 = tree, 2 = backlinks) by
    /// `PANE_STEP_PCT`, transferring the difference to/from the body pane
    /// (index 1) — body is never resized directly, it just absorbs
    /// whatever the other two give up or take. No-op if either pane would
    /// drop below `PANE_MIN_PCT`.
    fn resize_pane(&mut self, target: usize, grow: bool) {
        let step = Self::PANE_STEP_PCT as i32 * if grow { 1 } else { -1 };
        let new_target = self.pane_widths[target] as i32 + step;
        let new_body = self.pane_widths[1] as i32 - step;
        if new_target < Self::PANE_MIN_PCT as i32 || new_body < Self::PANE_MIN_PCT as i32 {
            return;
        }
        self.pane_widths[target] = new_target as u16;
        self.pane_widths[1] = new_body as u16;
    }

    /// `[` — shrinks the tree pane, giving the width to the body pane.
    pub fn shrink_tree_pane(&mut self) {
        self.resize_pane(0, false);
    }

    /// `]` — grows the tree pane, taking the width from the body pane.
    pub fn grow_tree_pane(&mut self) {
        self.resize_pane(0, true);
    }

    /// `{` — shrinks the backlinks pane, giving the width to the body pane.
    pub fn shrink_backlinks_pane(&mut self) {
        self.resize_pane(2, false);
    }

    /// `}` — grows the backlinks pane, taking the width from the body pane.
    pub fn grow_backlinks_pane(&mut self) {
        self.resize_pane(2, true);
    }

    /// Rows a `Ctrl+d`/`Ctrl+u` half-page scroll moves the body preview
    /// by. Fixed rather than computed from the pane's actual rendered
    /// height, which isn't threaded into `App` — large enough to feel
    /// like progress, small enough not to skip past short sections.
    const BODY_SCROLL_STEP: u16 = 10;

    /// `Ctrl+d` — scrolls the body preview down. Deliberately unclamped
    /// at the top end: computing the true max would mean duplicating
    /// `markdown.rs`'s render+wrap logic here just to count lines, so
    /// scrolling past the end just shows blank space and recovers with
    /// `Ctrl+u` — the same way plenty of simple pagers behave without
    /// tracking exact content height.
    pub fn scroll_body_down(&mut self) {
        self.body_scroll = self.body_scroll.saturating_add(Self::BODY_SCROLL_STEP);
    }

    /// `Ctrl+u` — scrolls the body preview up, floored at the top.
    pub fn scroll_body_up(&mut self) {
        self.body_scroll = self.body_scroll.saturating_sub(Self::BODY_SCROLL_STEP);
    }

    pub fn body_scroll(&self) -> u16 {
        self.body_scroll
    }

    /// Ancestor titles from the selected note's root down to itself
    /// (inclusive) — the rest of the status bar's breadcrumb. Empty when
    /// nothing's selected.
    pub fn breadcrumb_titles(&self) -> Vec<String> {
        let Some(id) = self.selected else {
            return Vec::new();
        };
        let Some((tree, _)) = self.resolve(id) else {
            return Vec::new();
        };
        let mut titles = Vec::new();
        let mut current = Some(id);
        while let Some(cur_id) = current {
            let Some(note) = tree.get(cur_id) else { break };
            titles.push(note.title.clone());
            current = note.parent;
        }
        titles.reverse();
        titles
    }


    /// Opens the selected note's body for editing. No-op if nothing's
    /// selected (there's no note to edit) — mirrors `begin_rename`'s guard.
    pub fn begin_edit_body(&mut self) {
        let Some(id) = self.selected else { return };
        if !self.require_editable(id) {
            return;
        }
        let Some(note) = self.tree.get(id) else { return };
        let lines: Vec<String> = if note.body.is_empty() {
            vec![String::new()]
        } else {
            note.body.lines().map(String::from).collect()
        };
        let mut editor = TextArea::new(lines);
        editor.set_block(Block::default().borders(Borders::ALL).title(note.title.clone()));
        self.body_editor = Some(editor);
        self.mode = Mode::EditBody;
    }

    /// Forwards one key event into the active body editor. No-op outside
    /// `Mode::EditBody` (nothing to forward into).
    pub fn body_editor_input(&mut self, key: crossterm::event::KeyEvent) {
        if let Some(editor) = &mut self.body_editor {
            editor.input(key);
        }
    }

    /// Writes the editor's current text back to the note being edited and
    /// returns to Normal mode. Persist-on-exit: there's no per-keystroke
    /// write-through here (unlike title edits) since a body can be large
    /// enough that writing on every keystroke would be wasteful. A no-op
    /// edit (body unchanged) skips both the disk write and the undo entry.
    pub fn save_and_exit_body_edit(&mut self) {
        self.mode = Mode::Normal;
        let Some(editor) = self.body_editor.take() else {
            return;
        };
        let Some(id) = self.selected else { return };
        let Some(previous_body) = self.tree.get(id).map(|note| note.body.clone()) else {
            return;
        };

        let new_body = editor.lines().join("\n");
        if new_body == previous_body {
            return;
        }
        self.tree.set_body(id, new_body);
        self.persist(id);
        self.record(UndoAction::EditBody {
            id,
            body: previous_body,
        });
    }

    pub fn body_editor(&self) -> Option<&TextArea<'static>> {
        self.body_editor.as_ref()
    }

    /// `:` — opens the command prompt (see `Mode::Command`'s doc comment).
    pub fn begin_command(&mut self) {
        self.command_input.clear();
        self.mode = Mode::Command;
    }

    pub fn command_input(&self) -> &str {
        &self.command_input
    }

    pub fn command_input_push(&mut self, c: char) {
        self.command_input.push(c);
    }

    pub fn command_input_backspace(&mut self) {
        self.command_input.pop();
    }

    pub fn cancel_command(&mut self) {
        self.command_input.clear();
        self.mode = Mode::Normal;
    }

    /// Parses and runs the typed command. Unknown commands, and commands
    /// given the wrong shape of argument, report through `last_error`
    /// rather than doing nothing silently. The command set is
    /// deliberately small for now — it exists mainly to give a few
    /// backend-only features (tag filtering, manual reindex) a way into
    /// the TUI without inventing a dedicated keybinding for each:
    ///
    /// - `q` / `quit` — same as `q` `q` in Normal mode, no confirmation
    ///   (there's nothing unsaved to protect against — Mycora always
    ///   writes through immediately)
    /// - `reindex` — rebuilds the index for every mounted vault, same as
    ///   `mycora reindex` from the CLI but without leaving the TUI
    /// - `tags <tag1,tag2,...>` — notes matching *any* of the given tags
    ///   (`TagFilterOp::Any`) across every mounted vault (see
    ///   `tags_scope`), each hit labeled with its own; opens
    ///   `Mode::TagResults` if there are hits
    /// - `tags list` — every distinct tag across every mounted vault,
    ///   note counts summed across all of them, in `Mode::TagList`;
    ///   `Enter` on one filters by it (same as typing `:tags <that-tag>`),
    ///   so you don't need to already know or type its exact spelling
    /// - `tags limit <vault-name>` / `tags unlimit` — narrows
    ///   `tags`/`tags list` to one named mounted vault instead of
    ///   spanning all of them, until lifted. Errors if `<vault-name>`
    ///   isn't currently mounted. Deliberately *not* persisted in
    ///   `Session` — a temporary working focus, not a display preference
    /// - `panes reset` — resets the split layout to `DEFAULT_PANE_WIDTHS`;
    ///   the only way back to it now that widths persist across restarts,
    ///   short of hand-editing or deleting `session.toml`
    /// - `export <path>` — flattens the selected note's subtree (see
    ///   `export::flatten_subtree`) to a Markdown file at `path`. A read
    ///   operation, not gated by `require_editable` — works on a
    ///   read-only mounted vault's note just as well as the active
    ///   vault's. Refuses if `path` already exists rather than
    ///   overwriting it.
    /// - `config unmount <show|hide>` / `config archive <show|hide>` —
    ///   toggles whether `TreeRow::UnmountedVault`/`TreeRow::ArchivedVault`
    ///   placeholder rows render in the tree at all, for decluttering a
    ///   registry with several of either. Persisted in `Session`
    ///   (`show_unmounted`/`show_archived`), not per-vault — a display
    ///   preference, same as `pane_widths`.
    /// - `tag add <tag>` / `tag del <tag>` — adds/removes a tag on the
    ///   selected note. Gated by `require_editable`; a no-op reported
    ///   via `last_message` (not an error) when the tag is already
    ///   there (`add`) or already gone (`del`).
    /// - `lang <en|fr|es|de>` — switches the interface language immediately
    ///   (every string reads `self.lang` live, so the very next frame
    ///   renders in the new language — no refresh mechanism needed) and
    ///   persists it to `config.toml`. Bare `lang` reports the current
    ///   one. See `command_lang` for the half-applied case.
    ///
    /// Kept in sync with `Lang::command_reference` (which `ui.rs`'s help
    /// popup renders, in the configured language) by hand — not worth
    /// generating one from the other at this size.
    pub fn execute_command(&mut self) {
        let input = std::mem::take(&mut self.command_input);
        self.mode = Mode::Normal;

        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }
        let (name, args) = match trimmed.split_once(char::is_whitespace) {
            Some((name, args)) => (name, args.trim()),
            None => (trimmed, ""),
        };

        match name {
            "q" | "quit" => self.should_quit = true,
            "reindex" => self.command_reindex(),
            "tags" => self.command_tags(args),
            "panes" => self.command_panes(args),
            "export" => self.command_export(args),
            "config" => self.command_config(args),
            "tag" => self.command_tag(args),
            "lang" => self.command_lang(args),
            _ => {
                self.last_message = None;
                self.last_error = Some(self.lang.unknown_command(name));
            }
        }
    }

    fn command_reindex(&mut self) {
        match self.reindex_mounted() {
            Ok(count) => {
                self.last_error = None;
                self.last_message = Some(self.lang.reindexed_notes(count));
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(self.lang.reindex_failed(&err));
            }
        }
    }

    /// `:tags tag1,tag2` — notes matching any of the given tags. Opens
    /// `Mode::TagResults` on a hit, otherwise reports through
    /// `last_message`/`last_error` instead. AND semantics (every tag
    /// required) and a keybinding for either aren't exposed yet — this is
    /// the first, simplest entry point for `Index::filter_by_tags`, which
    /// has had no TUI surface at all since v0.4.
    /// `:tags list` shows every known tag; `:tags <tag1,tag2,...>` filters
    /// by them (OR). The literal argument `"list"` is checked first, so a
    /// tag actually named "list" would need `:tags list,list` or similar
    /// to reach via this command — the same minor, accepted trade-off as
    /// `:panes reset`'s literal-argument dispatch.
    fn command_tags(&mut self, args: &str) {
        let trimmed = args.trim();
        if trimmed == "list" {
            self.command_tags_list();
            return;
        }
        if trimmed == "unlimit" {
            self.command_tags_unlimit();
            return;
        }
        if trimmed == "limit" {
            self.last_message = None;
            self.last_error = Some(self.lang.tags_limit_usage().to_string());
            return;
        }
        // Same "literal first-argument" dispatch as "list"/"unlimit"
        // above — a tag actually named "limit ..." needs a comma to
        // reach via filtering instead, same accepted edge case as a tag
        // literally named "list".
        if let Some(name) = trimmed.strip_prefix("limit ") {
            self.command_tags_limit(name.trim());
            return;
        }

        let tags: Vec<String> = args
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect();
        if tags.is_empty() {
            self.last_message = None;
            self.last_error = Some(self.lang.tags_usage().to_string());
            return;
        }

        self.show_tag_results(tags);
    }

    /// `:tags limit <name>`. Errors if `name` isn't a currently mounted
    /// vault — same "don't silently guess" instinct as `vault mount`
    /// refusing an unknown name — rather than silently limiting to
    /// nothing and reporting "no tags" as if that vault existed.
    fn command_tags_limit(&mut self, name: &str) {
        if name.is_empty() {
            self.last_message = None;
            self.last_error = Some(self.lang.tags_limit_usage().to_string());
            return;
        }
        if !self.mounted_vault_ids().contains(&name) {
            self.last_message = None;
            self.last_error = Some(self.lang.no_mounted_vault_named(name));
            return;
        }
        self.tags_limit = Some(name.to_string());
        self.last_error = None;
        self.last_message = Some(self.lang.tags_limited_to(name));
    }

    /// `:tags unlimit`. A no-op message (not an error) if nothing was
    /// limited, same "redundant, not wrong" instinct as `vault mount` on
    /// an already-mounted vault.
    fn command_tags_unlimit(&mut self) {
        if self.tags_limit.take().is_none() {
            self.last_error = None;
            self.last_message = Some(self.lang.tags_were_not_limited().to_string());
            return;
        }
        self.last_error = None;
        self.last_message = Some(self.lang.tags_no_longer_limited().to_string());
    }

    fn command_tags_list(&mut self) {
        match self.index.all_tags(&self.tags_scope()) {
            Ok(tags) if tags.is_empty() => {
                self.last_error = None;
                self.last_message = Some(match &self.tags_limit {
                    Some(name) => self.lang.no_tags_in(name),
                    None => self.lang.no_tags_anywhere().to_string(),
                });
            }
            Ok(tags) => {
                self.last_error = None;
                self.last_message = None;
                self.tag_list = tags;
                self.tag_list_selected = 0;
                self.mode = Mode::TagList;
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(self.lang.tag_list_failed(&err));
            }
        }
    }

    /// Filters by `tags` (OR) and opens `Mode::TagResults` on a match —
    /// shared by `:tags <tag1,tag2,...>` and `confirm_tag_list` (picking a
    /// tag from `:tags list` runs this with just that one tag).
    fn show_tag_results(&mut self, tags: Vec<String>) {
        match self
            .index
            .filter_by_tags(&self.tags_scope(), &tags, TagFilterOp::Any)
        {
            Ok(hits) if hits.is_empty() => {
                self.last_error = None;
                let joined = tags.join(", ");
                self.last_message = Some(match &self.tags_limit {
                    Some(name) => self.lang.no_notes_tagged_in(&joined, name),
                    None => self.lang.no_notes_tagged_anywhere(&joined),
                });
            }
            Ok(hits) => {
                self.last_error = None;
                self.last_message = None;
                self.tag_results = hits;
                self.tag_results_selected = 0;
                self.mode = Mode::TagResults;
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(self.lang.tag_filter_failed(&err));
            }
        }
    }

    fn command_panes(&mut self, args: &str) {
        if args.trim() != "reset" {
            self.last_message = None;
            self.last_error = Some(self.lang.panes_usage().to_string());
            return;
        }
        self.pane_widths = Self::DEFAULT_PANE_WIDTHS;
        self.last_error = None;
        self.last_message = Some(self.lang.panes_reset_done().to_string());
    }

    /// `:config unmount <show|hide>` / `:config archive <show|hide>` —
    /// see `execute_command`'s doc comment.
    fn command_config(&mut self, args: &str) {
        let (category, state) = match args.split_whitespace().collect::<Vec<_>>().as_slice() {
            [category, state] => (*category, *state),
            _ => {
                self.last_message = None;
                self.last_error = Some(self.lang.config_usage().to_string());
                return;
            }
        };
        let show = match state {
            "show" => true,
            "hide" => false,
            _ => {
                self.last_message = None;
                self.last_error = Some(self.lang.config_usage().to_string());
                return;
            }
        };
        let unmounted = match category {
            "unmount" => {
                self.show_unmounted = show;
                true
            }
            "archive" => {
                self.show_archived = show;
                false
            }
            _ => {
                self.last_message = None;
                self.last_error = Some(self.lang.config_usage().to_string());
                return;
            }
        };

        // Hiding a category the current selection was in leaves it
        // pointing at a row that no longer renders — fall back to the
        // active vault's first root (always at least one, see `App::new`'s
        // "Welcome to Mycora" auto-creation) rather than a selection
        // nothing on screen corresponds to.
        let selection_still_visible = match (
            self.selected_unmounted_vault.is_some(),
            self.selected_archived_vault.is_some(),
        ) {
            (true, _) => self.show_unmounted,
            (_, true) => self.show_archived,
            (false, false) => true,
        };
        if !selection_still_visible {
            self.set_selected(self.tree.roots().first().copied());
        }

        self.last_error = None;
        self.last_message = Some(self.lang.config_vaults_visibility(unmounted, show));
    }

    /// `:tag add <tag>` / `:tag del <tag>` — mutates the selected note's
    /// tags. Gated by `require_editable` like every other mutating
    /// command; a no-op (reported via `last_message`, not `last_error`)
    /// rather than a hard error when adding a tag that's already there
    /// or removing one that isn't — redundant, not wrong, same instinct
    /// as `vault mount` on an already-mounted vault. Appends/removes in
    /// place rather than re-sorting the whole list, so a deliberately
    /// ordered tag list in frontmatter isn't silently reshuffled by an
    /// unrelated add/del elsewhere in it.
    fn command_tag(&mut self, args: &str) {
        let (action, tag) = match args.split_once(char::is_whitespace) {
            Some((action, tag)) => (action, tag.trim()),
            None => {
                self.last_message = None;
                self.last_error = Some(self.lang.tag_usage().to_string());
                return;
            }
        };
        if tag.is_empty() || !matches!(action, "add" | "del") {
            self.last_message = None;
            self.last_error = Some(self.lang.tag_usage().to_string());
            return;
        }

        let Some(id) = self.selected else {
            self.last_message = None;
            self.last_error = Some(self.lang.nothing_selected_to_tag().to_string());
            return;
        };
        if !self.require_editable(id) {
            return;
        }
        let Some(note) = self.tree.get(id) else {
            return;
        };
        let previous = note.tags.clone();
        let already_has = previous.iter().any(|t| t == tag);

        let new_tags = if action == "add" {
            if already_has {
                self.last_error = None;
                self.last_message = Some(self.lang.already_tagged(tag));
                return;
            }
            let mut tags = previous.clone();
            tags.push(tag.to_string());
            tags
        } else {
            if !already_has {
                self.last_error = None;
                self.last_message = Some(self.lang.not_tagged(tag));
                return;
            }
            previous.iter().filter(|t| t.as_str() != tag).cloned().collect()
        };

        self.tree.set_tags(id, new_tags);
        self.persist(id);
        self.record(UndoAction::SetTags { id, tags: previous });
        self.last_error = None;
        self.last_message = Some(if action == "add" {
            self.lang.tag_added(tag)
        } else {
            self.lang.tag_removed(tag)
        });
    }

    /// `:lang <en|fr|es|de>` — switches `self.lang` in place (the whole UI
    /// re-renders from it on the next frame) and writes the choice
    /// through to `config.toml` so it survives restarts (confirmed with
    /// the user: a language is a durable preference, unlike `:tags
    /// limit`'s per-session focus). The in-memory switch is applied
    /// *before* the write, and kept even if the write fails — the
    /// failure message says exactly that ("switched for this session,
    /// but saving failed"), in the new language, rather than pretending
    /// nothing happened. Bare `:lang` just reports the current language.
    /// Switching to the language already active still rewrites the file
    /// — harmless, and it doubles as a way to materialize the key into a
    /// config that never had one.
    fn command_lang(&mut self, args: &str) {
        let code = args.trim();
        if code.is_empty() {
            self.last_error = None;
            self.last_message = Some(self.lang.language_now().to_string());
            return;
        }
        let Some(new_lang) = Lang::from_code(code) else {
            self.last_message = None;
            self.last_error = Some(self.lang.lang_usage().to_string());
            return;
        };

        self.lang = new_lang;
        match Config::set_language(&self.config_path, new_lang.code()) {
            Ok(()) => {
                self.last_error = None;
                self.last_message = Some(self.lang.language_now().to_string());
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(self.lang.language_save_failed(&err));
            }
        }
    }

    fn command_export(&mut self, args: &str) {
        let path_str = args.trim();
        if path_str.is_empty() {
            self.last_message = None;
            self.last_error = Some(self.lang.export_usage().to_string());
            return;
        }
        let Some(id) = self.selected else {
            self.last_message = None;
            self.last_error = Some(self.lang.nothing_selected_to_export().to_string());
            return;
        };

        let path = std::path::Path::new(path_str);
        if path.exists() {
            self.last_message = None;
            self.last_error = Some(self.lang.already_exists(path_str));
            return;
        }

        // `resolve`'s borrow only needs to live long enough to compute
        // `content` (an owned String) — done before `path.exists()`'s
        // check above so no borrow of `self` is still alive by the time
        // `last_error`/`last_message` get written to below.
        let content = match self.resolve(id) {
            Some((tree, _)) => crate::export::flatten_subtree(tree, id),
            None => {
                self.last_message = None;
                self.last_error = Some(self.lang.nothing_selected_to_export().to_string());
                return;
            }
        };

        match crate::export::write_output(&content, path) {
            Ok(()) => {
                self.last_error = None;
                self.last_message = Some(self.lang.exported_to(path_str));
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(self.lang.export_failed(&err));
            }
        }
    }

    pub fn move_tag_results_selection(&mut self, delta: isize) {
        if self.tag_results.is_empty() {
            return;
        }
        let len = self.tag_results.len() as isize;
        let new_pos = (self.tag_results_selected as isize + delta).rem_euclid(len) as usize;
        self.tag_results_selected = new_pos;
    }

    /// Jumps to the selected tag-filter result (expanding its ancestors so
    /// it's visible) and returns to Normal mode.
    pub fn confirm_tag_results(&mut self) {
        if let Some(hit) = self.tag_results.get(self.tag_results_selected) {
            let id = hit.note_id;
            self.reveal(id);
            self.set_selected(Some(id));
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_tag_results(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn tag_results(&self) -> &[IndexedNote] {
        &self.tag_results
    }

    pub fn tag_results_selected(&self) -> usize {
        self.tag_results_selected
    }

    pub fn move_tag_list_selection(&mut self, delta: isize) {
        if self.tag_list.is_empty() {
            return;
        }
        let len = self.tag_list.len() as isize;
        let new_pos = (self.tag_list_selected as isize + delta).rem_euclid(len) as usize;
        self.tag_list_selected = new_pos;
    }

    /// Filters by the focused tag — same as typing `:tags <that-tag>`
    /// yourself, transitioning straight into `Mode::TagResults`.
    pub fn confirm_tag_list(&mut self) {
        if let Some((tag, _)) = self.tag_list.get(self.tag_list_selected) {
            let tag = tag.clone();
            self.show_tag_results(vec![tag]);
        } else {
            self.mode = Mode::Normal;
        }
    }

    pub fn cancel_tag_list(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn tag_list(&self) -> &[(String, i64)] {
        &self.tag_list
    }

    pub fn tag_list_selected(&self) -> usize {
        self.tag_list_selected
    }
}
