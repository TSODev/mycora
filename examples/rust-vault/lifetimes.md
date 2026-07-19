---
id: 5b790f72-ee90-4f6c-8b7d-33313f4ac730
parent: a6a932af-4f5e-486c-b73b-7f21a7cce68c
order: 1
tags:
- langage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Lifetimes

Une *lifetime* (durée de vie) est la façon dont Rust nomme, quand c'est
ambigu, combien de temps une référence reste valide — annotée `'a` dans
les signatures de fonctions ou de structures qui stockent des
références. La grande majorité du temps, le compilateur les infère
seul ; elles ne se voient explicitement que dans les cas où plusieurs
références en jeu pourraient avoir des durées de vie différentes. C'est
la mécanique qui rend l'[[Ownership et borrowing]] vérifiable
statiquement plutôt que simplement conventionnelle.
