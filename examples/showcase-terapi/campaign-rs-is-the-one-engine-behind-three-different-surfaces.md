---
id: dec30f14-b73e-4267-ab7a-3d12a473b51a
parent: d24868b2-1d5d-40e2-881b-c1e7f363bcc3
order: 1
tags:
- architecture
created: 2026-07-14T09:06:00Z
updated: 2026-07-14T09:06:00Z
---

# campaign.rs is the one engine behind three different surfaces

`campaign.rs` (~2,500 lines) is where a campaign actually runs: the
TOML schema (`Campaign`/`Step`/`Assertion`/`Transform`), variable
substitution, and one executor per step kind (`run_single_step`,
`run_loop_step`, `run_poll_step`, `run_jq_step`, `run_parallel_step`,
`run_transform_step`, `run_search_step`, ...). `run_streaming()` is the
single execution engine that walks a campaign's steps, resolves
`{{VAR}}` placeholders, dispatches on `kind`, and emits `CampaignEvent`s
over an unbounded channel. Three different consumers read that same
stream: `campaign::run()` for the CLI, `app/campaigns_tab.rs` for the
in-TUI Campaigns tab, and the builder's own single-step preview — which
means a step-execution bug fix in `campaign.rs` fixes all three
surfaces at once, but also that a change there has to be considered
against all three, not just the one currently being worked on.
