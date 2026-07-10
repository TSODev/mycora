use std::collections::HashSet;

use crate::config::Config;
use crate::index::{Index, IndexedNote};
use crate::note::{Note, NoteId};
use crate::tree::Tree;
use crate::vault::Vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    /// Awaiting y/n confirmation for `pending_delete`.
    ConfirmDelete,
    /// Typing a full-text query; `search_results` updates on every keystroke.
    Search,
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
    search_results: Vec<IndexedNote>,
    search_selected: usize,
}

impl App {
    /// Loads config + vault from disk and returns the ready-to-run app along
    /// with any load warnings (malformed files, orphaned/duplicate ids) for
    /// the caller to print before the TUI takes over the terminal.
    pub fn new() -> anyhow::Result<(Self, Vec<String>)> {
        let config = Config::load()?;
        let active = config.active_vault().clone();
        let mut vault = Vault::open(active.path.clone())?;
        let (mut tree, report) = vault.load()?;

        let selected = if tree.roots().is_empty() {
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

        let index_path = Index::default_path(&config.home);
        let mut index = Index::open(&index_path)?;
        // Reindex once at startup so search reflects the vault as loaded,
        // without requiring `mycora reindex` to have been run separately —
        // the index is disposable, so rebuilding it here is just as valid
        // a source as an on-disk one from a previous session.
        index.reindex(&active.name, &tree, &vault)?;

        let app = Self {
            tree,
            vault,
            expanded,
            selected,
            mode: Mode::Normal,
            input: String::new(),
            should_quit: false,
            last_error: None,
            confirm_quit: false,
            pending_delete: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            index,
            vault_id: active.name,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
        };

        Ok((app, report.warnings))
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
        }
    }

    /// Enters search mode. Reindexes first so results reflect the live
    /// in-memory tree (including edits made this session that a prior
    /// `mycora reindex` run on disk wouldn't know about), not a stale copy.
    pub fn begin_search(&mut self) {
        if let Err(err) = self.index.reindex(&self.vault_id, &self.tree, &self.vault) {
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
        let mut current = self.tree.get(id).and_then(|note| note.parent);
        while let Some(ancestor) = current {
            self.expanded.insert(ancestor);
            current = self.tree.get(ancestor).and_then(|note| note.parent);
        }
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn search_results(&self) -> &[IndexedNote] {
        &self.search_results
    }

    pub fn search_selected(&self) -> usize {
        self.search_selected
    }
}
