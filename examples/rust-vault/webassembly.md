---
id: 1d805f9a-e23b-4871-b63f-33809f0a232b
parent: 2842830d-5650-4b09-8cd1-edc9bf0bb312
order: 1
tags:
- usage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# WebAssembly

Rust compile vers WebAssembly (cible `wasm32-unknown-unknown`, ajoutée
via [[rustup et les toolchains]]) sans runtime lourd embarqué —
contrairement à un langage garbage-collecté, qui devrait embarquer son
ramasse-miettes dans le binaire `.wasm`. Ça en fait un choix fréquent
pour du code performant exécuté dans le navigateur ou dans des
runtimes WASM côté serveur.
