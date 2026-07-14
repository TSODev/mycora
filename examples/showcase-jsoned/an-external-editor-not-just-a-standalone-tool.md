---
id: 021a8bfd-56ec-435d-a1b6-4e9daf8ecefd
parent: b40834c8-5aa3-404f-a230-61c767857208
order: 0
tags:
- philosophy
created: 2026-07-14T09:02:00Z
updated: 2026-07-14T09:02:00Z
---

# An external editor, not just a standalone tool

A secondary, explicit goal alongside standalone use: being usable as an
external editor from other terminal tools — concretely,
`TERAPI_JSON_EDITOR=jsoned`, since terapi (a sibling TSODev project) can
shell out to any configured JSON editor for its own request/response
bodies. This shaped a real architectural choice: when jsoned is invoked
with input piped on stdin and no file argument, it renders the TUI to
stderr instead of stdout, so stdout stays clean for the
save-and-exit output the calling tool actually wants back. See
[[Stdin-piped input renders to stderr, so stdout stays clean]].
