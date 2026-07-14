---
id: aec5476b-3b5d-4aa4-a415-57ecde8b4bcc
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 4
tags:
- design-decision
- bug
created: 2026-07-14T09:23:00Z
updated: 2026-07-14T09:23:00Z
---

# A panic hook restores the terminal before the report prints

Installed before raw mode and the alternate screen are entered, so a
panic mid-session leaves the terminal usable — otherwise it stays in
raw/alternate-screen state until `reset`/`stty sane` is run by hand.
The same fix, for the same reason, jsoned and Mycora both make in
their own `main.rs`.
