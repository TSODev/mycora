---
id: 50086b97-12ff-41e3-9da7-f20acd8211aa
parent: d24868b2-1d5d-40e2-881b-c1e7f363bcc3
order: 4
tags:
- architecture
created: 2026-07-14T09:09:00Z
updated: 2026-07-14T09:09:00Z
---

# External tools are env-var-gated, with sane defaults

Every external tool terapi can hand off to is optional and
environment-variable-controlled, never a hard dependency: `$EDITOR`/
`$VISUAL` for editing request bodies (falling back to `vi`),
`$TERAPI_JSON_EDITOR` for a dedicated JSON editor (falling back to
`jsoned`), and `$TERAPI_JSON_DIFFER`/`$TERAPI_DIFF` for diffing
responses (falling back to `diff -u | less`) — see [[Two env vars for the external differ, because one tool didn't fit the other's contract]]. A bundled `terapi-env.sh` auto-detects `jsoned`/`difft`/
`delta` if they're installed, rather than requiring manual
configuration just to get a better-than-default experience.
