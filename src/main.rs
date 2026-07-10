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
use mycora::tree::Tree;
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
    // Save regardless of should_quit vs. an error from run(): the app is
    // exiting either way, and this covers both q/q and Ctrl+C uniformly
    // (both just set should_quit and let the loop exit here) without
    // special-casing either path.
    let session_result = app.save_session();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = session_result {
        eprintln!("mycora: failed to save session: {err}");
    }

    result
}

/// Rebuilds the SQLite index for every *mounted* vault (`Config::mounted_vaults`)
/// from its Markdown files and reports how many notes were indexed in each.
/// The index itself is always disposable — this is safe to rerun any time.
fn reindex() -> anyhow::Result<()> {
    let config = Config::load()?;
    let index_path = Index::default_path(&config.home);
    let results = perform_reindex(&config)?;
    for (name, count) in &results {
        println!(
            "mycora: reindexed {count} note(s) from vault \"{name}\" into {}",
            index_path.display()
        );
    }
    Ok(())
}

/// Like `reindex`, but stays running afterward and reindexes again whenever
/// a file in any mounted vault's directory changes, rather than a single
/// pass. Reindexing is always a full rebuild of *every* mounted vault's
/// rows (not a per-file, per-vault diff) — matches `Index::reindex`'s own
/// "disposable, cheaper to regenerate wholesale than to diff" approach,
/// just triggered by filesystem events instead of a manual rerun. Every
/// vault directory is flat (`Vault::load` doesn't recurse), so each watch
/// is non-recursive too; that also means moving a trashed note into
/// `.trash/` still fires one legitimate change event (the file leaving the
/// watched root).
fn watch_reindex() -> anyhow::Result<()> {
    let config = Config::load()?;
    let index_path = Index::default_path(&config.home);

    let results = perform_reindex(&config)?;
    for (name, count) in &results {
        println!(
            "mycora: reindexed {count} note(s) from vault \"{name}\" into {}",
            index_path.display()
        );
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    for entry in config.mounted_vaults() {
        watcher.watch(&entry.path, RecursiveMode::NonRecursive)?;
        println!("mycora: watching {} for changes", entry.path.display());
    }
    println!("mycora: Ctrl+C to stop");

    loop {
        match rx.recv() {
            Ok(Ok(_event)) => {
                // Debounce: a single edit often fires several events (write
                // + rename-into-place for the atomic save). Swallow
                // anything else that arrives in the next short window
                // before reindexing once.
                while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
                match perform_reindex(&config) {
                    Ok(results) => {
                        let total: usize = results.iter().map(|(_, count)| count).sum();
                        println!("mycora: change detected, reindexed {total} note(s)")
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

/// Loads every mounted vault fresh from disk and rebuilds its index rows
/// together, returning `(vault name, note count)` per vault. Shared by the
/// one-shot and `--watch` reindex paths. Vaults are loaded before any of
/// them are indexed, and reindexed as one `Index::reindex_mounted` batch —
/// cross-vault wikilink resolution needs every vault's notes visible to
/// the index together, not one at a time (see that method's doc comment).
/// Prints a warning per broken wikilink (a `[[title]]` that didn't resolve
/// to any note in any mounted vault) the same way `vault.load()`'s own
/// warnings are printed — reported, not an error.
fn perform_reindex(config: &Config) -> anyhow::Result<Vec<(String, usize)>> {
    let index_path = Index::default_path(&config.home);
    let mut index = Index::open(&index_path)?;

    let mut loaded: Vec<(String, Tree, Vault)> = Vec::new();
    for entry in config.mounted_vaults() {
        let mut vault = Vault::open(entry.path.clone())?;
        let (tree, report) = vault.load()?;
        for warning in &report.warnings {
            eprintln!("mycora: [{}] {warning}", entry.name);
        }
        loaded.push((entry.name.clone(), tree, vault));
    }

    let batch: Vec<(&str, &Tree, &Vault)> = loaded
        .iter()
        .map(|(name, tree, vault)| (name.as_str(), tree, vault))
        .collect();
    let reports = index.reindex_mounted(&batch)?;

    let mut results = Vec::new();
    for ((name, tree, _), report) in loaded.iter().zip(reports.iter()) {
        for broken in &report.broken_links {
            let source_title = tree
                .get(broken.source)
                .map(|note| note.title.as_str())
                .unwrap_or("?");
            eprintln!(
                "mycora: [{name}] broken link in \"{source_title}\": [[{}]] matches no note",
                broken.title
            );
        }
        results.push((name.clone(), report.note_count));
    }
    Ok(results)
}

fn run(app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        event::poll_and_handle(app)?;
    }
    Ok(())
}
