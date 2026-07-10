---
id: 63035869-c82d-46d6-a0a7-ca6ee3417e2e
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 1
tags:
- design-decision
- links
- v0.5
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Fan-out ambiguous wikilinks

Note titles aren't required to be unique. When a
wikilink title matches more than one note, Mycora resolves it to
**every** match rather than just the first one found, and rather than
treating the ambiguity as a broken link.

This was an open design question, resolved deliberately: silently picking
"the first match" would be guessing on the user's behalf with no way to
tell which note actually got linked, and treating any duplicate title as
broken would punish something that's otherwise completely allowed (see
[[Multi-vault mounting]] — the same "don't silently guess" instinct
shaped that decision too). Fan-out keeps every plausible target reachable
via the backlinks panel, without hiding the ambiguity.

See [[Cross-links and backlinks]] for how resolution otherwise works
(broken links, self-links, cross-vault resolution).
