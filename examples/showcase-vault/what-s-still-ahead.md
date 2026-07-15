---
id: 63bdb3f5-f8a3-470a-b379-5b864d3a6dff
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 1
tags:
- roadmap
- planned
created: 2026-07-10T09:00:00Z
updated: 2026-07-15T09:30:00Z
---

# What's still ahead

- **v1.0 — Public release**: done. Published to crates.io (0.9.0
  through 0.11.0 so far); `PUBLISH.md` written up as a real release
  checklist. Only "announce, gather feedback, triage into a v1.x
  backlog" is still open
- **v1.1 backlog**: [[Cut, paste, and cross-vault copy]] and
  [[Attaching files to a note]] are both done; still unscheduled —
  concurrent-write safety for a vault shared across two Mycora
  processes or machines, and Windows support (the one real blocker
  found so far: `HOME` read literally in several places, rather than a
  cross-platform crate like `dirs`)
- Optional Postman/Terapi-style templating hooks (stretch goal, may not
  belong in Mycora itself) stay unstarted, same as before

[[Deferred: configurable keybindings]] stays explicitly out of scope
until real friction shows up in practice, rather than being scheduled
into any of the above.
