---
id: 5ebd7be5-27d1-4bb3-9bf3-a360e2a64ceb
parent: 2842830d-5650-4b09-8cd1-edc9bf0bb312
order: 3
tags:
- usage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Outils en ligne de commande — l'exemple de Mycora

[[Rust]] est un choix courant pour les outils en ligne de commande et
les applications terminal : binaire unique, démarrage instantané, pas
de runtime à installer. Mycora — l'application qui affiche ce vault —
en est un exemple concret : une TUI construite avec les crates
`ratatui` et `crossterm`, ses arguments parsés avec `clap`, son index
de recherche SQLite embarqué via `rusqlite` (feature `bundled`, pas de
dépendance système), le tout distribué comme un unique binaire publié
sur [[crates.io et les crates incontournables]].
