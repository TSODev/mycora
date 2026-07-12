---
id: d06bc381-bbdc-4091-b942-825c2c5a26bd
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 11
tags:
- design-decision
- v0.9
created: 2026-07-12T09:00:00Z
updated: 2026-07-12T09:00:00Z
---

# Every write to disk is atomic

Part of [[Roadmap]] v0.9's crash-safety goal: no data loss on an
unexpected exit. A note's own file was already written atomically since
early on (see [[Markdown as source of truth]]) — write the new content
to a `.tmp` file next to it, then `fs::rename` the `.tmp` over the real
path. A rename on the same filesystem is atomic, so the real file is
always either the complete old content or the complete new content,
never a half-written mix — a crash or power loss mid-write can't
truncate or corrupt it.

`config.toml` and `session.toml` didn't get the same treatment when they
were first built — both used a plain write, so a crash at the wrong
moment could leave either one truncated on the next launch. Audited
every write path in the crate and fixed both to use the same
temp-file-then-rename pattern as notes.

Deliberately **not** extended to two other write paths, for different
reasons:
- [[Exporting a subtree]]'s output file is an arbitrary path outside any
  vault, already refuses to overwrite an existing one, and isn't
  persistent Mycora state — a failed write there just means retrying
  the export, not losing anything Mycora is responsible for keeping safe.
- The SQLite index is explicitly disposable (see
  [[Disposable SQLite index]]) and `mycora reindex` rebuilds it from
  scratch at any time, so a partially-written index is a non-issue by
  design, not a gap to close.
