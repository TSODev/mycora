---
id: f15071c2-383a-40ba-91ee-4e46307c77d1
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 3
tags:
- architecture
created: 2026-07-14T09:08:00Z
updated: 2026-07-14T09:08:00Z
---

# Diff gets its own read-only app

`diff.rs` computes a structural, key-path diff; `diff_app.rs` wraps it
in a separate `DiffApp` that deliberately doesn't share the main `App`'s
edit/undo/search machinery — keeping diff mode simple and, more
importantly, safe: neither file being compared can accidentally be
mutated by a diff session. `diff.rs`'s array comparison is naive
index-alignment rather than a real LCS diff in this first version,
documented plainly as a known limitation: inserting an element at the
front of an array makes every later element show up as "changed."
