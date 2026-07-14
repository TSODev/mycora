---
id: 42845091-a7e6-4d95-b367-2150564af822
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 1
tags:
- architecture
- performance
created: 2026-07-14T09:06:00Z
updated: 2026-07-14T09:06:00Z
---

# The patch pattern: refresh_at, not a full rebuild

`App` (`src/app/mod.rs`, the largest file in the crate) owns the
`JNode` root plus three structures derived from it: `flat: Vec<FlatRow>`
(drives the Explorer panel and cursor), `annotated: Vec<AnnotatedLine>`
(drives the Source panel), and `lint_warnings: Vec<LintWarning>`. Every
mutating method follows the same shape: push an undo entry, mutate
`self.root` at one path, then call `refresh_at(target,
affects_annotated)` — which splices only the contiguous block belonging
to that path's subtree (the flat list is a pre-order depth-first
traversal, so a subtree is always contiguous) rather than rebuilding
the three derived structures from scratch. Measured effect: a single
edit on a 350,000-row file went from 1,375ms to about 0.1ms — see
[[Performance]]. Two navigation actions (`jump_to_lint`,
`expand_ancestors`) are deliberately excluded from patching instead of
silently approximated, since un-collapsing several non-contiguous
ancestors in one call has no single obvious patch target.
