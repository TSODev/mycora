---
id: fdc0be65-34ec-4b29-b040-532777b67fa5
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 1
tags:
- features
created: 2026-07-14T09:13:00Z
updated: 2026-07-14T09:13:00Z
---

# Editing scalars with type conversion

Any scalar value can be edited in place, including converting its type
outright — string to number, number to boolean, anything to null, and
back — rather than only ever editing within one type. The Edit mode's
type dropdown is deliberately restricted to scalar types only, to
prevent accidental container destruction — you can't accidentally turn
a value into an empty object or array through the same picker.
