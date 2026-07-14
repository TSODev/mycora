---
id: 82f01b3d-5e4c-474e-9379-3e901392305e
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 4
tags:
- architecture
created: 2026-07-14T09:09:00Z
updated: 2026-07-14T09:09:00Z
---

# The plugin system is narrow and compiled-in, on purpose

The `Plugin` trait (`plugin.rs`) is kept deliberately narrow — a
`JNode` in, a string argument, a `JNode` out — because that's exactly
what its one shipped plugin, `jq`, needs. The plugin registry is a
compiled-in `Vec<Box<dyn Plugin>>`, not a dynamic loader, so adding a
plugin today still means adding a Rust module and registering it, not
dropping in a script or a shared library. `jq` itself is bundled via
the pure-Rust `jaq` crate family specifically so no external `jq`
binary is required — keeping the single-binary promise intact even for
this feature.
