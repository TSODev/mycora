---
id: 6adb85bb-1b89-44c5-8d98-1ffb5b1f9f01
parent: baac7ee6-7144-45c6-8443-160c8f053f51
order: 0
tags:
- features
- multi-vault
- cli
created: 2026-07-10T20:00:00Z
updated: 2026-07-11T09:30:00Z
---

# Managing vaults from the CLI

Every operation on the [[Multi-vault mounting]] registry is also
available as a `mycora vault <subcommand>` shell command, instead of
requiring hand-edits to `config.toml`. These run *before* the TUI
starts — a separate thing from the [[Command palette]]'s `:` commands,
which run *inside* an already-open Mycora against the active vault.

- `vault add <name> <path> [--no-mount]` — registers an entry (mounted
  by default). Doesn't create the directory itself; errors on a
  duplicate name; migrates a legacy single-vault `vault_path` config
  into an explicit `"default"` entry first if that's all there was.
- `vault init <name> <path>` — creates the vault directory *and*
  registers it, always mounted, then reports honestly whether it
  actually became the active (read-write) vault — that only happens if
  it ends up named `"default"`.
- `vault rename <old> <new>` — renames a registry entry in place; path
  and mount state are untouched.
- `vault promote <name>` — makes a vault active by renaming it to
  `"default"`. Unlike `init`, `promote` *refuses* outright if a
  different vault already holds that name, rather than creating
  anything and reporting the conflict afterward.
- `vault mount <name>` / `vault unmount <name>` — toggle the `mounted`
  flag directly, without removing the entry.
- `vault remove <name>` — unregisters an entry. Never touches the
  vault's files on disk, and refuses outright on `"default"`.
- `vault list` — prints every registered vault, its path, and
  `[active, mounted]`-style status tags.

See [[CLI vault management stays registry-only]] for why `init` and
`promote` land on opposite answers to the same "what if `default`
already exists" question, and why none of these commands ever touch a
vault's Markdown files.

`mycora vault ...` isn't the only thing the CLI does beyond the TUI —
`mycora reindex` (see [[Search and indexing]]) and `mycora export`
(see [[Exporting a subtree]]) are top-level commands of their own, not
`vault` subcommands, each with an in-TUI `:` equivalent
(`:reindex`/`:export`) for when Mycora's already open. `mycora import`
(see [[Importing an Obsidian vault]]) is the odd one out: CLI-only,
same as `vault` itself — there's no `:import` any more than there's a
`:vault`, since both always create a brand new vault from scratch
rather than acting on whatever's already open.
