---
id: 8f413b8f-fad9-4803-8159-bd3cf52ec01d
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 2
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Le système de types et les traits

Un `trait` définit un comportement partagé (des méthodes) qu'un type
peut implémenter — l'équivalent, en plus statique, d'une interface. Pas
d'héritage de classes en Rust : la composition de traits (et les
`impl Trait` / objets `dyn Trait` quand la dynamique est vraiment
nécessaire) en tient lieu. Les traits sont aussi la brique de base de
la [[Généricité]] (`fn f<T: Trait>(x: T)`) et des
[[Macros]] `derive` (`#[derive(Debug, Clone)]` implémente un trait
automatiquement).
