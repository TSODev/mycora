---
id: 65cb0f34-4e9b-40b3-a078-2a3965a6fe90
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 14
tags:
- design-decision
- multi-vault
- cli
created: 2026-07-12T13:00:00Z
updated: 2026-07-12T14:00:00Z
---

# Compressing a vault trades files for one archive, deliberately

`mycora vault archive <name>` / `mycora vault unarchive <name>` grew
out of the same conversation that produced
[[Unmounted vaults are visible too]] — once an unmounted vault had a
place in the tree, "compress one down to reclaim disk space" and
"encrypt one at rest" were both floated as further vault states. Encryption (`vault lock`/`vault unlock`) was
**abandoned outright**, on the user's own call: passphrase handling, key
derivation, and the fallout of getting authenticated encryption subtly
wrong are a lot of security surface for a note-taking tool to carry,
with no recovery path if a passphrase is lost — a vault is just a
directory of Markdown files, so it already encrypts cleanly with
existing, audited tools entirely outside Mycora (LUKS, VeraCrypt, `age`).

Archiving *was* built, after two forks confirmed up front rather than
guessed at:

- **Format**: `tar.gz` (new `tar`/`flate2` dependencies, both
  pure-Rust — `flate2` defaults to its `miniz_oxide` backend, no system
  zlib needed) over `zip`. A vault is many small, textually similar
  Markdown files; tar.gz's solid, streaming compression genuinely does
  better on that shape of data than zip's per-file-independent
  compression — and it's what Cargo itself uses for packaging crates,
  the same "many small text files" case.
- **What happens to the original**: deleted, not kept alongside as a
  redundant copy — but only *after* `archive::verify_archive` re-opens
  the freshly written archive and reads every entry back, so a corrupt
  or truncated archive is caught while the original still exists rather
  than discovered only once it's already gone.

`VaultEntry` gained `archived: Option<PathBuf>` — the archive file's
location when archived, while `path` keeps meaning "where the live
directory is or would be," so unarchiving always knows where to restore
to regardless of how long a vault stayed archived. Deliberately doesn't
check "is this vault mounted" inside `Config` itself: that precondition
is checked by `main.rs`'s orchestration *before* the actual compression
work runs (archiving something meant to still be live would pull the
rug out from under it — unmount first), not duplicated inside `Config`
after potentially wasting real work on a vault that was never going to
qualify.

Like [[Importing an Obsidian vault]] and [[Exporting a subtree]],
`archive`/`unarchive` are CLI-only with no `:` palette equivalent — see
[[Managing vaults from the CLI]] — since both act on the registry and a
vault's files wholesale, not on anything currently open in the TUI.

An archived vault does now have its own placeholder row — see
[[Unmounted vaults are visible too]] for the icon/color decision
(a distinct `▦`, not `⊘` with a text suffix) and the `:config unmount
show/hide` / `:config archive show/hide` commands built alongside it.
