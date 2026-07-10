use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Mode};

pub fn poll_and_handle(app: &mut App) -> anyhow::Result<()> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(());
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        // Crossterm raw mode disables SIGINT generation, so Ctrl+C would
        // otherwise do nothing. Treat it as an immediate, unconditional
        // quit — bypasses modals and the q/q confirm dance, since the whole
        // point of Ctrl+C is being an emergency escape hatch (matches
        // Terapi/jsoned).
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            app.should_quit = true;
            return Ok(());
        }

        if key.code != KeyCode::Char('q') {
            app.reset_quit_confirmation();
        }
        match app.mode {
            Mode::Normal => handle_normal(app, key),
            Mode::Insert => handle_insert(app, key.code),
            Mode::ConfirmDelete => handle_confirm_delete(app, key.code),
            Mode::Search => handle_search(app, key.code),
            Mode::Backlinks => handle_backlinks(app, key.code),
        }
    }

    Ok(())
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.redo();
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.request_quit(),
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => app.expand_selected(),
        KeyCode::Char('h') | KeyCode::Left => app.collapse_selected(),
        KeyCode::Char(' ') => app.toggle_expand(),
        KeyCode::Char('o') => app.create_sibling(),
        KeyCode::Char('a') => app.create_child(),
        KeyCode::Char('y') => app.copy_selected(),
        KeyCode::Char('d') => app.request_delete(),
        KeyCode::Char('i') => app.begin_rename(),
        KeyCode::Tab => app.indent_selected(),
        KeyCode::BackTab => app.outdent_selected(),
        KeyCode::Char('K') => app.reorder_up(),
        KeyCode::Char('J') => app.reorder_down(),
        KeyCode::Char('u') => app.undo(),
        KeyCode::Char('/') => app.begin_search(),
        KeyCode::Char('b') => app.show_backlinks(),
        _ => {}
    }
}

fn handle_insert(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.commit_rename(),
        KeyCode::Esc => app.cancel_rename(),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => app.input.push(c),
        _ => {}
    }
}

fn handle_confirm_delete(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Esc => app.cancel_delete(),
        _ => {}
    }
}

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Up => app.move_search_selection(-1),
        KeyCode::Down => app.move_search_selection(1),
        KeyCode::Char(c) => app.search_input(c),
        _ => {}
    }
}

fn handle_backlinks(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.confirm_backlinks(),
        KeyCode::Esc => app.cancel_backlinks(),
        KeyCode::Up => app.move_backlinks_selection(-1),
        KeyCode::Down => app.move_backlinks_selection(1),
        _ => {}
    }
}
