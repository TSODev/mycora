---
id: 37ba71d8-8709-4a98-908f-f73a5233053f
parent: 79497d15-2996-481e-94ec-327ccb81d108
order: 0
tags:
- features
created: 2026-07-14T09:11:00Z
updated: 2026-07-14T09:11:00Z
---

# Requests and the response viewer

REST requests (GET/POST/PUT/PATCH/DELETE) with auto-parsed/reconstructed
URL parameters, a headers picker (common headers plus a Content-Type
value picker), a dual-mode body editor (Text or JSON), and `{{`
variable autocompletion in any field. Responses get a JSON tree
(fold/unfold), a Raw view, and a full HTTP wire view (redirects,
cookies, diagnostics); `/` searches the response, an extraction path
bar pulls a value out with the same dot-path language used by
campaigns, `f` follows a URL found in the response, `d` opens an
external diff against a previous response, and `E` opens the body in
an external JSON editor. Per-request options cover Skip TLS, Follow
redirects, a timeout preset, and a cookie jar — backed by one
persistent `reqwest::Client` (see [[A dead connection pool bug only showed up after a large request]]) with a `User-Agent: terapi/<version>`
header auto-injected. There's no dedicated SPARQL mode — see [[There's no dedicated SPARQL mode, on purpose]].
