---
id: 21554a21-affb-49bb-8f50-775ad65ed469
parent: 9d5926e8-6bca-4185-8d82-4502247057ca
order: 2
tags:
- philosophie
- concurrence
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Concurrence sans peur (fearless concurrency)

Les mêmes règles d'[[Ownership et borrowing]] qui empêchent les bugs
mémoire empêchent aussi, par construction, les accès concurrents non
synchronisés : les traits `Send` (une valeur peut être transférée à un
autre thread) et `Sync` (une référence peut être partagée entre
threads) sont vérifiés à la compilation. Un programme qui compile n'a,
par construction, pas de data race — d'où l'expression « concurrence
sans peur » : le parallélisme cesse d'être une source d'angoisse
particulière par rapport au reste du code.
