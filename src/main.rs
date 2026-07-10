use std::io;
use std::sync::mpsc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecursiveMode, Watcher};
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
    Reindex {
        /// Keep running and reindex again whenever a file in the vault
        /// directory changes, instead of exiting after one pass.
        #[arg(long)]
        watch: bool,
    },
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

    match cli.command {
        Some(Command::Reindex { watch: false }) => return reindex(),
        Some(Command::Reindex { watch: true }) => return watch_reindex(),
        None => {}
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
    let (count, index_path) = perform_reindex(&config)?;
    println!(
        "mycora: reindexed {count} note(s) from vault \"{}\" into {}",
        config.active_vault().name,
        index_path.display()
    );
    Ok(())
}

/// Like `reindex`, but stays running afterward and reindexes again whenever
/// a file in the vault directory changes, rather than a single pass.
/// Reindexing is always a full rebuild of the vault's rows (not a per-file
/// diff) — matches `Index::reindex`'s own "disposable, cheaper to
/// regenerate wholesale than to diff" approach, just triggered by
/// filesystem events instead of a manual rerun. The vault directory is
/// flat (`Vault::load` doesn't recurse), so the watch is non-recursive too;
/// that also means moving a trashed note into `.trash/` still fires one
/// legitimate change event (the file leaving the watched root).
fn watch_reindex() -> anyhow::Result<()> {
    let config = Config::load()?;
    let active = config.active_vault().clone();

    let (count, index_path) = perform_reindex(&config)?;
    println!(
        "mycora: reindexed {count} note(s) from vault \"{}\" into {}",
        active.name,
        index_path.display()
    );
    println!(
        "mycora: watching {} for changes (Ctrl+C to stop)",
        active.path.display()
    );

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&active.path, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(Ok(_event)) => {
                // Debounce: a single edit often fires several events (write
                // + rename-into-place for the atomic save). Swallow
                // anything else that arrives in the next short window
                // before reindexing once.
                while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
                match perform_reindex(&config) {
                    Ok((count, _)) => {
                        println!("mycora: change detected, reindexed {count} note(s)")
                    }
                    Err(err) => eprintln!("mycora: reindex failed: {err}"),
                }
            }
            Ok(Err(err)) => eprintln!("mycora: watch error: {err}"),
            Err(_) => break, // watcher/channel dropped
        }
    }
    Ok(())
}

/// Loads the active vault fresh from disk and rebuilds its index rows.
/// Shared by the one-shot and `--watch` reindex paths. Prints a warning per
/// broken wikilink (a `[[title]]` that didn't resolve to any note) the same
/// way `vault.load()`'s own warnings are printed — reported, not an error.
fn perform_reindex(config: &Config) -> anyhow::Result<(usize, std::path::PathBuf)> {
    let active = config.active_vault();
    let mut vault = Vault::open(active.path.clone())?;
    let (tree, report) = vault.load()?;
    for warning in &report.warnings {
        eprintln!("mycora: {warning}");
    }

    let index_path = Index::default_path(&config.home);
    let mut index = Index::open(&index_path)?;
    let reindex_report = index.reindex(&active.name, &tree, &vault)?;
    for broken in &reindex_report.broken_links {
        let source_title = tree
            .get(broken.source)
            .map(|note| note.title.as_str())
            .unwrap_or("?");
        eprintln!(
            "mycora: broken link in \"{source_title}\": [[{}]] matches no note",
            broken.title
        );
    }
    Ok((reindex_report.note_count, index_path))
}

fn run(app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        event::poll_and_handle(app)?;
    }
    Ok(())
}
