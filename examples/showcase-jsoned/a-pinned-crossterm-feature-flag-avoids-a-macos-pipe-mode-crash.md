---
id: 46a72e71-e449-4cc2-b4cf-9a7ae90dbb95
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 2
tags:
- design-decision
- bug
created: 2026-07-14T09:30:00Z
updated: 2026-07-14T09:30:00Z
---

# A pinned crossterm feature flag avoids a macOS pipe-mode crash

`crossterm` is pinned with `features = ["use-dev-tty"]` — not a
cosmetic choice: without it, the default `mio` backend fails to
register `/dev/tty` with `kqueue` when stdin is a pipe, crashing pipe
mode outright on macOS. This is exactly the mode [[Pipe mode]] and
[[Stdin-piped input renders to stderr, so stdout stays clean]] depend
on, so the flag is load-bearing for both.
