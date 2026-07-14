---
id: 30ec1a64-d2ca-4079-b94c-3e0c3eec75de
parent: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
order: 0
tags:
- architecture
created: 2026-07-14T09:05:00Z
updated: 2026-07-14T09:05:00Z
---

# Tabs and the app state machine

`App` holds a `Vec<Tab>`, one per open connection; each `Tab` owns its
own `AppState` enum (`Connection → TableList →
DataGrid/SqlEditor/ErdGraph → EditRecord`, plus `FkGrid`/`SqlResultGrid`)
and one screen struct per state. A keypress flows: `App::run` reads a
key → `Tab::handle_key` matches `self.state` → delegates to the current
screen's `handle_key`, which returns an action enum (`ConnectionAction`,
`DataGridAction`, `SqlEditorAction`, ...) for `Tab::handle_key` to
interpret. Multi-tab sessions (`Ctrl+T`/`[`/`]`/`Ctrl+W`) are a direct
consequence of this shape — a new tab is just a new independent `Tab`
value, nothing shared to coordinate.
