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

    let items: Vec<ListItem> = app
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
                format!("{indent}{marker}{}", note.title)
            };

            let style = if app.selected == Some(id) {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Mycora"));
    frame.render_widget(list, area);
}

fn draw_search(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .search_results()
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let style = if i == app.search_selected() {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(hit.title.clone(), style)))
        })
        .collect();

    let title = format!("Search: {}", app.search_query());
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
            "NORMAL  j/k move  h/l/space fold  a/o new  y copy  Tab/S-Tab move  K/J reorder  i rename  d delete  u undo  ^R redo  / search  q quit"
        }
        Mode::Insert => "INSERT  Enter confirm  Esc cancel",
        Mode::Search => "SEARCH  type to filter  Up/Down move  Enter open  Esc cancel",
        Mode::ConfirmDelete => unreachable!("handled above"),
    };
    frame.render_widget(Paragraph::new(text), area);
}
