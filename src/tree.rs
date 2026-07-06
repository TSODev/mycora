use std::collections::HashMap;

use crate::note::{Note, NoteId};

/// UI-agnostic in-memory tree of notes. Every note is a root or has exactly
/// one parent; cross-links are a separate concern and live outside this type.
pub struct Tree {
    notes: HashMap<NoteId, Note>,
    roots: Vec<NoteId>,
    next_id: usize,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            roots: Vec::new(),
            next_id: 0,
        }
    }

    pub fn create_note(&mut self, title: impl Into<String>, parent: Option<NoteId>) -> NoteId {
        let id = NoteId(self.next_id);
        self.next_id += 1;

        match parent {
            Some(parent_id) => {
                if let Some(parent_note) = self.notes.get_mut(&parent_id) {
                    parent_note.children.push(id);
                }
            }
            None => self.roots.push(id),
        }

        self.notes.insert(id, Note::new(title, parent));
        id
    }

    pub fn get(&self, id: NoteId) -> Option<&Note> {
        self.notes.get(&id)
    }

    pub fn rename(&mut self, id: NoteId, title: impl Into<String>) -> bool {
        match self.notes.get_mut(&id) {
            Some(note) => {
                note.title = title.into();
                true
            }
            None => false,
        }
    }

    pub fn set_body(&mut self, id: NoteId, body: impl Into<String>) -> bool {
        match self.notes.get_mut(&id) {
            Some(note) => {
                note.body = body.into();
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
    /// that's a distinct, explicit operation reserved for v0.3.
    pub fn delete(&mut self, id: NoteId) -> bool {
        let Some(note) = self.notes.remove(&id) else {
            return false;
        };

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

        true
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
        assert!(!tree.rename(NoteId(42), "New"));
    }

    #[test]
    fn delete_leaf_removes_from_parent() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));
        assert!(tree.delete(child));
        assert!(tree.children(parent).is_empty());
        assert!(tree.get(child).is_none());
    }

    #[test]
    fn delete_promotes_children_to_grandparent() {
        let mut tree = Tree::new();
        let grandparent = tree.create_note("Grandparent", None);
        let parent = tree.create_note("Parent", Some(grandparent));
        let child = tree.create_note("Child", Some(parent));

        assert!(tree.delete(parent));

        assert_eq!(tree.children(grandparent), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, Some(grandparent));
    }

    #[test]
    fn delete_root_promotes_children_to_roots() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        let child = tree.create_note("Child", Some(root));

        assert!(tree.delete(root));

        assert_eq!(tree.roots(), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, None);
    }

    #[test]
    fn delete_missing_note_returns_false() {
        let mut tree = Tree::new();
        assert!(!tree.delete(NoteId(99)));
    }
}
