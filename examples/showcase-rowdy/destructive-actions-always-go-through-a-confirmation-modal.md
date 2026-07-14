---
id: fc9fa531-2d2b-45a5-b7ed-4b630be693b5
parent: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
order: 3
tags:
- architecture
created: 2026-07-14T09:08:00Z
updated: 2026-07-14T09:08:00Z
---

# Destructive actions always go through a confirmation modal

Anything that mutates or deletes data goes through a `PendingAction`
held until a confirmation `Modal` fires on `y`/`Enter`, handled by a
dedicated `handle_modal_key`. Record edits go one step further and show
the literal UPDATE statement about to run before you confirm it, not
just a generic "are you sure?".
