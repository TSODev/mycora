---
id: 78eda13d-d65a-4ec1-bff0-d2b105c53d43
parent: 9d5926e8-6bca-4185-8d82-4502247057ca
order: 1
tags:
- philosophie
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Abstractions à coût nul

« Zero-cost abstractions » : écrire du code de haut niveau (itérateurs,
closures, `async`/`await`) ne doit rien coûter de plus, une fois
compilé, qu'une version bas niveau écrite à la main. Un `.iter().map().
filter().sum()` se compile essentiellement vers la même boucle qu'une
version impérative — voir la [[Généricité]] et le
[[Le système de types et les traits]], qui rend cette généricité
statique (résolue à la compilation) plutôt que dynamique.
