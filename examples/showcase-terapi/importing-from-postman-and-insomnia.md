---
id: becbb3bc-19ae-46c4-9c7d-4cd47171c8d2
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 6
tags:
- features
created: 2026-07-14T09:17:00Z
updated: 2026-07-14T09:17:00Z
---

# Importing from Postman and Insomnia

`terapi import <file>` auto-detects and converts a Postman v2.1
collection (folders, requests, auth, bodies, and collection variables
folded into a terapi environment) or an Insomnia v4 export (nested
folders, GraphQL requests, base and sub-environments merged; gRPC/
WebSocket entries are skipped with an explicit warning rather than
silently dropped), as well as auto-detecting whether a file is a
collection or a campaign. Postman's pre/post-request scripts are
explicitly unsupported — the importer reports how many it ignored
rather than pretending to run them.
