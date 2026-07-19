---
id: cfa33794-403f-490b-a711-335e51907c35
parent: 5c407a76-2d05-4c82-9467-fe5c7887d503
order: 5
tags:
- outils
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# crates.io et les crates incontournables

`crates.io` est le registre central de paquets (*crates*) de
l'écosystème, consulté par [[Cargo, le gestionnaire de projets]] à
chaque `cargo build`. Quelques crates reviennent dans une grande
partie des projets Rust : `serde` (sérialisation, voir [[Macros]]),
`tokio` (runtime asynchrone), `clap` (parsing d'arguments CLI), et,
pour les interfaces terminal comme celle de Mycora, `ratatui` +
`crossterm` — voir
[[Outils en ligne de commande — l'exemple de Mycora]].
