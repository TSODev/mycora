use std::collections::HashMap;

use crate::note::{Note, NoteId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    NotFound,
    /// The requested new parent is the note itself or one of its own
    /// descendants — reparenting there would disconnect the note from the
    /// tree's root.
    Cycle,
}

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
    /// disk (or restored from an undo snapshot). Doesn't update `roots` or
    /// any `children` list — call `rebuild_hierarchy` once every note has
    /// been inserted.
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
                // `parent_id != id` also catches a note whose `parent`
                // field (malformed on-disk frontmatter, not reachable
                // through any in-app mutation) names itself — without
                // this guard it would become its own sole child, never
                // appear in `roots`, and be silently unreachable from any
                // real traversal despite still existing in `self.notes`.
                Some(parent_id) if parent_id != id && self.notes.contains_key(&parent_id) => {
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

    /// Every note in the tree, in arbitrary order — for callers (the SQLite
    /// indexer) that need to visit all of them rather than walk structurally.
    pub fn iter(&self) -> impl Iterator<Item = (NoteId, &Note)> {
        self.notes.iter().map(|(id, note)| (*id, note))
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

    /// Replaces `id`'s tag list wholesale — the add/remove-a-single-tag
    /// logic (dedup on add, no-op if missing on remove) lives in the
    /// caller (`App::command_tag`); `Tree` just applies the already-
    /// decided result, same as `set_body` doesn't itself decide what the
    /// new body should be.
    pub fn set_tags(&mut self, id: NoteId, tags: Vec<String>) -> bool {
        match self.notes.get_mut(&id) {
            Some(note) => {
                note.tags = tags;
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

    /// Reparents `id` under `new_parent` (or to root if `None`), appending it
    /// after `new_parent`'s current last child. O(depth) for the cycle
    /// check, O(siblings) to detach — never O(size of vault).
    pub fn move_note(&mut self, id: NoteId, new_parent: Option<NoteId>) -> Result<(), MoveError> {
        if !self.notes.contains_key(&id) {
            return Err(MoveError::NotFound);
        }
        if new_parent == Some(id) || new_parent.is_some_and(|p| self.is_descendant(id, p)) {
            return Err(MoveError::Cycle);
        }

        let old_parent = self.notes[&id].parent;
        if old_parent == new_parent {
            return Ok(());
        }

        let old_siblings = match old_parent {
            Some(p) => self.notes.get_mut(&p).map(|note| &mut note.children),
            None => Some(&mut self.roots),
        };
        if let Some(siblings) = old_siblings {
            siblings.retain(|&s| s != id);
        }

        let order = self.next_order(new_parent);
        match new_parent {
            Some(p) => {
                if let Some(parent_note) = self.notes.get_mut(&p) {
                    parent_note.children.push(id);
                }
            }
            None => self.roots.push(id),
        }

        if let Some(note) = self.notes.get_mut(&id) {
            note.parent = new_parent;
            note.order = order;
            note.updated = time::OffsetDateTime::now_utc();
        }

        Ok(())
    }

    /// Is `candidate` equal to `ancestor` or one of its descendants? Walks
    /// up from `candidate` via `parent` links — O(depth), not O(subtree).
    fn is_descendant(&self, ancestor: NoteId, candidate: NoteId) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.notes.get(&id).and_then(|note| note.parent);
        }
        false
    }

    /// Swaps `id` with its previous sibling. Returns `false` (no-op) if `id`
    /// is already first, or doesn't exist.
    pub fn move_up(&mut self, id: NoteId) -> bool {
        self.swap_with_sibling(id, -1)
    }

    /// Swaps `id` with its next sibling. Returns `false` (no-op) if `id` is
    /// already last, or doesn't exist.
    pub fn move_down(&mut self, id: NoteId) -> bool {
        self.swap_with_sibling(id, 1)
    }

    fn swap_with_sibling(&mut self, id: NoteId, direction: isize) -> bool {
        let Some(parent) = self.notes.get(&id).map(|note| note.parent) else {
            return false;
        };

        let mut siblings: Vec<NoteId> = match parent {
            Some(p) => self
                .notes
                .get(&p)
                .map(|note| note.children.clone())
                .unwrap_or_default(),
            None => self.roots.clone(),
        };

        let Some(pos) = siblings.iter().position(|&s| s == id) else {
            return false;
        };
        let new_pos = pos as isize + direction;
        if new_pos < 0 || new_pos as usize >= siblings.len() {
            return false;
        }
        let new_pos = new_pos as usize;
        siblings.swap(pos, new_pos);

        for (i, &sibling_id) in siblings.iter().enumerate() {
            if let Some(note) = self.notes.get_mut(&sibling_id) {
                note.order = i as i64;
            }
        }

        match parent {
            Some(p) => {
                if let Some(parent_note) = self.notes.get_mut(&p) {
                    parent_note.children = siblings;
                }
            }
            None => self.roots = siblings,
        }

        true
    }

    /// Deep-copies `id` and its whole subtree under `new_parent`, with fresh
    /// ids and fresh `created`/`updated` timestamps (a copy is a new
    /// artifact, not a link — see ROADMAP.md's now-resolved copy-semantics
    /// question). Returns the new subtree root's id.
    pub fn deep_copy(&mut self, id: NoteId, new_parent: Option<NoteId>) -> Option<NoteId> {
        let note = self.notes.get(&id)?;
        let title = note.title.clone();
        let body = note.body.clone();
        let tags = note.tags.clone();
        let children = note.children.clone();

        let new_id = self.create_note(title, new_parent);
        if let Some(new_note) = self.notes.get_mut(&new_id) {
            new_note.body = body;
            new_note.tags = tags;
        }

        for child in children {
            self.deep_copy(child, Some(new_id));
        }

        Some(new_id)
    }

    /// Deep-copies `id` and its subtree from a *different* tree (`source`)
    /// into `self`, under `new_parent` — the cross-vault counterpart of
    /// `deep_copy`, which can only copy within its own tree (a read-only
    /// mounted vault's `Tree` and the active one are separate `Tree`
    /// instances, so a single-tree method can't reach across). Same fresh-
    /// ids/timestamps behavior as `deep_copy`. Returns the new subtree
    /// root's id, or `None` if `id` doesn't exist in `source`.
    pub fn deep_copy_from(
        &mut self,
        source: &Tree,
        id: NoteId,
        new_parent: Option<NoteId>,
    ) -> Option<NoteId> {
        let note = source.notes.get(&id)?;
        let title = note.title.clone();
        let body = note.body.clone();
        let tags = note.tags.clone();
        let children = note.children.clone();

        let new_id = self.create_note(title, new_parent);
        if let Some(new_note) = self.notes.get_mut(&new_id) {
            new_note.body = body;
            new_note.tags = tags;
        }

        for child in children {
            self.deep_copy_from(source, child, Some(new_id));
        }

        Some(new_id)
    }

    /// Removes `id` and its entire subtree, returning the full data of every
    /// removed note (root first, depth-first) so a caller can undo by
    /// reinserting it, or persist the removal (e.g. move to trash). O(size
    /// of subtree), never O(size of vault). `None` if `id` didn't exist.
    pub fn delete_subtree(&mut self, id: NoteId) -> Option<Vec<(NoteId, Note)>> {
        if !self.notes.contains_key(&id) {
            return None;
        }

        let ids = self.subtree_ids(id);

        let parent = self.notes[&id].parent;
        let siblings = match parent {
            Some(p) => self.notes.get_mut(&p).map(|note| &mut note.children),
            None => Some(&mut self.roots),
        };
        if let Some(siblings) = siblings {
            siblings.retain(|&s| s != id);
        }

        Some(
            ids.into_iter()
                .filter_map(|note_id| self.notes.remove(&note_id).map(|note| (note_id, note)))
                .collect(),
        )
    }

    /// Ids of `id` and all of its descendants, root first, depth-first.
    pub fn subtree_ids(&self, id: NoteId) -> Vec<NoteId> {
        let mut ids = Vec::new();
        self.collect_subtree(id, &mut ids);
        ids
    }

    fn collect_subtree(&self, id: NoteId, out: &mut Vec<NoteId>) {
        out.push(id);
        if let Some(note) = self.notes.get(&id) {
            for &child in &note.children {
                self.collect_subtree(child, out);
            }
        }
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
    fn set_tags_replaces_the_tag_list() {
        let mut tree = Tree::new();
        let id = tree.create_note("Note", None);
        assert!(tree.set_tags(id, vec!["a".to_string(), "b".to_string()]));
        assert_eq!(tree.get(id).unwrap().tags, vec!["a", "b"]);
    }

    #[test]
    fn set_tags_missing_note_returns_false() {
        let mut tree = Tree::new();
        assert!(!tree.set_tags(NoteId::new(), vec!["a".to_string()]));
    }

    #[test]
    fn delete_subtree_removes_leaf() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));

        let removed = tree.delete_subtree(child).unwrap();

        assert_eq!(removed.len(), 1);
        assert!(tree.children(parent).is_empty());
        assert!(tree.get(child).is_none());
    }

    #[test]
    fn delete_subtree_removes_all_descendants() {
        let mut tree = Tree::new();
        let grandparent = tree.create_note("Grandparent", None);
        let parent = tree.create_note("Parent", Some(grandparent));
        let child = tree.create_note("Child", Some(parent));

        let removed = tree.delete_subtree(parent).unwrap();

        let removed_ids: Vec<NoteId> = removed.iter().map(|(id, _)| *id).collect();
        assert_eq!(removed_ids, vec![parent, child]);
        assert!(tree.children(grandparent).is_empty());
        assert!(tree.get(parent).is_none());
        assert!(tree.get(child).is_none());
    }

    #[test]
    fn delete_subtree_missing_note_returns_none() {
        let mut tree = Tree::new();
        assert!(tree.delete_subtree(NoteId::new()).is_none());
    }

    #[test]
    fn delete_subtree_of_a_root_removes_it_from_roots() {
        let mut tree = Tree::new();
        let a = tree.create_note("A", None);
        let b = tree.create_note("B", None);

        tree.delete_subtree(a).unwrap();

        assert_eq!(tree.roots(), &[b]);
        assert!(tree.get(a).is_none());
    }

    #[test]
    fn delete_subtree_preserves_siblings() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let a = tree.create_note("A", Some(parent));
        let b = tree.create_note("B", Some(parent));

        tree.delete_subtree(a).unwrap();

        assert_eq!(tree.children(parent), &[b]);
        assert!(tree.get(b).is_some());
    }

    #[test]
    fn delete_subtree_returns_the_removed_notes_own_data() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        tree.set_body(parent, "parent body");
        let child = tree.create_note("Child", Some(parent));
        tree.set_body(child, "child body");

        let removed = tree.delete_subtree(parent).unwrap();

        assert_eq!(removed[0].0, parent);
        assert_eq!(removed[0].1.title, "Parent");
        assert_eq!(removed[0].1.body, "parent body");
        assert_eq!(removed[1].0, child);
        assert_eq!(removed[1].1.title, "Child");
        assert_eq!(removed[1].1.body, "child body");
    }

    #[test]
    fn move_note_reparents() {
        let mut tree = Tree::new();
        let a = tree.create_note("A", None);
        let b = tree.create_note("B", None);
        let child = tree.create_note("Child", Some(a));

        assert!(tree.move_note(child, Some(b)).is_ok());

        assert!(tree.children(a).is_empty());
        assert_eq!(tree.children(b), &[child]);
        assert_eq!(tree.get(child).unwrap().parent, Some(b));
    }

    #[test]
    fn move_note_rejects_cycle_into_own_descendant() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        let child = tree.create_note("Child", Some(root));
        let grandchild = tree.create_note("Grandchild", Some(child));

        assert_eq!(
            tree.move_note(root, Some(grandchild)),
            Err(MoveError::Cycle)
        );
        assert_eq!(tree.move_note(child, Some(child)), Err(MoveError::Cycle));
    }

    #[test]
    fn move_note_rejects_cycle_into_direct_child() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));

        assert_eq!(tree.move_note(parent, Some(child)), Err(MoveError::Cycle));
        // Nothing should have moved.
        assert_eq!(tree.children(parent), &[child]);
    }

    #[test]
    fn move_note_to_root_detaches_from_old_parent() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let child = tree.create_note("Child", Some(parent));

        assert!(tree.move_note(child, None).is_ok());

        assert!(tree.children(parent).is_empty());
        assert!(tree.roots().contains(&child));
        assert_eq!(tree.get(child).unwrap().parent, None);
    }

    #[test]
    fn move_note_to_same_parent_is_a_noop() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let a = tree.create_note("A", Some(parent));
        let b = tree.create_note("B", Some(parent));

        assert!(tree.move_note(a, Some(parent)).is_ok());

        // Order and membership must be untouched, not duplicated.
        assert_eq!(tree.children(parent), &[a, b]);
    }

    #[test]
    fn move_note_appends_after_existing_children() {
        let mut tree = Tree::new();
        let a = tree.create_note("A", None);
        let b = tree.create_note("B", None);
        let existing_child = tree.create_note("Existing", Some(b));

        assert!(tree.move_note(a, Some(b)).is_ok());

        assert_eq!(tree.children(b), &[existing_child, a]);
    }

    #[test]
    fn move_note_missing_note_errors() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        assert_eq!(
            tree.move_note(NoteId::new(), Some(root)),
            Err(MoveError::NotFound)
        );
    }

    #[test]
    fn move_up_and_down_swap_siblings() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let a = tree.create_note("A", Some(parent));
        let b = tree.create_note("B", Some(parent));

        assert!(tree.move_up(b));
        assert_eq!(tree.children(parent), &[b, a]);

        assert!(tree.move_down(b));
        assert_eq!(tree.children(parent), &[a, b]);
    }

    #[test]
    fn move_up_at_start_is_noop() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let a = tree.create_note("A", Some(parent));
        assert!(!tree.move_up(a));
    }

    #[test]
    fn move_down_at_end_is_noop() {
        let mut tree = Tree::new();
        let parent = tree.create_note("Parent", None);
        let a = tree.create_note("A", Some(parent));
        let b = tree.create_note("B", Some(parent));
        assert!(!tree.move_down(b));
        assert_eq!(tree.children(parent), &[a, b]);
    }

    #[test]
    fn move_up_and_down_reorder_root_level_siblings() {
        let mut tree = Tree::new();
        let a = tree.create_note("A", None);
        let b = tree.create_note("B", None);

        assert!(tree.move_up(b));
        assert_eq!(tree.roots(), &[b, a]);

        assert!(tree.move_down(b));
        assert_eq!(tree.roots(), &[a, b]);
    }

    #[test]
    fn move_up_and_down_on_a_missing_note_return_false() {
        let mut tree = Tree::new();
        assert!(!tree.move_up(NoteId::new()));
        assert!(!tree.move_down(NoteId::new()));
    }

    #[test]
    fn deep_copy_duplicates_subtree_with_new_ids() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        let child = tree.create_note("Child", Some(root));
        tree.set_body(child, "child body");

        let copy_root = tree.deep_copy(root, None).unwrap();

        assert_ne!(copy_root, root);
        assert_eq!(tree.get(copy_root).unwrap().title, "Root");
        let copy_children = tree.children(copy_root);
        assert_eq!(copy_children.len(), 1);
        assert_ne!(copy_children[0], child);
        assert_eq!(tree.get(copy_children[0]).unwrap().title, "Child");
        assert_eq!(tree.get(copy_children[0]).unwrap().body, "child body");

        // originals untouched
        assert_eq!(tree.children(root), &[child]);
    }

    #[test]
    fn deep_copy_missing_note_returns_none() {
        let mut tree = Tree::new();
        assert!(tree.deep_copy(NoteId::new(), None).is_none());
    }

    #[test]
    fn deep_copy_preserves_tags() {
        let mut tree = Tree::new();
        let id = NoteId::new();
        let mut note = Note::new("Root", None);
        note.tags = vec!["a".to_string(), "b".to_string()];
        tree.insert_loaded(id, note);
        tree.rebuild_hierarchy();

        let copy_id = tree.deep_copy(id, None).unwrap();

        assert_eq!(
            tree.get(copy_id).unwrap().tags,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn deep_copy_assigns_fresh_timestamps_not_the_originals() {
        let mut tree = Tree::new();
        let id = NoteId::new();
        let mut note = Note::new("Root", None);
        let ancient = time::OffsetDateTime::UNIX_EPOCH;
        note.created = ancient;
        note.updated = ancient;
        tree.insert_loaded(id, note);
        tree.rebuild_hierarchy();

        let copy_id = tree.deep_copy(id, None).unwrap();

        let copy = tree.get(copy_id).unwrap();
        assert_ne!(copy.created, ancient);
        assert_ne!(copy.updated, ancient);
    }

    #[test]
    fn deep_copy_from_duplicates_a_subtree_from_another_tree() {
        let mut source = Tree::new();
        let root = source.create_note("Root", None);
        let child = source.create_note("Child", Some(root));
        source.set_body(child, "child body");

        let mut dest = Tree::new();
        let dest_parent = dest.create_note("Destination", None);

        let copy_root = dest.deep_copy_from(&source, root, Some(dest_parent)).unwrap();

        assert_ne!(copy_root, root);
        assert_eq!(dest.get(copy_root).unwrap().title, "Root");
        assert_eq!(dest.get(copy_root).unwrap().parent, Some(dest_parent));
        let copy_children = dest.children(copy_root);
        assert_eq!(copy_children.len(), 1);
        assert_ne!(copy_children[0], child);
        assert_eq!(dest.get(copy_children[0]).unwrap().title, "Child");
        assert_eq!(dest.get(copy_children[0]).unwrap().body, "child body");

        // source untouched, and the copy never appears in it
        assert_eq!(source.children(root), &[child]);
        assert!(source.get(copy_root).is_none());
    }

    #[test]
    fn deep_copy_from_missing_note_returns_none() {
        let source = Tree::new();
        let mut dest = Tree::new();
        assert!(dest.deep_copy_from(&source, NoteId::new(), None).is_none());
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

    #[test]
    fn rebuild_hierarchy_promotes_a_note_whose_parent_is_itself() {
        // Not reachable through any in-app mutation (move_note's cycle
        // check already refuses this), but malformed on-disk frontmatter
        // (a hand-edited `parent:` field naming its own note) can still
        // produce one. Without the `parent_id != id` guard in
        // `rebuild_hierarchy`, this note would become its own sole child
        // and never appear in `roots` — invisible to every real
        // traversal despite still existing in the tree.
        let mut tree = Tree::new();
        let id = NoteId::new();
        tree.insert_loaded(id, Note::new("Self-parented", Some(id)));

        let orphaned = tree.rebuild_hierarchy();

        assert_eq!(orphaned, vec![id]);
        assert_eq!(tree.roots(), &[id]);
        assert_eq!(tree.get(id).unwrap().parent, None);
        assert!(tree.children(id).is_empty());
    }

    #[test]
    fn subtree_ids_is_root_first_depth_first() {
        let mut tree = Tree::new();
        let root = tree.create_note("Root", None);
        let a = tree.create_note("A", Some(root));
        let a1 = tree.create_note("A1", Some(a));
        let b = tree.create_note("B", Some(root));

        assert_eq!(tree.subtree_ids(root), vec![root, a, a1, b]);
    }
}
