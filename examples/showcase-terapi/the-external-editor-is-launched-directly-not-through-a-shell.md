---
id: 9fc20f2f-8517-4f13-afad-ce11ede3797f
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 8
tags:
- design-decision
created: 2026-07-14T09:27:00Z
updated: 2026-07-14T09:27:00Z
---

# The external editor is launched directly, not through a shell

`$EDITOR`/`$TERAPI_JSON_EDITOR` is launched via `Command::new` rather
than `sh -c "$EDITOR file"` — preserving TTY inheritance, which matters
for a TUI editor like `jsoned` to work at all when launched from
inside another TUI. The `sh -c` path is only used as a fallback, and
only when the configured editor string actually contains shell
metacharacters that need a shell to interpret.
