---
id: 494a7ddc-6af3-4e0d-b2a3-91ed4b5c7bd8
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 9
tags:
- design-decision
created: 2026-07-14T09:28:00Z
updated: 2026-07-14T09:28:00Z
---

# Campaign progress goes to stderr, campaign data goes to stdout

`terapi run`'s progress messages print to stderr while its actual
result data goes to stdout — so `terapi run campaign.toml --format
json | fx` (or any other downstream consumer) can pipe clean structured
data through while a human watching the same terminal still sees
progress as it happens, rather than having to choose `--silent` to get
a clean pipe.
