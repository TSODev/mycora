use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;

use crate::config::Config;
use crate::index::{Index, IndexedNote, SearchHit, TagFilterOp};
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
}

pub struct App {
    pub tree: Tree,
    pub vault: Vault,
    pub expanded: HashSet<NoteId>,
    pub selected: Option<NoteId>,
    pub mode: Mode,
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
}

/// `(syntax, description)` pairs for every command `execute_command`
/// recognizes — rendered by `ui.rs`'s command-palette help popup, shown
/// automatically for the duration of `Mode::Command` so the available
/// commands are discoverable without leaving the prompt to look them up.
pub const COMMAND_REFERENCE: &[(&str, &str)] = &[
    (":reindex", "rebuild the search index"),
    (
        ":tags <tag1,tag2,...>",
        "list notes matching any of the given tags",
    ),
    (":panes reset", "reset pane widths to the default 40/40/20"),
    (":q, :quit", "quit Mycora"),
];

impl App {
    /// Loads config + vault from disk and returns the ready-to-run app along
    /// with any load warnings (malformed files, orphaned/duplicate ids) for
    /// the caller to print before the TUI takes over the terminal.
    pub fn new() -> anyhow::Result<(Self, Vec<String>)> {
        let config = Config::load()?;
        let active = config.active_vault().clone();

        // Load every mounted vault (primary included) before indexing any
        // of them — cross-vault wikilink resolution needs every vault's
        // notes visible to the index together, not one at a time (see
        // `Index::reindex_mounted`'s doc comment).
        let mut loaded: Vec<(String, Tree, Vault)> = Vec::new();
        let mut warnings = Vec::new();
        for entry in config.mounted_vaults() {
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
            .expect("active_vault()'s name always exists among mounted_vaults()");
        let (_, mut tree, mut vault) = loaded.remove(primary_idx);

        let mut selected = if tree.roots().is_empty() {
            let welcome = tree.create_note("Welcome to Mycora", None);
            tree.set_body(
                welcome,
                "a: child  o: sibling  i: rename  y: copy  d: delete  u: undo  q: quit",
            );
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
            body_editor: None,
            session_path,
            pane_widths,
            command_input: String::new(),
            tag_results: Vec::new(),
            tag_results_selected: 0,
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
        )
    }

    /// Depth-first (id, depth) pairs for notes currently visible, respecting
    /// collapse state. Recomputed on demand rather than cached: fine at
    /// in-memory, single-vault scale (see ROADMAP v0.1).
    pub fn visible_notes(&self) -> Vec<(NoteId, usize)> {
        let mut out = Vec::new();
        for &root in self.tree.roots() {
            self.push_visible(root, 0, &mut out);
        }
        out
    }

    fn push_visible(&self, id: NoteId, depth: usize, out: &mut Vec<(NoteId, usize)>) {
        out.push((id, depth));
        if self.expanded.contains(&id) {
            for &child in self.tree.children(id) {
                self.push_visible(child, depth + 1, out);
            }
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_notes();
        if visible.is_empty() {
            self.selected = None;
            return;
        }

        let current_pos = self
            .selected
            .and_then(|id| visible.iter().position(|&(v, _)| v == id))
            .unwrap_or(0);

        let len = visible.len() as isize;
        let new_pos = (current_pos as isize + delta).rem_euclid(len) as usize;
        self.selected = Some(visible[new_pos].0);
    }

    pub fn toggle_expand(&mut self) {
        if let Some(id) = self.selected {
            if self.tree.children(id).is_empty() {
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
        let parent = self
            .selected
            .and_then(|id| self.tree.get(id))
            .and_then(|note| note.parent);
        let new_id = self.tree.create_note("New note", parent);
        if let Some(parent) = parent {
            self.expanded.insert(parent);
        }
        self.selected = Some(new_id);
        self.persist(new_id);
        self.record(UndoAction::Remove { root_id: new_id });
        self.begin_naming();
    }

    pub fn create_child(&mut self) {
        if let Some(parent) = self.selected {
            let new_id = self.tree.create_note("New note", Some(parent));
            self.expanded.insert(parent);
            self.selected = Some(new_id);
            self.persist(new_id);
            self.record(UndoAction::Remove { root_id: new_id });
            self.begin_naming();
        }
    }

    /// Deep-copies the selected note (and its subtree) as a new sibling
    /// right after it. Undoing removes the whole copy in one step.
    pub fn copy_selected(&mut self) {
        let Some(id) = self.selected else { return };
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
        self.selected = Some(new_root);
        self.record(UndoAction::Remove {
            root_id: new_root,
        });
    }

    /// Indents the selected note: reparents it under its immediately
    /// preceding sibling (becoming that sibling's last child).
    pub fn indent_selected(&mut self) {
        let Some(id) = self.selected else { return };
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
                self.last_error = Some(format!("trash failed: {err}"));
            }
        }
        self.selected = next;
        self.record(UndoAction::Restore { snapshot: removed });
    }

    fn neighbor_after(&self, id: NoteId) -> Option<NoteId> {
        let visible = self.visible_notes();
        let pos = visible.iter().position(|&(v, _)| v == id)?;
        visible
            .get(pos + 1)
            .or_else(|| pos.checked_sub(1).and_then(|p| visible.get(p)))
            .map(|&(v, _)| v)
    }

    pub fn begin_rename(&mut self) {
        if let Some(id) = self.selected {
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
            Err(err) => self.last_error = Some(format!("save failed: {err}")),
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
                self.selected = Some(id);
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
                self.selected = Some(id);
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
                self.selected = Some(id);
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
                        self.last_error = Some(format!("trash failed: {err}"));
                    }
                }
                self.selected = next;
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
                self.selected = Some(root_id);
                Some(UndoAction::Remove { root_id })
            }
            UndoAction::EditBody { id, body } => {
                let previous = self.tree.get(id)?.body.clone();
                self.tree.set_body(id, body);
                self.persist(id);
                self.selected = Some(id);
                Some(UndoAction::EditBody {
                    id,
                    body: previous,
                })
            }
        }
    }

    /// Enters search mode. Reindexes first so results reflect the live
    /// in-memory tree (including edits made this session that a prior
    /// `mycora reindex` run on disk wouldn't know about), not a stale copy.
    pub fn begin_search(&mut self) {
        if let Err(err) = self.reindex_mounted() {
            self.last_error = Some(format!("reindex failed: {err}"));
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
        self.search_results = match self.index.search(&self.vault_id, &self.search_query) {
            Ok(hits) => hits,
            Err(err) => {
                self.last_error = Some(format!("search failed: {err}"));
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
            self.selected = Some(id);
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_search(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Expands every ancestor of `id` so it's visible in `visible_notes()`.
    fn reveal(&mut self, id: NoteId) {
        reveal_ancestors(&self.tree, &mut self.expanded, id);
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
            self.selected = Some(id);
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
        self.index.backlinks(&self.vault_id, id).unwrap_or_default()
    }

    /// The active (editable) vault's registry name — the first segment of
    /// the status bar's breadcrumb.
    pub fn vault_name(&self) -> &str {
        &self.vault_id
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

    /// Ancestor titles from the selected note's root down to itself
    /// (inclusive) — the rest of the status bar's breadcrumb. Empty when
    /// nothing's selected.
    pub fn breadcrumb_titles(&self) -> Vec<String> {
        let Some(mut id) = self.selected else {
            return Vec::new();
        };
        let mut titles = Vec::new();
        while let Some(note) = self.tree.get(id) {
            titles.push(note.title.clone());
            match note.parent {
                Some(parent_id) => id = parent_id,
                None => break,
            }
        }
        titles.reverse();
        titles
    }

    /// Total links touching `id`'s subtree (itself + all descendants) — the
    /// aggregate badge shown on a collapsed branch. Best-effort: an index
    /// error just reports 0 rather than surfacing as `last_error`, since
    /// this runs during rendering on an immutable `&App` and a badge
    /// miscount isn't worth interrupting the user over.
    pub fn link_count_for(&self, id: NoteId) -> i64 {
        let subtree = self.tree.subtree_ids(id);
        self.index
            .link_count_for_subtree(&self.vault_id, &subtree)
            .unwrap_or(0)
    }

    /// One `(vault name, root notes)` entry per other mounted vault, for
    /// the read-only sections `ui.rs` renders below the primary tree. Each
    /// root's link count is its own subtree's badge, same as the primary
    /// tree's (best-effort: an index error just reports 0).
    pub fn other_vault_sections(&self) -> Vec<(&str, Vec<(&str, i64)>)> {
        self.other_vaults
            .iter()
            .map(|v| {
                let roots = v
                    .tree
                    .roots()
                    .iter()
                    .map(|&id| {
                        let note = v
                            .tree
                            .get(id)
                            .expect("root ids always resolve in their own tree");
                        let subtree = v.tree.subtree_ids(id);
                        let count = self
                            .index
                            .link_count_for_subtree(&v.id, &subtree)
                            .unwrap_or(0);
                        (note.title.as_str(), count)
                    })
                    .collect();
                (v.id.as_str(), roots)
            })
            .collect()
    }

    /// Opens the selected note's body for editing. No-op if nothing's
    /// selected (there's no note to edit) — mirrors `begin_rename`'s guard.
    pub fn begin_edit_body(&mut self) {
        let Some(id) = self.selected else { return };
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
    ///   (`TagFilterOp::Any`); opens `Mode::TagResults` if there are hits
    /// - `panes reset` — resets the split layout to `DEFAULT_PANE_WIDTHS`;
    ///   the only way back to it now that widths persist across restarts,
    ///   short of hand-editing or deleting `session.toml`
    ///
    /// Kept in sync with `COMMAND_REFERENCE` below by hand — only four
    /// entries, not worth generating one from the other.
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
            _ => {
                self.last_message = None;
                self.last_error = Some(format!("unknown command: {name}"));
            }
        }
    }

    fn command_reindex(&mut self) {
        match self.reindex_mounted() {
            Ok(count) => {
                self.last_error = None;
                self.last_message = Some(format!("reindexed {count} note(s)"));
            }
            Err(err) => {
                self.last_message = None;
                self.last_error = Some(format!("reindex failed: {err}"));
            }
        }
    }

    /// `:tags tag1,tag2` — notes matching any of the given tags. Opens
    /// `Mode::TagResults` on a hit, otherwise reports through
    /// `last_message`/`last_error` instead. AND semantics (every tag
    /// required) and a keybinding for either aren't exposed yet — this is
    /// the first, simplest entry point for `Index::filter_by_tags`, which
    /// has had no TUI surface at all since v0.4.
    fn command_tags(&mut self, args: &str) {
        let tags: Vec<String> = args
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect();
        if tags.is_empty() {
            self.last_message = None;
            self.last_error = Some("usage: :tags <tag1,tag2,...>".to_string());
            return;
        }

        match self.index.filter_by_tags(&self.vault_id, &tags, TagFilterOp::Any) {
            Ok(hits) if hits.is_empty() => {
                self.last_error = None;
                self.last_message = Some(format!("no notes tagged {}", tags.join(", ")));
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
                self.last_error = Some(format!("tag filter failed: {err}"));
            }
        }
    }

    fn command_panes(&mut self, args: &str) {
        if args.trim() != "reset" {
            self.last_message = None;
            self.last_error = Some("usage: :panes reset".to_string());
            return;
        }
        self.pane_widths = Self::DEFAULT_PANE_WIDTHS;
        self.last_error = None;
        self.last_message = Some("pane widths reset to default".to_string());
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
            self.selected = Some(id);
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
}
