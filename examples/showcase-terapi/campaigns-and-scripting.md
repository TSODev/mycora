---
id: 1e8109db-62a6-4448-93d5-1c4ff742cbad
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 4
tags:
- features
created: 2026-07-14T09:15:00Z
updated: 2026-07-14T09:15:00Z
---

# Campaigns and scripting

A campaign is a sequence of steps of different kinds — `http`,
`graphql`, `transform`, `seed`, `loop`, `poll`, `search`, `set`, `jq`,
`parallel`, `notify`, `build`, `file`, `pause`, `comment` — run by the
single engine described in [[campaign.rs is the one engine behind three different surfaces]]. TOML's `params` table-array (written with TOML's own doubled-bracket array-of-tables syntax) declares inputs with defaults,
overridable per run with CLI `-p` flags or a TUI form; `when`
conditionals (`eq`/`ne`/`exists`) gate a step; assertions
(`eq`/`ne`/`lt`/`lte`/`gt`/`gte`/`in`/`exists`/`contains`/`matches`)
verify a response; `foreach` repeats a step over a wildcard extraction,
injecting `item_N`/`item_field` variables per iteration. Input
connectors (CSV, a JSON file, or a JSON produced by an earlier `seed`
step) drive data-driven runs; output connectors chain a campaign's
results into another step, optionally including extracted variables.
`terapi run` supports `--only` (a subset of steps), `--format
json|csv`, `--retry N` (exponential backoff), and `--silent`, with
progress and data kept on separate streams — see [[Campaign progress goes to stderr, campaign data goes to stdout]].
