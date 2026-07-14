---
id: a4c88761-6208-43cc-90a3-ea17218070ce
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 9
tags:
- features
created: 2026-07-14T09:21:00Z
updated: 2026-07-14T09:21:00Z
---

# Pipe mode

`cat file.json | jsoned` opens piped input directly; `s` writes the
current JSON to stdout and exits, so jsoned can sit in the middle of a
shell pipeline as an interactive editing step rather than only ever
being a terminal endpoint.
