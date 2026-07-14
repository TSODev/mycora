---
id: 7e5f51a3-dfb6-422e-9668-c0f95cde725a
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 1
tags:
- design-decision
- performance
created: 2026-07-14T09:20:00Z
updated: 2026-07-14T09:20:00Z
---

# A large response body isn't worth trying to render

Benchmarking a 3.4MB response found `Paragraph::render`'s line-wrap
pass alone costing ~291ms, against ~93ms just to tokenize the same
body for the JSON view — and stripping color only clawed back about
30% of that. Past a `LARGE_BODY_THRESHOLD` of 1MB, the Raw and HTTP
views show a notice pointing at `r` (open in `$EDITOR`) or `E` (open in
the external JSON editor) instead of attempting to render inline; the
JSON tree view itself stays fast regardless, since it's cached and
windowed rather than laid out fresh every frame.
