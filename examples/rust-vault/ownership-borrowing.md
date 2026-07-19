---
id: e108a7e3-b9b2-47ca-90c2-a740166b031f
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 0
tags:
- langage
- memoire
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Ownership et borrowing

Chaque valeur a un unique propriétaire (*owner*) ; quand celui-ci sort
de portée, la valeur est libérée automatiquement — pas besoin de
`free`/`delete`, ni de ramasse-miettes (voir
[[Sécurité mémoire sans ramasse-miettes]]). On peut *emprunter*
(*borrow*) une valeur sans en prendre possession, via une référence
`&T` (partagée, autant qu'on veut) ou `&mut T` (exclusive, une seule à
la fois) — le compilateur refuse tout code qui violerait cette règle.
Durée de vie de ces emprunts : voir [[Lifetimes]].
