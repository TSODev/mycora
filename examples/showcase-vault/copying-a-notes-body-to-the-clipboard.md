---
id: a938d9e1-b835-43f1-8374-48c36a2ff960
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 15
tags:
- features
- interface
created: 2026-07-19T09:00:00Z
updated: 2026-07-19T09:00:00Z
---

# Copying a note's body to the clipboard

`Y` copies the selected note's raw body (the Markdown source, not the
rendered preview) to the system clipboard. Raised as a gap: selecting
just the body preview column with the mouse also grabs the tree/
backlinks panes next to it (see [[Layout]]), since a plain terminal
drag-select works by row across the whole terminal width, not by pane.

Implemented via an OSC 52 escape sequence written straight to stdout
rather than an OS-level clipboard crate — `arboard` and similar need
direct X11/Wayland access, and don't work over a bare SSH session the
way OSC 52 does, since it's the *client*-side terminal that intercepts
the sequence, not the remote shell. A small hand-rolled base64 encoder
backs it (RFC 4648, padded) — no new dependency for something this
small and stable, with a known-vector test for confidence.

Tmux-aware: detected via the `TMUX` environment variable (the same
check every other OSC 52 tool uses), the sequence gets wrapped in
tmux's own DCS passthrough — otherwise tmux swallows an arbitrary
escape sequence from the program it's running rather than forwarding
it to the real terminal underneath.

A status message confirms the copy; an empty body reports there's
nothing to copy instead of silently doing nothing.
