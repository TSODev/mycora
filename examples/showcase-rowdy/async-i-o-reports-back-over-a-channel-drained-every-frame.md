---
id: e26b3078-e6f4-4415-8ec1-41d1bd3ce477
parent: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
order: 1
tags:
- architecture
created: 2026-07-14T09:06:00Z
updated: 2026-07-14T09:06:00Z
---

# Async I/O reports back over a channel, drained every frame

Every database operation is async: `spawn_*` methods `tokio::spawn` a
task that sends a `DbEvent` back over an mpsc channel
(`Tab::db_tx`/`db_rx`). The main loop drains `db_rx` for every tab,
every frame — so a slow query on one tab's connection never blocks
input or rendering on another. Auto-reconnect (3 retries, exponential
backoff 1s → 2s → 4s, with a `RECONNECTING…` badge) is built on the
same mechanism: a dropped connection is just another event on the same
channel.
