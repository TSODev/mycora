---
id: 8c0c1fd5-f777-495b-a68d-c1301732220f
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 2
tags:
- features
created: 2026-07-14T09:13:00Z
updated: 2026-07-14T09:13:00Z
---

# Authentication

No Auth, Bearer, Basic, API Key (as a header or a query parameter), and
two OAuth2 flows — Client Credentials, and Authorization Code with a
local browser callback. Tokens are cached with their expiry and the
auth configuration is persisted with the request, but the token itself
is never written to disk.
