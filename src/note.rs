/// Identifies a note within a single in-memory `Tree`.
///
/// Not yet a stable identity scheme — see ROADMAP.md's open design question
/// on note identity (UUID vs. content-hash vs. path-derived), to be settled
/// before v0.3 once notes are persisted and can be renamed/moved on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub usize);

pub struct Note {
    pub title: String,
    pub body: String,
    pub parent: Option<NoteId>,
    pub children: Vec<NoteId>,
}

impl Note {
    pub fn new(title: impl Into<String>, parent: Option<NoteId>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            parent,
            children: Vec::new(),
        }
    }
}
