---
id: ab538b3a-fb41-4132-b401-b2991c9da5d0
parent: 36aea2c6-8ed7-4881-88df-315e4a1423c4
order: 0
tags:
- philosophy
created: 2026-07-14T09:02:00Z
updated: 2026-07-14T09:02:00Z
---

# Not Postman, not just another curl wrapper

Every alternative the README names solves one slice of the problem:
ATAC is a REST TUI with nothing for GraphQL or scripting; hurl scripts
well but has no interactive mode; HTTPie is terminal-based but isn't a
TUI at all; Postman and Insomnia do all of the above, at the cost of
being Electron apps that want a cloud account. Terapi's pitch is
"GraphQL native — schema introspection, variable editing, collections
save/load" plus "pipeline automation — chain requests, extract
variables, run campaigns headlessly" plus "single binary — `cargo
install terapi`, instant startup, zero Electron," in one tool instead
of stitched together from several.
