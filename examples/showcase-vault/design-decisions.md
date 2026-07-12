---
id: e6fb9f85-2fbf-4f78-8689-d224e904e422
parent: 81bfaab5-4f91-48db-8172-14d25b3f775e
order: 3
tags:
- design-decision
created: 2026-07-10T09:00:00Z
updated: 2026-07-12T12:00:00Z
---

# Design decisions

Specific choices made along the way, and the reasoning
behind each — the "why," not just the "what."

- [[Why a strict tree]]
- [[Fan-out ambiguous wikilinks]]
- [[Read-only secondary vaults]]
- [[Full-pane body editor, save on exit]]
- [[Deferred: configurable keybindings]]
- [[Disposable SQLite index]]
- [[CLI vault management stays registry-only]]
- [[Guard every mutation against the wrong vault]]
- [[Every pane actually scrolls now]]
- [[Folder structure becomes tree structure]]
- [[PDF export renders through a pure-Rust crate]]
- [[Every write to disk is atomic]]
- [[Reindex was quadratic, one missing index fixed it]]
- [[Unmounted vaults are visible too]]

Most of these were resolved as open questions during development, not
decided upfront — see [[Roadmap]] for the versioned history each one is
tied to.
