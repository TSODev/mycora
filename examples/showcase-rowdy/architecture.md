---
id: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
parent: 16cc5a85-e964-4b86-8a4a-6aec474e8530
order: 1
tags:
- architecture
created: 2026-07-14T09:04:00Z
updated: 2026-07-14T09:04:00Z
---

# Architecture

Each open connection is its own tab; each tab drives its own state
machine, screens, and async I/O independently.

- [[Tabs and the app state machine]] — Connection → TableList →
  DataGrid/SqlEditor/ErdGraph → EditRecord
- [[Async I/O reports back over a channel, drained every frame]]
- [[Foreign keys open a recursive sub-grid]]
- [[Destructive actions always go through a confirmation modal]]
- [[Credentials resolve through the OS keychain at connect time]]
