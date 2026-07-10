use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    draw_tree(frame, chunks[0], app);
    draw_status(frame, chunks[1], app);
}

fn draw_tree(frame: &mut Frame, area: Rect, app: &App) {
    if app.mode == Mode::Search {
        draw_search(frame, area, app);
        return;
    }
    if app.mode == Mode::Backlinks {
        draw_backlinks(frame, area, app);
        return;
    }
    if app.mode == Mode::EditBody {
        if let Some(editor) = app.body_editor() {
            frame.render_widget(editor, area);
        }
        return;
    }

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

fn draw_backlinks(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .backlinks_results()
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let style = if i == app.backlinks_selected() {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(hit.title.clone(), style)))
        })
        .collect();

    let target_title = app
        .selected
        .and_then(|id| app.tree.get(id))
        .map(|note| note.title.as_str())
        .unwrap_or("");
    let title = format!("Backlinks: {target_title}");
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    if app.mode == Mode::ConfirmDelete {
        let title = app.pending_delete_title().unwrap_or("this note");
        let descendants = app.pending_delete_descendant_count().unwrap_or(0);
        let text = if descendants > 0 {
            format!("Delete '{title}' and its {descendants} descendant(s)? y/n")
        } else {
            format!("Delete '{title}'? y/n")
        };
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    if app.confirm_quit {
        let paragraph = Paragraph::new("Press q again to quit")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    if let Some(err) = &app.last_error {
        let paragraph = Paragraph::new(format!("ERROR  {err}"))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        frame.render_widget(paragraph, area);
        return;
    }

    let text = match app.mode {
        Mode::Normal => {
            "NORMAL  j/k move  h/l/space fold  a/o new  y copy  Tab/S-Tab move  K/J reorder  i rename  d delete  u undo  ^R redo  / search  b backlinks  e edit  q quit"
        }
        Mode::Insert => "INSERT  Enter confirm  Esc cancel",
        Mode::Search => "SEARCH  type to filter  Up/Down move  Enter open  Esc cancel",
        Mode::Backlinks => "BACKLINKS  Up/Down move  Enter open  Esc cancel",
        Mode::EditBody => "EDIT BODY  Esc save & exit",
        Mode::ConfirmDelete => unreachable!("handled above"),
    };
    frame.render_widget(Paragraph::new(text), area);
}
