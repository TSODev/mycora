---
id: 5ab0a46e-f26d-4b3e-9eee-9fd87ff8442a
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 5
tags:
- design-decision
- bug
created: 2026-07-14T09:33:00Z
updated: 2026-07-14T09:33:00Z
---

# A panic hook restores the terminal before the report prints

Installed before raw mode or the alternate screen are even entered, so
a panic mid-session restores the terminal first — otherwise a crash
used to leave the terminal broken (raw mode, alternate screen, both
still active) until running `reset`/`stty sane` by hand. The same
pitfall, and the same fix, that Mycora's own `main.rs` documents for
itself.
