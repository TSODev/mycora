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
        // A status message (e.g. `:export`'s "exported to ...") is
        // feedback about the *previous* action, not something that
        // should linger forever — clear it before dispatch so any other
        // keypress moves on from it. Whatever the key itself does can
        // still set a fresh one right after, overwriting this in the
        // same call.
        app.clear_transient_status();
        match app.mode {
            Mode::Normal => handle_normal(app, key),
            Mode::Insert => handle_insert(app, key.code),
            Mode::ConfirmDelete => handle_confirm_delete(app, key.code),
            Mode::Search => handle_search(app, key.code),
            Mode::Backlinks => handle_backlinks(app, key.code),
            Mode::EditBody => handle_edit_body(app, key),
            Mode::Command => handle_command(app, key.code),
            Mode::TagResults => handle_tag_results(app, key.code),
            Mode::TagList => handle_tag_list(app, key.code),
            Mode::Links => handle_links(app, key.code),
        }
    }

    Ok(())
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.redo();
        return;
    }
    // Ctrl+d/Ctrl+u (vim's half-page scroll) need the same early check as
    // Ctrl+r above: a plain `match key.code` can't tell Ctrl+d from bare
    // `d` (delete) or Ctrl+u from bare `u` (undo), since KeyCode::Char
    // doesn't carry modifiers.
    if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.scroll_body_down();
        return;
    }
    if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.scroll_body_up();
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
        KeyCode::Char('b') => app.focus_backlinks(),
        KeyCode::Char('f') => app.begin_links(),
        KeyCode::Char('e') => app.begin_edit_body(),
        KeyCode::Char('[') => app.shrink_tree_pane(),
        KeyCode::Char(']') => app.grow_tree_pane(),
        KeyCode::Char('{') => app.shrink_backlinks_pane(),
        KeyCode::Char('}') => app.grow_backlinks_pane(),
        KeyCode::Char(':') => app.begin_command(),
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
        KeyCode::Esc | KeyCode::Char('b') => app.cancel_backlinks(),
        KeyCode::Char('j') | KeyCode::Down => app.move_backlinks_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_backlinks_selection(-1),
        _ => {}
    }
}

/// `Esc` saves and exits (see `Mode::EditBody`'s doc comment on why —
/// there's no separate discard, `u` afterward covers that). Everything
/// else — including `Enter` for newlines, arrow keys, `Tab` — goes
/// straight to the textarea widget rather than being special-cased here.
///
/// The one exception: while the `[[wikilink]]` autocomplete popup is
/// open (`App::link_autocomplete_is_open`), `Up`/`Down`/`Tab`/`Enter`/
/// `Esc` are intercepted for the popup instead of reaching the textarea
/// — `Tab`/`Enter` would otherwise insert a literal tab or newline
/// (never useful mid-title), and `Esc` would otherwise save and exit the
/// whole editor rather than just dismissing the popup. Every other key
/// (plain typing, Backspace, arrow-key navigation, ...) still falls
/// through to the normal path below, which keeps the popup in sync as a
/// side effect — see `App::body_editor_input`'s doc comment.
fn handle_edit_body(app: &mut App, key: KeyEvent) {
    if app.link_autocomplete_is_open() {
        match key.code {
            KeyCode::Esc => {
                app.cancel_link_autocomplete();
                return;
            }
            KeyCode::Up => {
                app.move_link_autocomplete_selection(-1);
                return;
            }
            KeyCode::Down => {
                app.move_link_autocomplete_selection(1);
                return;
            }
            KeyCode::Tab | KeyCode::Enter => {
                app.accept_link_autocomplete();
                return;
            }
            _ => {}
        }
    }

    if key.code == KeyCode::Esc {
        app.save_and_exit_body_edit();
    } else {
        app.body_editor_input(key);
    }
}

fn handle_command(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.execute_command(),
        KeyCode::Esc => app.cancel_command(),
        KeyCode::Backspace => app.command_input_backspace(),
        KeyCode::Char(c) => app.command_input_push(c),
        _ => {}
    }
}

fn handle_tag_results(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.confirm_tag_results(),
        KeyCode::Esc => app.cancel_tag_results(),
        KeyCode::Char('j') | KeyCode::Down => app.move_tag_results_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_tag_results_selection(-1),
        _ => {}
    }
}

fn handle_tag_list(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.confirm_tag_list(),
        KeyCode::Esc => app.cancel_tag_list(),
        KeyCode::Char('j') | KeyCode::Down => app.move_tag_list_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_tag_list_selection(-1),
        _ => {}
    }
}

fn handle_links(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.confirm_links(),
        KeyCode::Esc => app.cancel_links(),
        KeyCode::Char('j') | KeyCode::Down => app.move_links_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_links_selection(-1),
        _ => {}
    }
}
