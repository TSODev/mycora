---
id: f4e0230e-972f-4732-a402-a23297f9148c
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 3
tags:
- features
created: 2026-07-14T09:14:00Z
updated: 2026-07-14T09:14:00Z
---

# Collections and environments

Collections are one TOML file each, with folders and requests, a
search/filter tree (`/`), duplicate (`D`), edit-in-place, opening the
raw file in `$EDITOR`, and alphabetical sorting. Environments are named
sets of variables with an active-environment indicator and a warning
for any `{{VAR}}` that doesn't resolve; history keeps the last 100
requests, deduplicated and GraphQL-aware — see [[history.toml was carrying a field nobody ever read]] for a real cost this format used to
carry silently.
