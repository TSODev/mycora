---
id: 955647d0-9937-4373-bd12-af0136ef8b3f
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 4
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Gestion des erreurs avec Result et Option

Pas d'exceptions en Rust : une absence de valeur se représente par
`Option<T>` (`Some(T)` / `None`), une opération qui peut échouer par
`Result<T, E>` (`Ok(T)` / `Err(E)`). L'opérateur `?` propage une erreur
au premier appelant en une syntaxe minimale, sans `try`/`catch`. Ces
deux types s'utilisent avec les [[Enums et pattern matching]] du
langage et forcent, à la compilation, à traiter explicitement le cas
d'échec plutôt que de le découvrir à l'exécution.
