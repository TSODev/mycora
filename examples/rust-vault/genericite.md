---
id: 1db28b8c-dce2-4c05-a577-62921e4d1e03
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 5
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Généricité

Une fonction ou un type générique (`fn max<T: PartialOrd>(a: T, b: T)
-> T`) est monomorphisé à la compilation : le compilateur génère une
version spécialisée par type concret réellement utilisé, aussi rapide
que si elle avait été écrite à la main pour ce type précis — c'est ce
qui rend la généricité de Rust « zero-cost », voir
[[Abstractions à coût nul]]. Les bornes (`T: Trait`) s'appuient
directement sur [[Le système de types et les traits]].
