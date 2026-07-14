---
id: 57ba5e3f-8dc7-4be4-bb69-4ca72f7e4874
parent: d24868b2-1d5d-40e2-881b-c1e7f363bcc3
order: 0
tags:
- architecture
created: 2026-07-14T09:05:00Z
updated: 2026-07-14T09:05:00Z
---

# One binary, four entry points

`main.rs`'s `clap` subcommands dispatch to four independent modes, all
in one crate (not a workspace): plain `terapi` (the interactive TUI,
`ui.rs` + `app/`), `terapi run <campaign.toml>` (the headless campaign
runner, `campaign.rs`), `terapi build [campaign.toml]` (a second,
independent TUI for visually building a campaign, its own
`BuilderApp`), and `terapi import <file>` (Postman/Insomnia collection
or campaign import, auto-detected). The interactive TUI and the
campaign builder don't share an event loop or app struct — two
genuinely separate TUIs living in the same binary, only meeting at the
shared `campaign.rs` engine and `storage.rs`'s file formats.
