use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode};

/// `Length(2)` status band at the bottom (see `draw_status`) below the
/// main content — matches Terapi/jsoned's 2-line status bar convention.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(frame.area());

    draw_main(frame, chunks[0], app);
    draw_status(frame, chunks[1], app);
}

/// Full-pane overlays (search, the body editor) take over the whole main
/// area, same as before the split layout. Every other mode — Normal,
/// Insert (renaming), ConfirmDelete (whose prompt is only in the status
/// bar), and Backlinks (which shifts focus onto the backlinks pane rather
/// than opening its own overlay — see `App::focus_backlinks`'s doc
/// comment) — gets the three-pane split: tree, a read-only body preview,
/// and the backlinks list, both following the current selection live.
/// Column widths come from `App::pane_widths` (default 40/40/20,
/// adjustable with `[`/`]`/`{`/`}` — see that method's doc comment).
fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::Search => {
            draw_search(frame, area, app);
            return;
        }
        Mode::EditBody => {
            if let Some(editor) = app.body_editor() {
                frame.render_widget(editor, area);
            }
            return;
        }
        Mode::Normal | Mode::Insert | Mode::ConfirmDelete | Mode::Backlinks => {}
    }

    let widths = app.pane_widths();
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(widths[0]),
            Constraint::Percentage(widths[1]),
            Constraint::Percentage(widths[2]),
        ])
        .split(area);

    draw_tree(frame, panes[0], app);
    draw_body_preview(frame, panes[1], app);
    draw_backlinks_pane(frame, panes[2], app);
}

fn draw_tree(frame: &mut Frame, area: Rect, app: &App) {
    let mut items: Vec<ListItem> = app
        .visible_notes()
        .into_iter()
        .map(|(id, depth)| {
            let note = app
                .tree
                .get(id)
                .expect("visible note ids always resolve in the tree");

            let marker = if app.tree.children(id).is_empty() {
                "  "
            } else if app.expanded.contains(&id) {
                "▾ "
            } else {
                "▸ "
            };
            let indent = "  ".repeat(depth);

            let label = if app.mode == Mode::Insert && app.selected == Some(id) {
                format!("{indent}{marker}{}", app.input)
            } else {
                let collapsed_with_children =
                    !app.expanded.contains(&id) && !app.tree.children(id).is_empty();
                if collapsed_with_children {
                    let count = app.link_count_for(id);
                    if count > 0 {
                        let plural = if count == 1 { "" } else { "s" };
                        format!("{indent}{marker}{} ({count} link{plural})", note.title)
                    } else {
                        format!("{indent}{marker}{}", note.title)
                    }
                } else {
                    format!("{indent}{marker}{}", note.title)
                }
            };

            let style = if app.selected == Some(id) {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    // Other mounted vaults: read-only, so never selectable/navigable (see
    // `App::other_vault_sections`'s doc comment) — just a dimmed separator
    // per vault followed by its top-level roots, always collapsed.
    for (vault_name, roots) in app.other_vault_sections() {
        let dim = Style::default().add_modifier(Modifier::DIM);
        items.push(ListItem::new(Line::from(Span::styled(
            format!("── {vault_name} ──"),
            dim,
        ))));
        for (title, count) in roots {
            let label = if count > 0 {
                let plural = if count == 1 { "" } else { "s" };
                format!("▸ {title} ({count} link{plural})")
            } else {
                format!("▸ {title}")
            };
            items.push(ListItem::new(Line::from(Span::styled(label, dim))));
        }
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Mycora"));
    frame.render_widget(list, area);
}

/// Read-only preview of the selected note's body — plain text, not
/// rendered Markdown (that's a separate, still-open ROADMAP item). Empty
/// when nothing's selected or the note has no body yet.
fn draw_body_preview(frame: &mut Frame, area: Rect, app: &App) {
    let note = app.selected.and_then(|id| app.tree.get(id));
    let title = note.map(|n| n.title.as_str()).unwrap_or("");
    let body = note.map(|n| n.body.as_str()).unwrap_or("");

    let paragraph = Paragraph::new(crate::markdown::render(body))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(paragraph, area);
}

/// List of notes linking to the selected note — follows the current
/// selection live. Doesn't reindex first, same as `App::link_count_for`'s
/// badges: reflects whatever the last reindex resolved (on startup, or the
/// next time `/` is used), not a live-as-you-type view of unreindexed
/// edits. Interactive when `b` shifts focus here (`Mode::Backlinks`): the
/// focused entry is highlighted and the border turns cyan, matching the
/// tree's own selection styling; otherwise it's just a glance list.
fn draw_backlinks_pane(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.mode == Mode::Backlinks;

    let items: Vec<ListItem> = app
        .live_backlinks()
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let style = if focused && i == app.backlinks_selected() {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(hit.title.clone(), style)))
        })
        .collect();

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title("Backlinks"),
    );
    frame.render_widget(list, area);
}

fn draw_search(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .search_results()
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let title_style = if i == app.search_selected() {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let snippet_base = Style::default().add_modifier(Modifier::DIM);
            let snippet_match = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);

            let title_line = Line::from(Span::styled(hit.title.clone(), title_style));
            let snippet_line = Line::from(spans_from_snippet(
                &hit.snippet,
                snippet_base,
                snippet_match,
            ));
            ListItem::new(vec![title_line, snippet_line])
        })
        .collect();

    let title = format!("Search: {}", app.search_query());
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

/// Splits an FTS5 snippet on the `\u{1}`/`\u{2}` sentinels
/// `Index::search` wraps each matched term in (see `SearchHit`'s doc
/// comment) into styled spans — `base` for surrounding context, `matched`
/// for whatever was inside the sentinels. The sentinels themselves are
/// never included in the output.
fn spans_from_snippet(snippet: &str, base: Style, matched: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = snippet;
    while let Some(start) = rest.find('\u{1}') {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_string(), base));
        }
        let after_start = &rest[start + '\u{1}'.len_utf8()..];
        match after_start.find('\u{2}') {
            Some(end) => {
                spans.push(Span::styled(after_start[..end].to_string(), matched));
                rest = &after_start[end + '\u{2}'.len_utf8()..];
            }
            None => {
                spans.push(Span::styled(after_start.to_string(), base));
                return spans;
            }
        }
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base));
    }
    spans
}

/// Background shared by both status rows — matches Terapi/jsoned's status
/// bar convention rather than the old unstyled default background.
const STATUS_BG: Color = Color::Indexed(236);

/// Row 1: contextual breadcrumb (`vault › branch › note`). Row 2: mode
/// indicator + keybinding hints — or, when one applies, the delete
/// confirmation prompt, the quit-confirmation notice, or the last error,
/// same precedence as before the 2-line band existed. Hints are styled
/// per Terapi's hint-parser convention (bold key, dim colon/separator,
/// muted label) rather than jsoned's plain concatenated string.
fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    draw_breadcrumb(frame, rows[0], app);
    draw_hint_row(frame, rows[1], app);
}

fn draw_breadcrumb(frame: &mut Frame, area: Rect, app: &App) {
    let mut text = app.vault_name().to_string();
    for title in app.breadcrumb_titles() {
        text.push_str(" › ");
        text.push_str(&title);
    }

    let paragraph =
        Paragraph::new(text).style(Style::default().bg(STATUS_BG).fg(Color::Gray));
    frame.render_widget(paragraph, area);
}

fn draw_hint_row(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Style::default().bg(STATUS_BG);

    if app.mode == Mode::ConfirmDelete {
        let title = app.pending_delete_title().unwrap_or("this note");
        let descendants = app.pending_delete_descendant_count().unwrap_or(0);
        let text = if descendants > 0 {
            format!("Delete '{title}' and its {descendants} descendant(s)? y/n")
        } else {
            format!("Delete '{title}'? y/n")
        };
        let paragraph =
            Paragraph::new(text).style(bg.fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    if app.confirm_quit {
        let paragraph = Paragraph::new("Press q again to quit")
            .style(bg.fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    if let Some(err) = &app.last_error {
        let paragraph = Paragraph::new(format!("ERROR  {err}"))
            .style(bg.fg(Color::Red).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    let (mode_label, hints) = match app.mode {
        Mode::Normal => (
            "NORMAL",
            "j/k: move  h/l/space: fold  a/o: new  y: copy  Tab/S-Tab: move  \
             K/J: reorder  i: rename  e: edit  d: delete  u: undo  ^R: redo  \
             /: search  b: backlinks  [/]: tree width  {/}: backlinks width  \
             q: quit",
        ),
        Mode::Insert => ("INSERT", "Enter: confirm  Esc: cancel"),
        Mode::Search => (
            "SEARCH",
            "type: filter  Up/Down: move  Enter: open  Esc: cancel",
        ),
        Mode::Backlinks => (
            "BACKLINKS",
            "j/k: move  Enter: jump  Esc/b: back to tree",
        ),
        Mode::EditBody => ("EDIT BODY", "Esc: save & exit"),
        Mode::ConfirmDelete => unreachable!("handled above"),
    };

    let mode_style = bg.fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let key_style = bg.add_modifier(Modifier::BOLD);
    let sep_style = bg.add_modifier(Modifier::DIM);
    let label_style = bg.fg(Color::Gray);

    let mut spans = vec![
        Span::styled(mode_label, mode_style),
        Span::styled("  ", sep_style),
    ];
    spans.extend(spans_from_hints(hints, key_style, sep_style, label_style));

    frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
}

/// Splits a `"key: label  key: label  ..."` hint string (double-space
/// separated) into styled spans — bold key, dim colon/separator, muted
/// label — matching Terapi's hint-parser convention.
fn spans_from_hints(
    text: &str,
    key_style: Style,
    sep_style: Style,
    label_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, token) in text.split("  ").filter(|t| !t.is_empty()).enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", sep_style));
        }
        match token.split_once(": ") {
            Some((key, label)) => {
                spans.push(Span::styled(key.to_string(), key_style));
                spans.push(Span::styled(": ", sep_style));
                spans.push(Span::styled(label.to_string(), label_style));
            }
            None => spans.push(Span::styled(token.to_string(), key_style)),
        }
    }
    spans
}
