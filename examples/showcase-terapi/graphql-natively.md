---
id: 3224a436-e012-4930-8032-000145668c48
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 1
tags:
- features
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T09:12:00Z
---

# GraphQL, natively

`g` toggles GraphQL mode, splitting into Query/Variables/Headers/
Schema/Options/Auth sub-tabs. The query editor gets the same `{{VAR}}`
picker as REST requests, plus `Ctrl+Space` autocompletion; schema
introspection runs two shallow queries (depth ≤3) with a type filter
search and scrollable field detail. Collections save a GraphQL query
and its variables together, and GQL requests get their own magenta
badge throughout the UI so they're visually distinct from REST ones at
a glance.
