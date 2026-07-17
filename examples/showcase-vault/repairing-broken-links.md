---
id: e9d2d5dd-a657-44a5-8182-429b59b11aa6
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 13
tags:
- features
- links
- cli
created: 2026-07-17T09:00:00Z
updated: 2026-07-17T10:30:00Z
---

# Repairing broken links

[[Cross-links and backlinks]] already reports a broken link (a
wikilink title matching no note) via `mycora reindex`'s warnings — but
does nothing about it. Two ways to actually fix one: `mycora repair`, a
headless CLI for batch fixes; and `:brokenlinks`, the TUI's own
review-one-at-a-time overlay (below). `mycora repair` has three tiers
from safest to most invasive:

```sh
mycora repair                  # report only — the safe default
mycora repair --create-stubs   # + create a stub note for unmatched links
mycora repair --apply          # + retarget confidently-matched links
mycora repair --vault <name>   # narrow reporting/fixing to one vault
```

With no flags, it only reports — every broken link across every
mounted vault, with a best-guess suggestion where one exists:

- **Case difference** — Mycora's own title matching is case-sensitive,
  so a lowercased link next to a note titled with a capital (e.g.
  "commandes" vs. "Commandes") is a very common real cause of a broken
  link.
- **Similar title** — otherwise, a close-enough fuzzy match
  (`strsim::jaro_winkler` on lowercased titles) against another note's
  title. Two notes close enough to be ambiguous, or nothing close at
  all, gets no suggestion — `repair` never guesses when it isn't
  reasonably sure.

This default run changes nothing — it's also exactly the preview of
what `--apply` would do. `--create-stubs` creates an empty note for
every broken link with *no* plausible suggestion, one per distinct
missing title per vault, always safe since it only ever adds a note.
`--apply` is the one flag that edits an existing note: it rewrites a
confidently-matched broken link's text in place. There's no undo for
this outside your own backups or version control — unlike everything
the TUI itself does, a CLI run never touches the undo stack, so `repair`
without `--apply` first is the way to preview a fix before committing
to it.

`strsim` was already a transitive dependency (`clap`'s own "did you
mean" argument suggestions) before this — promoted to a direct one
rather than hand-rolling similarity scoring from scratch, since it
added no new compiled code to the binary.

`:brokenlinks` in the [[Command palette]] is the TUI's own take on the
same idea — a full-pane list of the same broken links with the same
suggestions, `j`/`k` to move, `Enter` to jump. Rather than trusting an
automated `--apply`, this is for reviewing and fixing them one at a
time: `Enter` jumps to the link's source note *and* scrolls the body
preview near the broken text itself (not just the top of the note), so
`e` opens the body editor already at the right spot — an ordinary
manual edit, no special mechanism, then `Esc` saves like any other
change. `Ctrl+O` afterward returns to wherever the jump started from,
same as [[Navigation history]] everywhere else.
