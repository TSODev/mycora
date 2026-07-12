---
id: c3ab15bb-8701-4bfc-a7f2-356b563167da
parent: 6238be61-b346-445d-adc0-ec88f2b9c3c7
order: 2
tags:
- philosophy
- storage
created: 2026-07-10T09:00:00Z
updated: 2026-07-12T09:00:00Z
---

# Markdown as source of truth

Notes are stored as Markdown files with YAML frontmatter —
one file per note (`id`, `parent`, `order`, `tags`, `created`, `updated`,
then `# Title` and the body). Nothing about your data should require
Mycora to remain readable: open any note in any text editor and it's
still just Markdown with a small metadata header.

Writes are **atomic**: every persistent file Mycora owns (a note, plus
`config.toml` and `session.toml`) is written to a temp file first, then
renamed into place — a rename on the same filesystem can't leave a
half-written file behind, so a crash or power loss mid-write can't
truncate or corrupt any of them. See [[Roadmap]] v0.9.

The corollary: **the index is disposable**. A local SQLite database
(tree position, backlinks, full-text search) is derived entirely from the
Markdown files and can be rebuilt from scratch at any time — see
[[Search and indexing]] for what that index actually holds, and why
schema changes there don't need real migrations, just a drop-and-recreate.

Malformed files, duplicate ids, and orphaned parents are self-healed with
a warning rather than causing a crash or silent data loss — see
[[Tree operations]]. That includes a note whose `parent` names itself:
structurally the same as any other unresolvable parent, so it gets the
same treatment (promoted to root, warned about, healed on next save)
rather than silently disappearing from navigation.
