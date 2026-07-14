---
id: 41d02d61-ba49-461b-8cc1-47e13647998e
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 3
tags:
- design-decision
created: 2026-07-14T09:22:00Z
updated: 2026-07-14T09:22:00Z
---

# Ctrl+C is handled unconditionally, before any mode dispatch

Raw mode disables the terminal's own SIGINT generation, so `Ctrl+C`
being checked before any mode-specific key handling is the only true
emergency-quit path left — deliberately bypassing modals, confirmation
prompts, and any "press twice to quit" convention rather than getting
tangled up in whichever mode happens to be active.
