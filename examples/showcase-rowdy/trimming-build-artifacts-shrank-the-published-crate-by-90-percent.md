---
id: ab2a8e39-5c15-4406-99d0-c2cba9d2f051
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 12
tags:
- design-decision
created: 2026-07-14T09:33:00Z
updated: 2026-07-14T09:33:00Z
---

# Trimming build artifacts shrank the published crate by 90 percent

Adding `*.cast`/`*.pdf` to both `.gitignore` and Cargo's `exclude` list
took the published crate from 1.96 MB down to 178 KB compressed — a
reminder to check what actually ends up in a `cargo package` tarball,
not just what's in the git repo.
