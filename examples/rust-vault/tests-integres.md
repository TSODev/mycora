---
id: 1109c0fa-1f1b-42a9-904b-f30c7627ba58
parent: 5c407a76-2d05-4c82-9467-fe5c7887d503
order: 4
tags:
- outils
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Tests intégrés au langage

Un test unitaire est une fonction annotée `#[test]`, généralement dans
un module `#[cfg(test)] mod tests` juste à côté du code qu'elle
vérifie — pas de framework externe requis. `cargo test` les compile et
les exécute en parallèle par défaut. C'est exactement cette convention
que Mycora suit lui-même pour ses 232 tests, comme documenté dans son
propre CLAUDE.md — voir
[[Outils en ligne de commande — l'exemple de Mycora]].
