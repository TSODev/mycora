---
id: b9ebf7e1-bda8-409d-a9cf-a600abbc864f
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 2
tags:
- features
created: 2026-07-14T09:13:00Z
updated: 2026-07-14T09:13:00Z
---

# Multi-tab sessions with auto-reconnect

`Ctrl+T` opens a new tab, `[`/`]` switch between them, `Ctrl+W` closes
one — each tab is a fully independent connection (see [[Tabs and the app state machine]]). A dropped connection retries automatically up to
3 times with exponential backoff (1s → 2s → 4s), showing a
`RECONNECTING…` badge rather than just failing the next query.
