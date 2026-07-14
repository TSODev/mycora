---
id: 58113b13-8127-41be-aebe-399da3503828
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 3
tags:
- features
created: 2026-07-14T09:15:00Z
updated: 2026-07-14T09:15:00Z
---

# Undo and redo

50 levels of history, captured before every change. Unlike Mycora's own
undo/redo (built on inverses recomputed against the live tree at apply
time), jsoned's history is full snapshots of the whole tree — see
[[Undo and redo still clone the whole tree]] for what that costs on a
large document.
