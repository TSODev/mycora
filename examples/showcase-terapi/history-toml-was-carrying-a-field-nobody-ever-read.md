---
id: 59cd6da8-297b-438f-a1fa-21026d1c2d3d
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 2
tags:
- design-decision
- performance
- bug
created: 2026-07-14T09:21:00Z
updated: 2026-07-14T09:21:00Z
---

# history.toml was carrying a field nobody ever read

`history.toml` had a `response_body` field that was written on every
request and never once read anywhere in the codebase. On one real
23MB history file, that dead weight cost 9.5 seconds to parse
synchronously in `App::new()` — before any UI could even render.
Removing the field dropped that reload to about 20ms.
