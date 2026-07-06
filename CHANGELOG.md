# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Added
- **Markdown persistence (v0.2)** — notes now survive a restart as one
  Markdown file per note in a flat vault directory: YAML frontmatter
  (`id`, `parent`, `order`, `tags`, `created`, `updated`) plus a leading
  `# H1` for the title. Every create/rename/delete writes through
  immediately (atomic: temp file + rename), no explicit save step.
  `NoteId` is now a UUID v4 generated at creation — replaces v0.1's
  in-memory `usize` counter, resolving the note-identity open design
  question. Malformed files, duplicate ids, and notes with an
  unresolvable parent are self-healed and reported as warnings on
  load rather than causing a crash or silent data loss. Vault path is
  configurable via `~/.config/mycora/config.toml`.
- **Test-vault generator** (`examples/generate-test-vault.rs`) — builds
  a synthetic vault (category → sub-category → leaf notes, random tags,
  `[[wikilink]]` cross-references) for TUI load-testing. Reuses
  `mycora::vault::Vault` directly so its output is guaranteed to match
  the app's real on-disk format. Split the crate into `lib.rs` + `main.rs`
  to make this possible.

---

## [0.1.0] — 2026-07-06

### Added
- **In-memory tree skeleton (v0.1)** — initial `Note`/`Tree` core model
  (create, rename, delete with child promotion to the deleted note's
  parent), a minimal ratatui TUI shell with vim-inspired normal/insert
  modal input, and single-pane tree navigation with expand/collapse.
  No persistence yet — notes exist only for the process lifetime.
- Published to [crates.io](https://crates.io/crates/mycora), dual-licensed
  MIT OR Apache-2.0.
