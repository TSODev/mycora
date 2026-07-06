use std::collections::HashMap;

use crate::note::{Note, NoteId};

/// UI-agnostic in-memory tree of notes. Every note is a root or has exactly
/// one parent; cross-links are a separate concern and live outside this type.
pub struct Tree {
    notes: HashMap<NoteId, Note>,
    roots: Vec<NoteId>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            roots: Vec::new(),
        }
    }

    pub fn create_note(&mut self, title: impl Into<String>, parent: Option<NoteId>) -> NoteId {
        let id = NoteId::new();
        let order = self.next_order(parent);

        let mut note = Note::new(title, parent);
        note.order = order;

        match parent {
            Some(parent_id) => {
                if let Some(parent_note) = self.notes.get_mut(&parent_id) {
                    parent_note.children.push(id);
                }
            }
            None => self.roots.push(id),
        }

        self.notes.insert(id, note);
        id
    }

    fn next_order(&self, parent: Option<NoteId>) -> i64 {
        let siblings: &[NoteId] = match parent {
            Some(parent_id) => self
                .notes
                .get(&parent_id)
                .map(|note| note.children.as_slice())
                .unwrap_or(&[]),
            None => self.roots.as_slice(),
        };

        siblings
            .iter()
            .filter_map(|id| self.notes.get(id).map(|note| note.order))
            .max()
            .map_or(0, |max| max + 1)
    }

    /// Inserts a note with a pre-existing id and parent pointer, as read from
    /// disk. Doesn't update `roots` or any `children` list — call
    /// `rebuild_hierarchy` once every note from the vault has been inserted.
    pub fn insert_loaded(&mut self, id: NoteId, note: Note) {
        self.notes.insert(id, note);
    }

    /// Rebuilds `roots` and every note's `children` from each note's current
    /// `parent` field, sorted by `order`. Used once after bulk-loading notes
    /// from disk via `insert_loaded`. Notes whose `parent` doesn't resolve to
    /// another loaded note are promoted to roots (their `parent` cleared);
    /// their ids are returned so the caller can flag/repair them.
    pub fn rebuild_hierarchy(&mut self) -> Vec<NoteId> {
        let mut children_of: HashMap<NoteId, Vec<NoteId>> = HashMap::new();
        let mut roots = Vec::new();
        let mut orphaned = Vec::new();

        let ids: Vec<NoteId> = self.notes.keys().copied().collect();
        for id in ids {
            let parent = self.notes[&id].parent;
            match parent {
                Some(parent_id) if self.notes.contains_key(&parent_id) => {
                    children_of.entry(parent_id).or_default().push(id);
                }
                Some(_) => {
                    if let Some(note) = self.notes.get_mut(&id) {
                        note.parent = None;
                    }
                    roots.push(id);
                    orphaned.push(id);
                }
                None => roots.push(id),
            }
        }

        for children in children_of.values_mut() {
            children.sort_by_key(|id| self.notes[id].order);
        }
        roots.sort_by_key(|id| self.notes[id].order);

        for note in self.notes.values_mut() {
            note.children.clear();
        }
        for (parent_id, children) in children_of {
            if let Some(parent_note) = self.notes.get_mut(&parent_id) {
                parent_note.children = children;
            }
        }

        self.roots = roots;
        orphaned
    }

    pub fn get(&self, id: NoteId) -> Option<&Note> {
        self.notes.get(&id)
    }

    pub fn rename(&mut self, id: NoteId, title: impl Into<String>) -> bool {
        match self.notes.get_mut(&id) {
            Some(note) => {
                note.title = title.into();
                note.updated = time::OffsetDateTime::now_utc();
                true
            }
            None => false,
        }
    }

    pub fn set_body(&mut self, id: NoteId, body: impl Into<String>) -> bool {
        match self.notes.get_mut(&id) {
            Some(note) => {
                note.body = body.into();
                note.updated = time::OffsetDateTime::now_utc();
                true
            }
            None => false,
        }
    }

    pub fn roots(&self) -> &[NoteId] {
        &self.roots
    }

    pub fn children(&self, id: NoteId) -> &[NoteId] {
        self.notes
            .get(&id)
            .map(|note| note.children.as_slice())
            .unwrap_or(&[])
    }

    /// Removes a single note, promoting its children to its own parent (or to
    /// the root list) in its place. This is not a cascading subtree delete —
    /// that's a distinct, explicit operation reserved for v0.3. Returns the
    /// ids of the children whose `parent` field changed, so callers can
    /// re-persist them; `None` if `id` didn't exist.
    pub fn delete(&mut self, id: NoteId) -> Option<Vec<NoteId>> {
        let note = self.notes.remove(&id)?;

        for &child in &note.children {
            if let Some(child_note) = self.notes.get_mut(&child) {
                child_note.parent = note.parent;
            }
        }

        let siblings = match note.parent {
            Some(parent_id) => match self.notes.get_mut(&parent_id) {
                Some(parent_note) => &mut parent_note.children,
                None => &mut self.roots,
            },
            None => &mut self.roots,
        };

        if let Some(pos) = siblings.iter().position(|&s| s == id) {
            siblings.splice(pos..pos + 1, note.children.iter().copied());
        }

        Some(note.children)
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_note_adds_root() {
        let mut tree = Tree::new();
        let id = tree.create_note("Root", None);
        assert_eq!(tree.roots(), &[id]);
        assert_eq!(tree.get(id).unwrap().title, "Root");
    }

    #[test]
    fn create_note_registers_with_parent() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));
        assert_eq!(tree.children(parent), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, Some(parent));
    }

    #[test]
    fn rename_updates_title() {
        let mut tree = Tree::new();
        let id = tree.create_note("Old", None);
        assert!(tree.rename(id, "New"));
        assert_eq!(tree.get(id).unwrap().title, "New");
    }

    #[test]
    fn rename_missing_note_returns_false() {
        let mut tree = Tree::new();
        assert!(!tree.rename(NoteId::new(), "New"));
    }

    #[test]
    fn delete_leaf_removes_from_parent() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));
        assert!(tree.delete(child).is_some());
        assert!(tree.children(parent).is_empty());
        assert!(tree.get(child).is_none());
    }

    #[test]
    fn delete_promotes_children_to_grandparent() {
        let mut tree = Tree::new();
        let grandparent = tree.create_note("Grandparent", None);
        let parent = tree.create_note("Parent", Some(grandparent));
        let child = tree.create_note("Child", Some(parent));

        assert!(tree.delete(parent).is_some());

        assert_eq!(tree.children(grandparent), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, Some(grandparent));
    }

    #[test]
    fn delete_root_promotes_children_to_roots() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        let child = tree.create_note("Child", Some(root));

        assert!(tree.delete(root).is_some());

        assert_eq!(tree.roots(), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, None);
    }

    #[test]
    fn delete_missing_note_returns_false() {
        let mut tree = Tree::new();
        assert!(tree.delete(NoteId::new()).is_none());
    }

    #[test]
    fn rebuild_hierarchy_orders_children_by_order_field() {
        let mut tree = Tree::new();
        let root_id = NoteId::new();
        tree.insert_loaded(root_id, Note::new("Root", None));

        let mut child_a = Note::new("A", Some(root_id));
        child_a.order = 1;
        let a_id = NoteId::new();

        let mut child_b = Note::new("B", Some(root_id));
        child_b.order = 0;
        let b_id = NoteId::new();

        tree.insert_loaded(a_id, child_a);
        tree.insert_loaded(b_id, child_b);

        let orphaned = tree.rebuild_hierarchy();

        assert!(orphaned.is_empty());
        assert_eq!(tree.roots(), &[root_id]);
        assert_eq!(tree.children(root_id), &[b_id, a_id]);
    }

    #[test]
    fn rebuild_hierarchy_promotes_notes_with_missing_parent() {
        let mut tree = Tree::new();
        let missing_parent = NoteId::new();
        let orphan_id = NoteId::new();
        tree.insert_loaded(orphan_id, Note::new("Orphan", Some(missing_parent)));

        let orphaned = tree.rebuild_hierarchy();

        assert_eq!(orphaned, vec![orphan_id]);
        assert_eq!(tree.roots(), &[orphan_id]);
        assert_eq!(tree.get(orphan_id).unwrap().parent, None);
    }
}
