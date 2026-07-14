---
id: 25664df4-9fa6-48e9-82a8-04db8e38d495
parent: 0b1b54d8-5d6a-4bb5-8cb7-fce58c33885e
order: 4
tags:
- roadmap
created: 2026-07-14T09:32:00Z
updated: 2026-07-14T09:32:00Z
---

# Roadmap

No separate forward-looking roadmap document exists for terapi (no
`ROADMAP.md` the way Mycora or jsoned keep one) — this is
`CHANGELOG.md`'s version history, read as a timeline, plus the open
items its own docs already name explicitly.

Latest released: **0.10.10**; `[Unreleased]` currently holds a status
bar redesign (see [[The status bar reuses colors that already meant something]]) and an import-fixture fix. Recent trajectory: 0.10.9 (XML/
HTML response handling, plus a roughly 65x typing-speed fix), 0.10.8
(`$TERAPI_JSON_DIFFER`), 0.10.6 (a GraphQL Auth tab, schema filtering),
0.10.5 (the external viewer, follow-URL), 0.10.3 (rate limiting,
built-in variables), 0.10.0 (`loop`/`build`/`poll`/`set`/`jq`/
`parallel`/`notify` step kinds — the single largest expansion of the
campaign engine), 0.9.x (Postman/Insomnia import, `--only`/`--format`/
`--retry`, the search step), 0.8.0 (`terapi build` shipped), 0.7.x
(OAuth2, the redirect chain/cookie jar, `foreach`), 0.6.x (the
Campaigns TUI tab, assertions, connectors), 0.5.0 (GraphQL support
shipped), down through 0.1.0-0.3.0's REST foundation.

Explicitly open, by the project's own docs: no automated test suite
and no CI (see [[Correctness is verified by running the binary, not a test suite]]); the `time`/`cookie` dependency pin as a live trap to
re-check on any future `cargo update` (see [[A semver-compatible patch release still broke cargo install]]); and Postman's pre/post-request
scripts, explicitly unsupported on import.
