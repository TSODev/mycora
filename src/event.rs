use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::{App, Mode};

pub fn poll_and_handle(app: &mut App) -> anyhow::Result<()> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(());
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        match app.mode {
            Mode::Normal => handle_normal(app, key.code),
            Mode::Insert => handle_insert(app, key.code),
        }
    }

    Ok(())
}

fn handle_normal(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => app.expand_selected(),
        KeyCode::Char('h') | KeyCode::Left => app.collapse_selected(),
        KeyCode::Char(' ') => app.toggle_expand(),
        KeyCode::Char('o') => app.create_sibling(),
        KeyCode::Char('a') => app.create_child(),
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('i') | KeyCode::Char('r') => app.begin_rename(),
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
