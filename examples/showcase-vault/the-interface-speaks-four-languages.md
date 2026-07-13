---
id: a730b274-c34a-4522-86ab-c653d075dfe9
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 4
tags:
- interface
- i18n
created: 2026-07-13T12:00:00Z
updated: 2026-07-13T13:30:00Z
---

# The interface speaks four languages

`language = "fr"` in `config.toml` renders every label, hint, prompt,
and status message in that language — the [[Status bar]]'s hint row,
the [[Command palette]]'s help popup, pane titles, error messages, all
of it. English (`"en"`) is the default; French (`"fr"`), Spanish
(`"es"`), and German (`"de"`) are the others. `:lang fr` switches live
from inside the TUI — the very next frame renders in the new language,
because every string is re-read from the current language on every
draw — and writes the choice back to `config.toml` so it sticks across
restarts. English and French were reviewed carefully; Spanish and
German were added the same afternoon, machine-translated, and are
flagged as not yet reviewed by a native speaker.

What deliberately never translates: keybindings and command syntax.
`:tags limit`, `show`/`hide`, `y/n` — interface *syntax* stays identical
in every language, the way vim's `:w` doesn't translate, so every
keybinding reference and everyone's muscle memory stay valid regardless
of language. Note *content* is yours and untouched either way (the
welcome note auto-created in an empty vault is stamped in whichever
language was configured at the time — it's content, so it stays as
written).

Every language lives inside the binary as compile-checked message
tables, not external language files — a missing translation is a
compile error, not a runtime surprise, and there's nothing extra to
install or keep in sync next to the executable. That trade-off (adding
a language means recompiling) fits the same self-contained instinct as
bundling SQLite — see [[Search and indexing]] — and it's also what made
going from two languages to four cheap: the compiler refuses to build
until every message has an arm for the new language, so nothing gets
silently left in English. An unrecognized `language` code refuses to
start with a clear error rather than silently falling back to English.
