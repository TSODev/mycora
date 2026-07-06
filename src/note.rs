use time::OffsetDateTime;
use uuid::Uuid;

/// Identifies a note across restarts. Backed by a random UUID v4, generated
/// once at creation and persisted in frontmatter — stable across renames,
/// moves, and content edits, unlike a path- or content-derived id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub Uuid);

impl NoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NoteId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Note {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub parent: Option<NoteId>,
    pub children: Vec<NoteId>,
    pub order: i64,
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
}

impl Note {
    pub fn new(title: impl Into<String>, parent: Option<NoteId>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            title: title.into(),
            body: String::new(),
            tags: Vec::new(),
            parent,
            children: Vec::new(),
            order: 0,
            created: now,
            updated: now,
        }
    }
}
