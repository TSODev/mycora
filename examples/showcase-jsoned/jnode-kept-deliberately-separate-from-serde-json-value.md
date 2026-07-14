---
id: 600febd0-c60b-4241-a182-c9a6ba87a69b
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 0
tags:
- architecture
created: 2026-07-14T09:05:00Z
updated: 2026-07-14T09:05:00Z
---

# JNode, kept deliberately separate from serde_json::Value

`JNode` (`src/tree.rs`) is a hand-rolled mutable tree —
`Object(IndexMap<String, JNode>, collapsed)` / `Array(Vec<JNode>,
collapsed)` / `Scalar` — kept apart from `serde_json::Value`
specifically so cursor position, fold state, and undo history can live
directly on the tree itself, rather than bolted onto a generic value
type that was never meant to carry any of that. `JScalar::Number`
stores the raw source string rather than a parsed number, to preserve
source formatting like `1.0` vs `1` — a detail a generic JSON value
type would normally throw away. `JPath` is a `Vec<JKey>`, and
`JKey::Field` wraps its string in an `Rc<str>` specifically so cloning
a path — done constantly across flatten/annotate/lint/diff — is a
refcount bump, not a string copy.
