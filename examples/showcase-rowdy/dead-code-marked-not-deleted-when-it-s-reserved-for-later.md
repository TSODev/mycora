---
id: f5d5c9f0-9696-456e-afdd-3e144496dd2c
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 11
tags:
- design-decision
created: 2026-07-14T09:32:00Z
updated: 2026-07-14T09:32:00Z
---

# Dead code marked, not deleted, when it's reserved for later

Rather than deleting unused-but-intentional API surface — fields and
methods clearly reserved for planned future work, like
`Column::type_name` or several `KvClient` methods (`disconnect`, `get`,
`set`, `del`) — they're marked `#[allow(dead_code)]` instead of
removed, keeping the shape of a trait or struct honest about where
it's headed even before every part of it has a caller yet.
