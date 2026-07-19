---
id: 79cb47ba-b230-4656-8366-bc40743e12b4
parent: 2842830d-5650-4b09-8cd1-edc9bf0bb312
order: 2
tags:
- usage
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Systèmes embarqués

Sur microcontrôleur, sans système d'exploitation (`#![no_std]`, qui
retire la dépendance à la bibliothèque standard et donc à toute
allocation implicite), Rust offre les mêmes garanties de sûreté
mémoire qu'en contexte applicatif classique — voir
[[Ownership et borrowing]] — dans un domaine où C reste historiquement
dominant et où les bugs mémoire ont un coût matériel direct.
