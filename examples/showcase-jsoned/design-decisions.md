---
id: cc955771-76eb-4c7e-af57-38e84c4d4224
parent: 599f041a-3fa6-4fdd-b145-8fb8f7e8a8ef
order: 4
tags:
- design-decision
created: 2026-07-14T09:27:00Z
updated: 2026-07-14T09:27:00Z
---

# Design decisions

Specific choices made along the way, and the reasoning behind them —
grouped here are the ones that aren't really "architecture" so much as
a deliberate tradeoff or a bug that revealed one. The larger structural
choices — the tree model, the patch-based performance pattern, why
undo/redo doesn't (yet) share in it, the plugin system's shape — live
under [[Architecture]] instead, since they're as much "what" as "why."

- [[JSONL is just one JSON value per line]]
- [[Stdin-piped input renders to stderr, so stdout stays clean]]
- [[A pinned crossterm feature flag avoids a macOS pipe-mode crash]]
- [[Ctrl+C is checked first in every mode, not just Normal]]
- [[No criterion: manual timed tests instead]]
- [[A panic hook restores the terminal before the report prints]]
- [[The keymap cheat sheet reuses jsoned's own accent colors]]

See [[Roadmap]] for the versions each of these landed in.
