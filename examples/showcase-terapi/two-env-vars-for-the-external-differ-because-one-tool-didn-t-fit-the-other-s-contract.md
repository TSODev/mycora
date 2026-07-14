---
id: c6b7f4e9-26c5-4a44-8221-0d7d3cbd2d6d
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 7
tags:
- design-decision
created: 2026-07-14T09:26:00Z
updated: 2026-07-14T09:26:00Z
---

# Two env vars for the external differ, because one tool didn't fit the other's contract

`$TERAPI_DIFF` already existed as a two-bare-positionals contract
(`$TERAPI_DIFF file1 file2`), which some diff tools just don't fit —
`jsoned <file> --diff <file2>` needs its own flag, not two positional
arguments. Rather than force every future tool into one contract,
`$TERAPI_JSON_DIFFER` was added alongside it (checked first, with
`$TERAPI_DIFF` as the fallback), leaving room for tools with different
calling conventions instead of the older variable's shape becoming a
ceiling on what could be plugged in.
