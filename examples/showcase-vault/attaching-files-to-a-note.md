---
id: c929df1d-f2c3-4e3d-9cc9-608604f3c935
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 9
tags:
- features
created: 2026-07-15T09:00:00Z
updated: 2026-07-15T09:00:00Z
---

# Attaching files to a note

`Ctrl+A` while editing a note's body (see
[[Full-pane body editor, save on exit]]) opens a small inline prompt for
a file path — `~/` expands to your home directory. `Enter` copies the
file (never moves it) into `attachments/` at the vault's root,
disambiguating a name collision the same way a brand-new note's
filename already is, and inserts a `![alt](attachments/name.ext)`
Markdown link right at the cursor. `Esc` cancels just the prompt,
leaving the rest of the edit session untouched.

Images and other media are deliberately never rendered inline — see
[[Attachments are copied and linked, never rendered]] for why that's
not an oversight.
