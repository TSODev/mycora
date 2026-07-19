---
id: aac4e8bb-5cac-4d6d-aaa3-d0b0d4831b4d
parent: 9d5926e8-6bca-4185-8d82-4502247057ca
order: 0
tags:
- philosophie
- memoire
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Sécurité mémoire sans ramasse-miettes

Le *borrow checker* de Rust vérifie à la compilation qu'aucune donnée
n'est utilisée après sa libération, qu'aucune référence ne survit à ce
qu'elle pointe, et qu'aucune valeur n'a deux propriétaires mutables en
même temps. Ces règles reposent sur l'[[Ownership et borrowing]] et
suffisent, dans l'immense majorité du code, à éliminer toute une
classe de bugs (use-after-free, double-free, data races) sans le coût
en temps d'exécution d'un ramasse-miettes.
