---
id: 3c2899f7-c127-4e34-abb4-17bcc11428b3
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 5
tags:
- architecture
created: 2026-07-14T09:10:00Z
updated: 2026-07-14T09:10:00Z
---

# Four entry points, one binary

`main.rs`'s `Cli` (via `clap`) dispatches to one of four modes before
any terminal state is touched, so a parse error always prints cleanly
rather than leaving a half-initialized terminal behind: `--diff <b>
--to text|json` (headless diff), `--diff <b>` alone (the read-only diff
TUI), `--to <fmt>` (headless format conversion), or the main TUI
otherwise. Stdin piped in with no file argument is its own case too —
see [[Stdin-piped input renders to stderr, so stdout stays clean]].
`install_panic_hook()` restores the terminal before the default panic
report prints, installed before raw mode/alternate screen are even
entered.
