---
id: 9c9f3bc5-25ec-4777-aedd-6c469633d045
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 1
tags:
- features
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T09:12:00Z
---

# Connection profiles with encrypted credentials

Profiles are saved in `config.toml` (add/edit/delete from the
Connection screen), with credentials optionally encrypted into the OS
keychain rather than stored in plain text. Pre/post-connect shell hooks
run around the connection itself (a post-connect script waits for the
app to actually exit before running, "for a clean tunnel teardown" —
see [[Disconnect hooks run fire-and-forget, except on the way out]]),
and a `?readonly=true` URL param marks a connection read-only end to
end.
