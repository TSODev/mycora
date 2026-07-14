---
id: cb629638-26f7-453a-b4d7-e6f5e2174653
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 6
tags:
- design-decision
created: 2026-07-14T09:25:00Z
updated: 2026-07-14T09:25:00Z
---

# Converted XML and HTML responses are tagged, so they're never mistaken for the real payload

`xml_convert.rs` turns an XML or HTML response body into a JSON-shaped
tree so the same viewer/extract/diff code paths work on it as on any
JSON response — but the converted tree carries a `FromXML: true`
marker specifically so it's visually obvious this is a converted view,
not the server's actual response format. The conversion is scoped to
the viewer only: campaign `extract`/`assert` steps still parse the raw
response and are unaffected by it.
