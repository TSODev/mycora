use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode, TreeRow, COMMAND_REFERENCE};

/// Split-pane border accents. Named ANSI colors, not RGB/indexed — the
/// terminal maps these to whatever's actually configured (light theme,
/// dark theme, Solarized, ...), so light/dark theming ("respecting
/// terminal colors", per ROADMAP.md's v0.7 entry) comes for free rather
/// than needing an explicit theme switch. The backlinks pane has no idle
/// accent of its own; it only turns cyan when focused (see
/// `draw_backlinks_pane`) — that color is reused, not clashing, with the
/// status bar's own "this is the active thing" cyan (the mode label).
const PANE_TREE_COLOR: Color = Color::Blue;
const PANE_BODY_COLOR: Color = Color::Magenta;

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

/// Full-pane overlays (search, the body editor, `:tags` results) take over
/// the whole main area, same as before the split layout. Every other mode
/// — Normal, Insert (renaming), ConfirmDelete (whose prompt is only in
/// the status bar), Backlinks (which shifts focus onto the backlinks pane
/// rather than opening its own overlay — see `App::focus_backlinks`'s doc
/// comment), and Command (whose `:` prompt also lives in the status bar —
/// see `draw_hint_row`) — gets the three-pane split: tree, a read-only
/// body preview, and the backlinks list, both following the current
/// selection live. Column widths come from `App::pane_widths` (default
/// 40/40/20, adjustable with `[`/`]`/`{`/`}` — see that method's doc
/// comment). `Mode::Command` additionally overlays a small help popup
/// listing every recognized command (see `draw_command_help`) — shown for
/// as long as the `:` prompt is open, on request, so commands are
/// discoverable without leaving the prompt to look them up.
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
        Mode::TagResults => {
            draw_tag_results(frame, area, app);
            return;
        }
        Mode::Normal | Mode::Insert | Mode::ConfirmDelete | Mode::Backlinks | Mode::Command => {}
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

    if app.mode == Mode::Command {
        draw_command_help(frame, area);
    }
}

/// Renders `app.visible_rows()` — the active vault's tree followed by
/// each read-only mounted vault behind its own dimmed `── name ──`
/// separator, fully navigable (not roots-only) since `TreeRow::Note`
/// already carries per-row expand/children/link-count state resolved
/// against whichever tree the row actually belongs to (see
/// `App::visible_rows`'s doc comment). Read-only rows are dimmed, and
/// stay reversed-and-dimmed rather than plain-reversed when selected, so
/// a read-only selection still reads as "read-only" even while
/// highlighted.
fn draw_tree(frame: &mut Frame, area: Rect, app: &App) {
    let dim = Style::default().add_modifier(Modifier::DIM);

    let items: Vec<ListItem> = app
        .visible_rows()
        .into_iter()
        .map(|row| match row {
            TreeRow::VaultSeparator(name) => {
                ListItem::new(Line::from(Span::styled(format!("── {name} ──"), dim)))
            }
            TreeRow::Note {
                id,
                depth,
                title,
                has_children,
                expanded,
                link_count,
                editable,
            } => {
                let marker = if !has_children {
                    "  "
                } else if expanded {
                    "▾ "
                } else {
                    "▸ "
                };
                let indent = "  ".repeat(depth);

                let label = if app.mode == Mode::Insert && app.selected == Some(id) {
                    format!("{indent}{marker}{}", app.input)
                } else if !expanded && has_children && link_count > 0 {
                    let plural = if link_count == 1 { "" } else { "s" };
                    format!("{indent}{marker}{title} ({link_count} link{plural})")
                } else {
                    format!("{indent}{marker}{title}")
                };

                let base = if editable { Style::default() } else { dim };
                let style = if app.selected == Some(id) {
                    base.add_modifier(Modifier::REVERSED)
                } else {
                    base
                };

                ListItem::new(Line::from(Span::styled(label, style)))
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PANE_TREE_COLOR))
            .title("Mycora"),
    );
    frame.render_widget(list, area);
}

/// Read-only preview of the selected note's body, rendered as Markdown
/// (see `crate::markdown`) — resolved via `App::selected_note` so a note
/// in a read-only mounted vault is just as readable here as one in the
/// active vault. Empty when nothing's selected or the note has no body
/// yet.
fn draw_body_preview(frame: &mut Frame, area: Rect, app: &App) {
    let note = app.selected_note();
    let title = note.map(|n| n.title.as_str()).unwrap_or("");
    let body = note.map(|n| n.body.as_str()).unwrap_or("");

    let paragraph = Paragraph::new(crate::markdown::render(body))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PANE_BODY_COLOR))
                .title(title),
        );
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

/// Notes matched by a `:tags` command — full-pane overlay like `Search`,
/// but over a fixed result set rather than a live-as-you-type query, so
/// there's no query text or snippet to show, just titles.
fn draw_tag_results(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .tag_results()
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let style = if i == app.tag_results_selected() {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(hit.title.clone(), style)))
        })
        .collect();

    let list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Tag results"));
    frame.render_widget(list, area);
}

/// Small reference popup listing every command `App::execute_command`
/// recognizes (`COMMAND_REFERENCE`), anchored to the bottom of the main
/// area — directly above the status bar row where the `:` prompt itself
/// is being typed (see `draw_hint_row`'s `Mode::Command` branch). Static:
/// it doesn't filter as you type, just lists everything, since the
/// command set is small enough that filtering wouldn't save much. `Clear`
/// first so it reads as an opaque popup over the tree/body/backlinks
/// panes rather than blending with whatever text is underneath it.
fn draw_command_help(frame: &mut Frame, area: Rect) {
    let width = COMMAND_REFERENCE
        .iter()
        .map(|(cmd, desc)| cmd.len() + desc.len() + 4)
        .max()
        .unwrap_or(20) as u16
        + 2; // borders
    let height = COMMAND_REFERENCE.len() as u16 + 2; // borders

    let popup = popup_rect(area, width, height);
    frame.render_widget(Clear, popup);

    let cmd_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().add_modifier(Modifier::DIM);

    let lines: Vec<Line> = COMMAND_REFERENCE
        .iter()
        .map(|(cmd, desc)| {
            Line::from(vec![
                Span::styled(*cmd, cmd_style),
                Span::raw("  "),
                Span::styled(*desc, desc_style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Commands"),
    );
    frame.render_widget(paragraph, popup);
}

/// A `width`x`height` rect anchored to the bottom-center of `area`,
/// clamped so it never exceeds `area`'s own bounds.
fn popup_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height.saturating_sub(height);
    Rect {
        x,
        y,
        width,
        height,
    }
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

/// `READ-ONLY` label reserved on the right of the breadcrumb row — a
/// fixed-width column so the breadcrumb's own width doesn't shift as you
/// move in and out of read-only vaults. Blank (but still painted with
/// `STATUS_BG`) when the selection is editable.
const READ_ONLY_MARKER_WIDTH: u16 = 12;

fn draw_breadcrumb(frame: &mut Frame, area: Rect, app: &App) {
    let mut text = app.vault_name().to_string();
    for title in app.breadcrumb_titles() {
        text.push_str(" › ");
        text.push_str(&title);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(READ_ONLY_MARKER_WIDTH),
        ])
        .split(area);

    let breadcrumb =
        Paragraph::new(text).style(Style::default().bg(STATUS_BG).fg(Color::Gray));
    frame.render_widget(breadcrumb, chunks[0]);

    let marker = if app.selected_is_read_only() {
        "READ-ONLY"
    } else {
        ""
    };
    let marker = Paragraph::new(marker)
        .style(
            Style::default()
                .bg(STATUS_BG)
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(marker, chunks[1]);
}

fn draw_hint_row(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Style::default().bg(STATUS_BG);

    // Checked before last_error/last_message/confirm_quit: those are
    // independent fields that could still be set from before `:` was
    // pressed, but the live command input is what the user's looking at
    // right now and should always win while typing it.
    if app.mode == Mode::Command {
        let text = format!(":{}", app.command_input());
        let paragraph = Paragraph::new(text).style(bg.fg(Color::Gray));
        frame.render_widget(paragraph, area);
        return;
    }

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

    if let Some(msg) = &app.last_message {
        let paragraph = Paragraph::new(msg.as_str()).style(bg.fg(Color::Cyan));
        frame.render_widget(paragraph, area);
        return;
    }

    let (mode_label, hints) = match app.mode {
        Mode::Normal => (
            "NORMAL",
            "j/k: move  h/l/space: fold  a/o: new  y: copy  Tab/S-Tab: move  \
             K/J: reorder  i: rename  e: edit  d: delete  u: undo  ^R: redo  \
             /: search  b: backlinks  [/]: tree width  {/}: backlinks width  \
             colon: command  q: quit",
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
        Mode::TagResults => ("TAG RESULTS", "j/k: move  Enter: open  Esc: cancel"),
        Mode::ConfirmDelete | Mode::Command => unreachable!("handled above"),
    };

    let mode_style = bg.fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let key_style = bg.add_modifier(Modifier::BOLD);
    let sep_style = bg.add_modifier(Modifier::DIM);
    let label_style = bg.fg(Color::Gray);

    // In Normal mode with a read-only note selected, every mutating key
    // dims out (same style as the separators) instead of its usual
    // bold-key/muted-label styling — it'll still just report "this vault
    // is read-only" if pressed (see `App::require_editable`), so the hint
    // row says so before the user tries. Every other mode's hints are
    // either non-mutating already (Search, Backlinks, TagResults, ...)
    // or only ever reachable with an editable selection to begin with, so
    // no dimming applies there.
    let disabled_keys: &[&str] = if app.mode == Mode::Normal && app.selected_is_read_only() {
        &["a/o", "y", "Tab/S-Tab", "K/J", "i", "e", "d"]
    } else {
        &[]
    };

    let mut spans = vec![
        Span::styled(mode_label, mode_style),
        Span::styled("  ", sep_style),
    ];
    spans.extend(spans_from_hints(
        hints,
        key_style,
        sep_style,
        label_style,
        disabled_keys,
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
}

/// Splits a `"key: label  key: label  ..."` hint string (double-space
/// separated) into styled spans — bold key, dim colon/separator, muted
/// label — matching Terapi's hint-parser convention. A token whose key
/// exactly matches an entry in `disabled_keys` renders fully dimmed
/// (`sep_style` for both key and label) instead, to mark it as currently
/// unusable without removing it from the hint row entirely.
fn spans_from_hints(
    text: &str,
    key_style: Style,
    sep_style: Style,
    label_style: Style,
    disabled_keys: &[&str],
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, token) in text.split("  ").filter(|t| !t.is_empty()).enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", sep_style));
        }
        match token.split_once(": ") {
            Some((key, label)) => {
                let disabled = disabled_keys.contains(&key);
                let this_key_style = if disabled { sep_style } else { key_style };
                let this_label_style = if disabled { sep_style } else { label_style };
                spans.push(Span::styled(key.to_string(), this_key_style));
                spans.push(Span::styled(": ", sep_style));
                spans.push(Span::styled(label.to_string(), this_label_style));
            }
            None => spans.push(Span::styled(token.to_string(), key_style)),
        }
    }
    spans
}
