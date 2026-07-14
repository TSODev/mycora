---
id: ca474e80-ef72-4d43-8c70-f563245c517c
parent: 36aea2c6-8ed7-4881-88df-315e4a1423c4
order: 1
tags:
- philosophy
created: 2026-07-14T09:03:00Z
updated: 2026-07-14T09:03:00Z
---

# Local-first: TOML collections, no cloud account

Collections, environments, history, and campaigns are all plain TOML
files under a resolved `TERAPI_DIR` (checked in this order: the env var
itself, then `./.terapi/`, then `~/.config/terapi/`) — git-friendly by
construction, diffable in a pull request, and requiring no account or
network round-trip just to open the app. This is also what makes
campaigns portable: a `campaign.toml` is just a file, shareable the
same way any other text file in a repo is.
