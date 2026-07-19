---
id: 08c58ea8-3278-4f2b-b5f5-b3ab69af4b38
parent: 5c407a76-2d05-4c82-9467-fe5c7887d503
order: 0
tags:
- outils
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Cargo, le gestionnaire de projets

`cargo` construit le projet, résout et télécharge les dépendances
(*crates*) depuis [[crates.io et les crates incontournables]], lance
les [[Tests intégrés au langage]] (`cargo test`), publie un paquet,
génère la documentation (`cargo doc`) — un seul outil pour tout le
cycle de vie, là où d'autres écosystèmes en assemblent plusieurs. Un
fichier `Cargo.toml` déclare les dépendances par nom et version ; un
`Cargo.lock` versionné fige les versions résolues pour des builds
reproductibles. Cargo lui-même est distribué et mis à jour via
[[rustup et les toolchains]].
