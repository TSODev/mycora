---
id: 4c0f100d-1a5d-45db-a630-e624ca170e1f
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 4
tags:
- features
- session
- v0.7
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Session persistence

Mycora remembers, per vault, the last selected note and
which branches were expanded — saved once at shutdown (not on every
keystroke, since this is ephemeral navigation state rather than user
content) to `~/.local/share/mycora/session.toml`, alongside the SQLite
[[Search and indexing]] index in the XDG data directory.

On restart, ids that no longer resolve (a note was deleted, or the vault
changed) are simply dropped rather than kept dangling, and the restored
selection's ancestors are always expanded so it's actually visible on
screen, regardless of what was saved.

Pane widths (see [[Layout]]) are deliberately **not** part of this: they're
a display preference, not per-vault navigation state, so they reset to
the 40/40/20 default on every launch.
