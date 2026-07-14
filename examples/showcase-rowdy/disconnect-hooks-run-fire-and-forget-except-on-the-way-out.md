---
id: 350a2c17-fd6e-4009-8eca-aa0a61075d9e
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 7
tags:
- design-decision
created: 2026-07-14T09:28:00Z
updated: 2026-07-14T09:28:00Z
---

# Disconnect hooks run fire-and-forget, except on the way out

A configured post-disconnect script runs fire-and-forget when you
simply return to the connection screen — no reason to make the UI wait
on it. But on the way out of the app entirely (`Ctrl-C`/`q`), that same
script is awaited before exit, "for a clean tunnel teardown" — so an
SSH tunnel or similar has a chance to close properly instead of being
killed mid-command. Pre-connect scripts get the opposite treatment: a
non-zero exit code doesn't block the connection attempt, since the
tunnel it's supposed to open might already be up.
