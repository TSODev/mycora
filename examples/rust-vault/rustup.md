---
id: c7bdbcd4-b7cb-4002-bbdf-beba642bb97d
parent: 5c407a76-2d05-4c82-9467-fe5c7887d503
order: 1
tags:
- outils
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# rustup et les toolchains

`rustup` installe et gère les *toolchains* Rust (compilateur `rustc` +
[[Cargo, le gestionnaire de projets]] + composants associés), permet
de basculer entre canaux `stable`/`beta`/`nightly`, d'ajouter des
cibles de compilation croisée (par exemple pour le [[WebAssembly]] ou
les [[Systèmes embarqués]]), et respecte un fichier
`rust-toolchain.toml` par projet quand la version doit être épinglée
(ce dépôt Mycora, lui, n'en épingle pas — voir son propre CLAUDE.md).
