use std::collections::HashSet;

use crate::note::NoteId;
use crate::tree::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

pub struct App {
    pub tree: Tree,
    pub expanded: HashSet<NoteId>,
    pub selected: Option<NoteId>,
    pub mode: Mode,
    pub input: String,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut tree = Tree::new();
        let welcome = tree.create_note("Welcome to Mycora", None);
        tree.set_body(
            welcome,
            "a: new child  o: new sibling  i: rename  d: delete  q: quit",
        );

        let mut expanded = HashSet::new();
        expanded.insert(welcome);

        Self {
            tree,
            expanded,
            selected: Some(welcome),
            mode: Mode::Normal,
            input: String::new(),
            should_quit: false,
        }
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
        self.begin_naming();
    }

    pub fn create_child(&mut self) {
        if let Some(parent) = self.selected {
            let new_id = self.tree.create_note("New note", Some(parent));
            self.expanded.insert(parent);
            self.selected = Some(new_id);
            self.begin_naming();
        }
    }

    /// Starts insert mode with an empty input, for naming a freshly created
    /// note. Unlike `begin_rename`, doesn't prefill the placeholder title —
    /// the user types the name outright instead of editing "New note" away.
    fn begin_naming(&mut self) {
        self.input.clear();
        self.mode = Mode::Insert;
    }

    pub fn delete_selected(&mut self) {
        if let Some(id) = self.selected {
            let next = self.neighbor_after(id);
            self.tree.delete(id);
            self.expanded.remove(&id);
            self.selected = next;
        }
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
        if !self.input.trim().is_empty() {
            if let Some(id) = self.selected {
                self.tree.rename(id, self.input.clone());
            }
        }
        self.input.clear();
        self.mode = Mode::Normal;
    }

    pub fn cancel_rename(&mut self) {
        self.input.clear();
        self.mode = Mode::Normal;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
