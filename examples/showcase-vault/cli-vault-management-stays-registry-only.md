---
id: e3d96cf0-c14f-4a59-a73d-ebead9584ccc
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 6
tags:
- design-decision
- multi-vault
- cli
created: 2026-07-10T20:00:00Z
updated: 2026-07-12T13:00:00Z
---

# CLI vault management stays registry-only

[[Managing vaults from the CLI]] grew one command at a time, each time
raising a real design question explicitly with the user before writing
any code, rather than guessing:

- **`vault remove` never touches files on disk.** It only ever
  unregisters the `config.toml` entry — confirmed up front, not
  discovered as a bug later. Notes are the source of truth
  ([[Markdown as source of truth]]) and the registry is just a pointer
  to them, so "remove" here means "forget," not "delete." The same
  instinct behind [[Full-pane body editor, save on exit]]'s own
  no-destructive-default stance, and `Vault::trash_note` never
  permanently deleting a note either.
- **`vault init` and `vault promote` disagree, on purpose, about what
  happens when a vault named `"default"` already exists.** `init`
  creates and mounts the new vault anyway, reporting honestly that it's
  staying read-only rather than becoming active — because directory
  creation is the point of `init`, and becoming active is a bonus that
  shouldn't block on a name conflict. `promote` *refuses* outright
  instead — because becoming active *is* the entire point of `promote`,
  so silently auto-swapping names to force it through would be touching
  an entry the caller didn't name, the same category of surprise
  [[Fan-out ambiguous wikilinks]] and [[Read-only secondary vaults]]
  both already avoided by not silently guessing. Both questions were
  raised via the same kind of explicit check-in before implementing;
  they just landed differently because the two commands' actual jobs
  differ.
- **`vault unmount` surfaced a real bug, fixed alongside the feature
  that exposed it.** `Config::active_vault`'s self-heal could return a
  vault that `App::new` then failed to find among the ones it actually
  loaded — previously reachable only by hand-editing every registry
  entry to `mounted = false`, but trivial once a command existed to do
  that directly. Fixed rather than shipped as a companion bug: see
  [[Multi-vault mounting]] for what the self-heal itself is for.
- **`vault archive`/`vault unarchive` are the one deliberate exception
  to this whole note's own title.** Every other subcommand only ever
  edits `config.toml`; archiving actually compresses (and then deletes)
  the vault's real directory, because that's the entire point of it —
  a registry-only "archive" that left the uncompressed original sitting
  right there wouldn't have reclaimed anything. See
  [[Compressing a vault trades files for one archive, deliberately]]
  for that decision and the two forks it took to get there.

Every command that writes to `config.toml` shares one implementation:
parse the whole file fresh, mutate in memory, rewrite it — the same
"rewrite rather than diff" instinct as [[Disposable SQLite index]],
just applied to the registry instead of the search index.
