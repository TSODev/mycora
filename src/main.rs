use std::io;

use anyhow::Context;
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use mycora::app::App;
use mycora::config::Config;
use mycora::index::Index;
use mycora::vault::Vault;
use mycora::{event, ui};

/// Mycora — a tree-native, mycelium-linked note-taking TUI
#[derive(Parser)]
#[command(name = "mycora", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Rebuild the SQLite index for the active vault from its Markdown files.
    Reindex,
}

/// Restores the terminal (raw mode + alternate screen) before a panic's
/// default report prints — otherwise a panic while the TUI is active leaves
/// the terminal in a broken state (garbled input, invisible cursor) until
/// the user runs `reset`/`stty sane`.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(panic_info);
    }));
}

fn main() -> anyhow::Result<()> {
    install_panic_hook();
    let cli = Cli::parse();

    if let Some(Command::Reindex) = cli.command {
        return reindex();
    }

    let (mut app, warnings) = App::new()?;
    for warning in &warnings {
        eprintln!("mycora: {warning}");
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut app, &mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Rebuilds the SQLite index for the active vault (`config.active_vault()`)
/// from its Markdown files and reports how many notes were indexed. The
/// index itself is always disposable — this is safe to rerun any time.
fn reindex() -> anyhow::Result<()> {
    let config = Config::load()?;
    let active = config.active_vault().clone();

    let mut vault = Vault::open(active.path.clone())?;
    let (tree, report) = vault.load()?;
    for warning in &report.warnings {
        eprintln!("mycora: {warning}");
    }

    let home = std::env::var("HOME").context("HOME environment variable is not set")?;
    let index_path = Index::default_path(&home);
    let mut index = Index::open(&index_path)?;
    let count = index.reindex(&active.name, &tree, &vault)?;

    println!(
        "mycora: reindexed {count} note(s) from vault \"{}\" into {}",
        active.name,
        index_path.display()
    );
    Ok(())
}

fn run(app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        event::poll_and_handle(app)?;
    }
    Ok(())
}
