---
id: 5c340108-f4b7-4b11-800f-b0764ad2e767
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 1
tags:
- design-decision
created: 2026-07-14T09:29:00Z
updated: 2026-07-14T09:29:00Z
---

# Stdin-piped input renders to stderr, so stdout stays clean

When jsoned is invoked with data piped on stdin and no file argument,
it renders the TUI to stderr rather than stdout — so stdout stays clean
for whatever `s` (save-to-stdout-and-exit) eventually writes. This one
small choice is what makes `TERAPI_JSON_EDITOR=jsoned` work at all: a
calling tool can pipe JSON in, let a person edit it interactively, and
read back only the final result on stdout, with the TUI itself never
polluting that stream.
