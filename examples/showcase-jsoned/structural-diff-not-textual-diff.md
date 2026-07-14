---
id: 680d2a6b-f556-41ae-aeae-2ab280d6b961
parent: b40834c8-5aa3-404f-a230-61c767857208
order: 1
tags:
- philosophy
created: 2026-07-14T09:03:00Z
updated: 2026-07-14T09:03:00Z
---

# Structural diff, not textual diff

`jsoned a.json --diff b.json` compares by key path, not by line — it
understands JSON/YAML/TOML/CSV shape directly, so a reordered key or a
reformatted whitespace change doesn't show up as noise the way it would
in a line-based diff. It's explicitly not trying to replace
difftastic/delta for general text diffing. See [[Diff gets its own read-only app]] for how it's implemented, and its one documented
limitation.
