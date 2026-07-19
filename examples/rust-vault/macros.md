---
id: 34576d41-6518-46a2-a524-cd6243ac3377
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 6
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Macros

Les macros Rust opèrent sur le code lui-même plutôt que sur du texte :
les macros déclaratives (`macro_rules!`) transforment un motif de
syntaxe en un autre, et les macros procédurales `#[derive(...)]`
génèrent une implémentation de trait entière à la compilation — par
exemple `serde`, la bibliothèque de sérialisation la plus utilisée de
l'écosystème (voir
[[crates.io et les crates incontournables]]), dérive `Serialize`/
`Deserialize` pour n'importe quelle structure sans code écrit à la
main.
