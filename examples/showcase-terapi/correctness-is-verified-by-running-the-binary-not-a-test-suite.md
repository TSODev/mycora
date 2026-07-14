---
id: e8ec9245-5eca-45bd-95c5-6783f3d32aeb
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 12
tags:
- design-decision
- testing
created: 2026-07-14T09:31:00Z
updated: 2026-07-14T09:31:00Z
---

# Correctness is verified by running the binary, not a test suite

CLAUDE.md says this outright, rather than leaving it to be discovered
the hard way: "There are no automated tests in this repo... Do not
assume a test suite exists; verify behavior by running the binary
directly." There's no CI either (no `.github/` directory) — a sharp
contrast with Mycora's own 188 in-crate unit tests, and one of the more
notable differences between the two projects' engineering conventions.
