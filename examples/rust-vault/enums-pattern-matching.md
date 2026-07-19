---
id: f6997825-6fb7-41d9-9b6c-ab1b9593d639
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 3
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Enums et pattern matching

Les `enum` de Rust sont des types somme : chaque variante peut porter
ses propres données (`enum Shape { Circle(f64), Rect(f64, f64) }`), pas
seulement une étiquette. Le `match` associé est exhaustif — le
compilateur refuse de compiler si une variante n'est pas traitée. Les
deux types les plus utilisés du langage — `Option` et `Result`, voir
[[Gestion des erreurs avec Result et Option]] — sont eux-mêmes de
simples `enum` de la bibliothèque standard.
