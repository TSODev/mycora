---
id: 08ff6f56-250c-4cb4-8bfa-8b22cf417fae
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 11
tags:
- design-decision
created: 2026-07-14T09:30:00Z
updated: 2026-07-14T09:30:00Z
---

# There's no dedicated SPARQL mode, on purpose

"A SPARQL endpoint is just an HTTP endpoint, so REST mode already
covers it" — a deliberate scope decision, with the tradeoff named
explicitly rather than glossed over: no schema introspection, no
autocompletion, no SPARQL-aware syntax highlighting. A SPARQL query is
just plain text in the body editor, same as any other REST body.
