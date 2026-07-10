---
id: d8b6fd32-36c4-4c30-9588-fb20a4c9c2c1
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 6
tags:
- features
- theming
- v0.7
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Theming

Every explicit color in the app is a **named** ANSI color
(`Blue`, `Cyan`, `Yellow`, `Red`, `Green`, `Gray`, ...), not RGB or a
256-color index — with one deliberate exception: the [[Status bar]]'s
background, kept as an explicit indexed color to match the same
convention used by Mycora's sibling terminal tools.

Named colors are mapped by the terminal emulator itself, according to
whatever scheme it's configured with (light, dark, Solarized, ...) — so
light/dark support "just works" without Mycora needing its own
theme-switcher or config option.

The split-pane borders (see [[Layout]]) carry a bit of deliberate color on
top of that baseline: tree = blue, body preview = magenta, backlinks =
cyan only when focused — chosen to avoid colors that already carry other
meaning elsewhere (cyan = focused/active, yellow = confirmation prompts,
red = errors, green = markdown code).
