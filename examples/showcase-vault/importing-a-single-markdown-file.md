---
id: cdea8b85-7b56-41f7-a388-c453698eb81d
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 12
tags:
- features
- import
created: 2026-07-17T09:00:00Z
updated: 2026-07-17T09:00:00Z
---

# Importing a single Markdown file

[[Importing an Obsidian vault]] is CLI-only and always creates a brand
new vault. `:import <path>` in the [[Command palette]] is the other
case: pulling one external `.md` file into a vault you already have
open, as a **new child note of the selected note** — no
copy-pasting the content in by hand.

Parsed exactly the way a single file inside an Obsidian-vault import
is — the two share one parser rather than two subtly different ones:

- **Title** comes from the filename, not a heading inside the file.
- **Tags** carry over from optional YAML frontmatter (single string or
  list form). Missing or unparseable frontmatter just means no tags,
  not a failed import — tags are best-effort here, same as in the bulk
  importer.
- **Links**: an aliased or heading-anchored wikilink gets rewritten down
  to a plain bare-title one, since [[Cross-links and backlinks]]'s
  resolution only understands that bare form.

`~/` expands to the home directory, same as the attach-file prompt (see
[[Attaching files to a note]]). Requires a selection — the new note
needs a parent — and the active vault to be editable, same as `a`/`o`.
Creating the note and writing it to disk both happen as one step,
undoable with `u` like any other note creation.
