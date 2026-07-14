---
id: 6732733d-5f51-4537-a635-84da02a39ad7
parent: d24868b2-1d5d-40e2-881b-c1e7f363bcc3
order: 3
tags:
- architecture
created: 2026-07-14T09:08:00Z
updated: 2026-07-14T09:08:00Z
---

# Async work reports back over per-concern channels

A single `#[tokio::main]` runs the whole binary, but the TUI's own
event loop stays synchronous — every async operation (`tokio::spawn`)
reports its result back over its own `mpsc::UnboundedReceiver`
(`response_rx`, `schema_rx`, `campaign_rx`, `oauth2_rx`,
`step_preview_rx`), drained once per frame. A slow request in flight
never blocks the rest of the UI from redrawing or accepting input,
without needing the render loop itself to become async.
