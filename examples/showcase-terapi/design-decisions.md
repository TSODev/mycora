---
id: 2423e022-ff5b-4605-b555-48b7500dd11f
parent: 0b1b54d8-5d6a-4bb5-8cb7-fce58c33885e
order: 3
tags:
- design-decision
created: 2026-07-14T09:18:00Z
updated: 2026-07-14T09:18:00Z
---

# Design decisions

Specific choices made along the way, and the reasoning behind them —
mostly surfaced by real bugs or real tradeoffs rather than decided
upfront. The larger structural choices — the four entry points, the
one shared campaign engine, the variable-resolution order — live under
[[Architecture]] instead.

- [[A dead connection pool bug only showed up after a large request]]
- [[A large response body isn't worth trying to render]]
- [[history.toml was carrying a field nobody ever read]]
- [[Ctrl+C is handled unconditionally, before any mode dispatch]]
- [[A panic hook restores the terminal before the report prints]]
- [[A semver-compatible patch release still broke cargo install]]
- [[Converted XML and HTML responses are tagged, so they're never mistaken for the real payload]]
- [[Two env vars for the external differ, because one tool didn't fit the other's contract]]
- [[The external editor is launched directly, not through a shell]]
- [[Campaign progress goes to stderr, campaign data goes to stdout]]
- [[The status bar reuses colors that already meant something]]
- [[There's no dedicated SPARQL mode, on purpose]]
- [[Correctness is verified by running the binary, not a test suite]]

See [[Roadmap]] for the versions each of these landed in.
