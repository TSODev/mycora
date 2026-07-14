---
id: 0b931664-911f-4981-a6f9-7190f48b5b4e
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 7
tags:
- features
created: 2026-07-14T09:19:00Z
updated: 2026-07-14T09:19:00Z
---

# Structural diff between files

`jsoned a.json --diff b.json` opens a read-only structural diff, even
across formats (JSON against YAML, say) — headless with `--to
text|json` for scripting. See [[Structural diff, not textual diff]]
for what "structural" means here, and its one documented limitation.
