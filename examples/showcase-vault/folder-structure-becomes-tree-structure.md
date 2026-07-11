---
id: 9c305c76-0e99-40b6-8ef2-c3f307bdae58
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 9
tags:
- design-decision
- import
created: 2026-07-11T09:30:00Z
updated: 2026-07-11T09:30:00Z
---

# Folder structure becomes tree structure

[[Importing an Obsidian vault]] had to resolve a real tension before
any code got written: [[Why a strict tree]] says every Mycora note has
exactly one parent, full stop, but Obsidian has no `parent` field at
all. Its only organizational structure is the filesystem — folders —
and its notes otherwise form a free graph via wikilinks, not a tree.

Raised this one with the user before designing further rather than
picking a default alone. The alternative considered: a flat import,
every note landing as a sibling root with no imported hierarchy at all,
relying purely on wikilinks to hold things together afterward. Simpler
to build, but it throws away real information — however someone
organized their Obsidian vault into folders was a deliberate choice on
their part, not incidental.

**Confirmed: map the folder structure onto the tree.** A subdirectory
becomes a parent note in the import; if a same-named `.md` file already
sits next to it (Obsidian's own "folder note" convention), that file's
own content becomes the parent note instead of an empty placeholder, so
nothing real gets thrown away either way. Preserves the actual shape of
someone's existing vault rather than flattening it into an undifferentiated
pile that only wikilinks hold together.
