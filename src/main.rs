use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecursiveMode, Watcher};
use ratatui::{backend::CrosstermBackend, Terminal};

use mycora::app::App;
use mycora::archive;
use mycora::config::Config;
use mycora::index::Index;
use mycora::link;
use mycora::note::NoteId;
use mycora::repair::{self, Confidence};
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
    /// Rebuild the SQLite index for every mounted vault from its Markdown files.
    ///
    /// Not just the active one — read-only mounted vaults are indexed too.
    Reindex {
        /// Keep running and reindex again whenever a file in any mounted
        /// vault's directory changes, instead of exiting after one pass.
        #[arg(long)]
        watch: bool,
    },
    /// Manage the vault registry in config.toml.
    Vault {
        #[command(subcommand)]
        action: VaultCommand,
    },
    /// Flatten a note's subtree to a single Markdown or PDF file.
    ///
    /// Matches by exact title within the active vault. Errors if zero or
    /// multiple notes share that title — use the TUI's `:export` instead
    /// to disambiguate by direct selection. Refuses if the output path
    /// already exists rather than overwriting it. Output format is
    /// inferred from the path's extension: `.pdf` renders a paginated
    /// PDF, anything else is written as plain Markdown.
    Export {
        /// Exact title of the note whose subtree to export.
        title: String,
        /// Path to write the export to (.pdf for PDF, anything else for Markdown).
        output: PathBuf,
    },
    /// Import an existing Obsidian-style vault as a new Mycora vault.
    ///
    /// Folder structure becomes tree structure: a subdirectory becomes a
    /// parent note (reusing a same-named .md file as that note's content
    /// if one exists, else an empty placeholder), everything inside it
    /// becomes children. `[[Title|Alias]]`/`[[Title#Heading]]` links are
    /// rewritten down to plain `[[Title]]` so Mycora's own wikilink
    /// resolution can actually find them. Always creates a new vault and
    /// mounts it, same as `vault init` — refuses if the destination
    /// already exists and isn't empty.
    Import {
        /// Path to the existing Obsidian vault directory to read from.
        source: PathBuf,
        /// Name for the new vault in the registry.
        name: String,
        /// Path to create the new Mycora vault directory at.
        path: PathBuf,
    },
    /// Report (and optionally fix) broken [[wikilink]]s across every
    /// mounted vault.
    ///
    /// With no flags, only reports — the safe default, and its own
    /// preview of exactly what --apply would do. Detection always
    /// considers every mounted vault's note titles as candidates, for
    /// accurate cross-vault suggestions, even when --vault narrows which
    /// vault's own broken links get reported/fixed.
    Repair {
        /// Retarget a broken link to its best-guess match (a
        /// case-insensitive exact match, or a close-enough fuzzy match)
        /// by rewriting the note's body. Links without a confident match
        /// are left untouched. The only flag that rewrites an existing
        /// note's body — there's no undo for this outside your own
        /// backups/version control.
        #[arg(long)]
        apply: bool,
        /// Create an empty stub note for every broken link with no
        /// plausible match, so it resolves. One stub per distinct
        /// missing title per vault, not one per occurrence — never
        /// touches an existing file.
        #[arg(long)]
        create_stubs: bool,
        /// Only report/fix broken links whose source note is in this
        /// mounted vault.
        #[arg(long)]
        vault: Option<String>,
    },
}

#[derive(Subcommand)]
enum VaultCommand {
    /// Register a vault in config.toml's registry.
    Add {
        /// Name for the vault — its index vault_id, must be unique in the
        /// registry (and is what "default" needs to match to be the
        /// editable vault, see Config::active_vault).
        name: String,
        /// Path to the vault's directory on disk. Doesn't need to exist
        /// yet — Vault::open creates it on first use, same as launching
        /// the TUI against a brand-new vault_path does today.
        path: PathBuf,
        /// Register it without mounting (mounted = false in the written
        /// entry). Mounted by default, matching every other vault entry.
        #[arg(long)]
        no_mount: bool,
    },
    /// Create a vault directory, register it (always mounted), and report
    /// whether it became the active (read-write) vault.
    Init {
        /// Name for the vault — its index vault_id, must be unique in the
        /// registry.
        name: String,
        /// Path to the vault's directory on disk; created if it doesn't
        /// exist yet.
        path: PathBuf,
    },
    /// Rename a registered vault. Path and mount state are unaffected.
    Rename {
        /// Current name.
        old_name: String,
        /// New name.
        new_name: String,
    },
    /// Make a vault the active (read-write) one, by renaming it to
    /// "default" — the name Config::active_vault looks for. Fails if a
    /// different vault already holds that name; rename it out of the way
    /// first with `vault rename`.
    Promote {
        /// Name of the vault to promote.
        name: String,
    },
    /// Flag a registered vault to load at startup.
    Mount {
        /// Name of the vault to mount.
        name: String,
    },
    /// Flag a registered vault to *not* load at startup — stays known to
    /// the registry, just inactive until mounted again.
    Unmount {
        /// Name of the vault to unmount.
        name: String,
    },
    /// Unregister a vault. Never touches its files on disk — only
    /// forgets the config.toml entry. Refuses on "default"; rename or
    /// promote another vault first.
    Remove {
        /// Name of the vault to remove.
        name: String,
    },
    /// List every registered vault, its path, and its mount/active state.
    List,
    /// Compress an unmounted vault's directory into a single archive
    /// file, then remove the original directory. Refuses if the vault is
    /// still mounted (unmount it first) or already archived.
    Archive {
        /// Name of the vault to archive.
        name: String,
        /// Where to write the compressed archive. Defaults to
        /// `<name>.tar.gz` next to the vault's own directory if omitted.
        output: Option<PathBuf>,
    },
    /// Reverse of `archive`: extracts the vault's archive back to its
    /// original directory and removes the archive file. Leaves it
    /// unmounted — `vault mount` separately to activate it again.
    Unarchive {
        /// Name of the vault to unarchive.
        name: String,
    },
    /// Rename every note's file to match its current title.
    ///
    /// A note created via `a`/`o` in the TUI gets its filename from
    /// whatever title it had at that exact moment (often the "New note"
    /// placeholder, before you've typed a real one) — renaming the note
    /// afterward has never renamed the file to match, only new saves do
    /// (as of the version that added this command). This retroactively
    /// fixes every note already on disk with a stale, title-mismatched
    /// filename; safe to run repeatedly, notes whose filename already
    /// matches their title are left untouched.
    SyncFilenames {
        /// Name of the vault to fix.
        name: String,
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
        Some(Command::Vault {
            action: VaultCommand::Add {
                name,
                path,
                no_mount,
            },
        }) => return vault_add(&name, path, !no_mount),
        Some(Command::Vault {
            action: VaultCommand::Init { name, path },
        }) => return vault_init(&name, path),
        Some(Command::Vault {
            action:
                VaultCommand::Rename {
                    old_name,
                    new_name,
                },
        }) => return vault_rename(&old_name, &new_name),
        Some(Command::Vault {
            action: VaultCommand::Promote { name },
        }) => return vault_promote(&name),
        Some(Command::Vault {
            action: VaultCommand::Mount { name },
        }) => return vault_set_mounted(&name, true),
        Some(Command::Vault {
            action: VaultCommand::Unmount { name },
        }) => return vault_set_mounted(&name, false),
        Some(Command::Vault {
            action: VaultCommand::Remove { name },
        }) => return vault_remove(&name),
        Some(Command::Vault {
            action: VaultCommand::List,
        }) => return vault_list(),
        Some(Command::Vault {
            action: VaultCommand::Archive { name, output },
        }) => return vault_archive(&name, output),
        Some(Command::Vault {
            action: VaultCommand::Unarchive { name },
        }) => return vault_unarchive(&name),
        Some(Command::Vault {
            action: VaultCommand::SyncFilenames { name },
        }) => return vault_sync_filenames(&name),
        Some(Command::Export { title, output }) => return export_note(&title, output),
        Some(Command::Import { source, name, path }) => return import_vault(source, &name, path),
        Some(Command::Repair {
            apply,
            create_stubs,
            vault,
        }) => return repair(apply, create_stubs, vault.as_deref()),
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

/// Resolves the config path and delegates to `Config::add_vault` — the CLI
/// side of `mycora vault add`. Prints a one-line confirmation on success;
/// errors (config dir undeterminable, config.toml unparseable, duplicate
/// name) propagate as `main`'s own `Result` does for every other
/// subcommand.
fn vault_add(name: &str, path: PathBuf, mounted: bool) -> anyhow::Result<()> {
    let config_path = Config::default_path()?;
    Config::add_vault(&config_path, name, path.clone(), mounted)?;
    let mount_note = if mounted { "" } else { " (not mounted)" };
    println!(
        "mycora: added vault \"{name}\" ({}) to {}{mount_note}",
        path.display(),
        config_path.display()
    );
    Ok(())
}

/// Creates the vault directory (`Vault::open`, same lazy `create_dir_all`
/// every vault gets on first use), registers it always-mounted via
/// `Config::add_vault`, then reports whether it actually became the
/// active (read-write) vault. Doesn't force that: `Config::active_vault`
/// only picks an entry named `"default"` (or the first mounted one if
/// none is), so if another vault already holds that name, this one is
/// still created and mounted but stays read-only in the TUI — the user
/// is told so explicitly rather than an existing vault's registry entry
/// being silently renamed to make room (confirmed with the user before
/// implementing: creating and reporting honestly, not reassigning).
fn vault_init(name: &str, path: PathBuf) -> anyhow::Result<()> {
    Vault::open(path.clone())?;

    let config_path = Config::default_path()?;
    Config::add_vault(&config_path, name, path.clone(), true)?;

    let config = Config::load()?;
    if config.active_vault().name == name {
        println!(
            "mycora: created and mounted \"{name}\" ({}) as the active (read-write) vault",
            path.display()
        );
    } else {
        println!(
            "mycora: created and mounted \"{name}\" ({}), but \"{}\" is still the active \
             (read-write) vault — \"{name}\" stays read-only in the TUI until you rename \
             entries in {} so \"{name}\" is the one named \"default\"",
            path.display(),
            config.active_vault().name,
            config_path.display()
        );
    }
    Ok(())
}

fn vault_rename(old_name: &str, new_name: &str) -> anyhow::Result<()> {
    let config_path = Config::default_path()?;
    Config::rename_vault(&config_path, old_name, new_name)?;
    println!(
        "mycora: renamed vault \"{old_name}\" to \"{new_name}\" in {}",
        config_path.display()
    );
    Ok(())
}

fn vault_promote(name: &str) -> anyhow::Result<()> {
    let config_path = Config::default_path()?;
    Config::promote_vault(&config_path, name)?;
    println!(
        "mycora: \"{name}\" is now the active (read-write) vault in {}",
        config_path.display()
    );
    Ok(())
}

fn vault_set_mounted(name: &str, mounted: bool) -> anyhow::Result<()> {
    let config_path = Config::default_path()?;
    if mounted {
        Config::mount_vault(&config_path, name)?;
    } else {
        Config::unmount_vault(&config_path, name)?;
    }
    let state = if mounted { "mounted" } else { "unmounted" };
    println!(
        "mycora: \"{name}\" is now {state} in {}",
        config_path.display()
    );
    Ok(())
}

fn vault_remove(name: &str) -> anyhow::Result<()> {
    let config_path = Config::default_path()?;
    Config::remove_vault(&config_path, name)?;
    println!(
        "mycora: removed vault \"{name}\" from {} (its files on disk were not touched)",
        config_path.display()
    );
    Ok(())
}

/// Prints every registered vault, its path, and whether it's mounted/the
/// active one — reads via `Config::load()` (not the raw file) so this
/// reflects the same self-healing/legacy-migration view the TUI and every
/// other command see, not a literal dump of config.toml.
fn vault_list() -> anyhow::Result<()> {
    let config = Config::load()?;
    let active_name = config.active_vault().name.clone();

    println!("mycora: {} vault(s) registered", config.vaults.len());
    for entry in &config.vaults {
        let mut state = Vec::new();
        if entry.name == active_name {
            state.push("active");
        }
        if entry.archived.is_some() {
            state.push("archived");
        } else if entry.mounted {
            state.push("mounted");
        } else {
            state.push("not mounted");
        }
        println!(
            "  {:<16} {}  [{}]",
            entry.name,
            entry.path.display(),
            state.join(", ")
        );
    }
    Ok(())
}

/// Compresses `name`'s directory into a single gzip-compressed tar and
/// removes the original — `mycora vault archive`. Refuses if `name` is
/// still mounted (unmount it first — archiving a directory that's still
/// meant to be live would silently pull the rug out from under it) or
/// already archived. Verifies the archive is readable before deleting
/// anything, so a failure partway through never leaves the vault's notes
/// existing nowhere at all.
fn vault_archive(name: &str, output: Option<PathBuf>) -> anyhow::Result<()> {
    let config = Config::load()?;
    let entry = config
        .vaults
        .iter()
        .find(|v| v.name == name)
        .with_context(|| format!("no vault named \"{name}\""))?;

    if entry.mounted {
        bail!(
            "vault \"{name}\" is mounted — unmount it first with `mycora vault unmount {name}`, \
             then retry `mycora vault archive {name}`"
        );
    }
    if entry.archived.is_some() {
        bail!("vault \"{name}\" is already archived");
    }
    if !entry.path.exists() {
        bail!(
            "{} does not exist — nothing to archive",
            entry.path.display()
        );
    }

    let output = output.unwrap_or_else(|| entry.path.with_file_name(format!("{name}.tar.gz")));
    if output.exists() {
        bail!("{} already exists", output.display());
    }

    archive::archive_vault_dir(&entry.path, &output)
        .with_context(|| format!("archiving {}", entry.path.display()))?;
    archive::verify_archive(&output).with_context(|| format!("verifying {}", output.display()))?;

    std::fs::remove_dir_all(&entry.path)
        .with_context(|| format!("removing {}", entry.path.display()))?;

    let config_path = Config::default_path()?;
    Config::archive_vault(&config_path, name, output.clone())?;

    println!(
        "mycora: archived \"{name}\" to {} (original directory removed)",
        output.display()
    );
    Ok(())
}

/// Reverse of `vault_archive`: extracts the archive back to `name`'s
/// registered directory and removes the archive file. Leaves the vault
/// unmounted afterward — `vault mount` is a separate, explicit step,
/// same "one command, one effect" reasoning as every other `vault ...`
/// subcommand.
fn vault_unarchive(name: &str) -> anyhow::Result<()> {
    let config = Config::load()?;
    let entry = config
        .vaults
        .iter()
        .find(|v| v.name == name)
        .with_context(|| format!("no vault named \"{name}\""))?;

    let Some(archive_path) = &entry.archived else {
        bail!("vault \"{name}\" is not archived");
    };

    let destination_occupied = std::fs::read_dir(&entry.path)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if destination_occupied {
        bail!(
            "{} already exists and is not empty — nothing to unarchive into",
            entry.path.display()
        );
    }

    archive::unarchive_vault_dir(archive_path, &entry.path)
        .with_context(|| format!("unarchiving into {}", entry.path.display()))?;

    std::fs::remove_file(archive_path)
        .with_context(|| format!("removing {}", archive_path.display()))?;

    let config_path = Config::default_path()?;
    Config::unarchive_vault(&config_path, name)?;

    println!(
        "mycora: unarchived \"{name}\" to {} (archive file removed) — still unmounted, \
         `mycora vault mount {name}` to activate it",
        entry.path.display()
    );
    Ok(())
}

/// Retroactively fixes every note whose filename no longer matches its
/// title — see `VaultCommand::SyncFilenames`'s doc comment for why that
/// can happen. Loads the named vault directly (doesn't need it mounted;
/// this is a maintenance pass over its files, not something that touches
/// the registry), re-saves every note via `Vault::save_note` — the exact
/// same rename-on-mismatch logic a normal in-TUI edit now triggers,
/// just run once over everything instead of waiting for each note's
/// next edit — and reports how many files actually moved. Safe to run
/// repeatedly: a note whose filename already matches is left untouched
/// (`save_note` only renames when the slug actually differs).
fn vault_sync_filenames(name: &str) -> anyhow::Result<()> {
    let config = Config::load()?;
    let entry = config
        .vaults
        .iter()
        .find(|v| v.name == name)
        .with_context(|| format!("no vault named \"{name}\""))?;
    if entry.archived.is_some() {
        bail!(
            "vault \"{name}\" is archived — `mycora vault unarchive {name}` first, then retry"
        );
    }

    let mut vault = Vault::open(entry.path.clone())?;
    let (tree, report) = vault.load()?;
    for warning in &report.warnings {
        eprintln!("mycora: [{name}] {warning}");
    }

    let ids: Vec<NoteId> = tree
        .roots()
        .iter()
        .flat_map(|&root| tree.subtree_ids(root))
        .collect();
    let mut renamed_count = 0;
    for id in &ids {
        let note = tree
            .get(*id)
            .expect("every id from subtree_ids resolves in the same tree");
        if vault.save_note(*id, note)? {
            renamed_count += 1;
        }
    }

    println!(
        "mycora: checked {} note(s) in \"{name}\", renamed {renamed_count} file(s) to match \
         their titles",
        ids.len()
    );
    Ok(())
}

/// Finds every note titled exactly `title` in the active vault's whole
/// tree (not just its roots), errors unless there's exactly one match —
/// titles aren't required to be unique in Mycora, same as
/// [[wikilink]] resolution's own fan-out behavior, but a CLI export needs
/// one unambiguous target and has no selection context (unlike the TUI's
/// `:export`) to disambiguate with — then writes its subtree
/// (`mycora::export::flatten_subtree`) to `output`, rendering it to a PDF
/// instead of writing raw Markdown if `output` ends in `.pdf`. Refuses if
/// `output` already exists rather than overwriting it.
fn export_note(title: &str, output: PathBuf) -> anyhow::Result<()> {
    let config = Config::load()?;
    let active = config.active_vault();
    let mut vault = Vault::open(active.path.clone())?;
    let (tree, report) = vault.load()?;
    for warning in &report.warnings {
        eprintln!("mycora: {warning}");
    }

    let matches: Vec<NoteId> = tree
        .roots()
        .iter()
        .flat_map(|&root| tree.subtree_ids(root))
        .filter(|&id| tree.get(id).is_some_and(|note| note.title == title))
        .collect();

    let id = match matches.as_slice() {
        [] => bail!("no note titled \"{title}\" in vault \"{}\"", active.name),
        [id] => *id,
        _ => bail!(
            "{} notes are titled \"{title}\" in vault \"{}\" — use the TUI's `:export` \
             instead to disambiguate by direct selection",
            matches.len(),
            active.name
        ),
    };

    if output.exists() {
        bail!("{} already exists", output.display());
    }

    let content = mycora::export::flatten_subtree(&tree, id);
    mycora::export::write_output(&content, &output)
        .map_err(|err| anyhow::anyhow!(err))
        .with_context(|| format!("writing {}", output.display()))?;
    println!("mycora: exported \"{title}\" to {}", output.display());
    Ok(())
}

/// Converts an Obsidian-style vault at `source` into a brand new Mycora
/// vault at `path`, registering and mounting it (same as `vault_init`).
/// Refuses if `path` already exists and has any content — importing on
/// top of something else isn't a case worth guessing about, same
/// don't-silently-clobber instinct as `export`'s refuse-on-existing-file.
fn import_vault(source: PathBuf, name: &str, path: PathBuf) -> anyhow::Result<()> {
    if !source.is_dir() {
        bail!("{} is not a directory", source.display());
    }
    if path.exists() && path.read_dir()?.next().is_some() {
        bail!("{} already exists and is not empty", path.display());
    }

    let (tree, warnings) = mycora::import::import_obsidian_vault(&source)?;
    for warning in &warnings {
        eprintln!("mycora: {warning}");
    }

    let mut vault = Vault::open(path.clone())?;
    let ids: Vec<NoteId> = tree
        .roots()
        .iter()
        .flat_map(|&root| tree.subtree_ids(root))
        .collect();
    for id in &ids {
        let note = tree
            .get(*id)
            .expect("every id from subtree_ids resolves in the same tree");
        vault.save_note(*id, note)?;
    }

    let config_path = Config::default_path()?;
    Config::add_vault(&config_path, name, path.clone(), true)?;

    println!(
        "mycora: imported {} note(s) from {} into \"{name}\" ({}), mounted",
        ids.len(),
        source.display(),
        path.display()
    );
    Ok(())
}

/// Rebuilds the SQLite index for every *mounted* vault (`Config::mounted_vaults`)
/// from its Markdown files and reports how many notes were indexed in each.
/// The index itself is always disposable — this is safe to rerun any time.
fn reindex() -> anyhow::Result<()> {
    let config = Config::load()?;
    let index_path = Index::default_path()?;
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
    let index_path = Index::default_path()?;

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

/// One mounted vault, fully loaded and reindexed — the unit
/// `load_and_reindex_mounted` produces and both `perform_reindex` and
/// `perform_repair` consume.
struct LoadedVault {
    name: String,
    tree: Tree,
    vault: Vault,
    report: mycora::index::ReindexReport,
}

/// Loads every mounted vault fresh from disk and rebuilds its index rows
/// together. Shared by the reindex paths (one-shot and `--watch`) and by
/// `mycora repair`, the only other CLI path that needs every mounted
/// vault loaded together with accurate cross-vault broken-link data.
/// Vaults are loaded before any of them are indexed, and reindexed as one
/// `Index::reindex_mounted` batch — cross-vault wikilink resolution needs
/// every vault's notes visible to the index together, not one at a time
/// (see that method's doc comment). Prints `vault.load()`'s own warnings
/// as it goes, the same way it always has.
fn load_and_reindex_mounted(config: &Config) -> anyhow::Result<Vec<LoadedVault>> {
    let index_path = Index::default_path()?;
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

    Ok(loaded
        .into_iter()
        .zip(reports)
        .map(|((name, tree, vault), report)| LoadedVault {
            name,
            tree,
            vault,
            report,
        })
        .collect())
}

/// Prints a warning per broken wikilink (a `[[title]]` that didn't
/// resolve to any note in any mounted vault) the same way `vault.load()`'s
/// own warnings are printed — reported, not an error — and returns
/// `(vault name, note count)` per vault.
fn perform_reindex(config: &Config) -> anyhow::Result<Vec<(String, usize)>> {
    let loaded = load_and_reindex_mounted(config)?;

    let mut results = Vec::new();
    for lv in &loaded {
        for broken in &lv.report.broken_links {
            let source_title = lv
                .tree
                .get(broken.source)
                .map(|note| note.title.as_str())
                .unwrap_or("?");
            eprintln!(
                "mycora: [{}] broken link in \"{source_title}\": [[{}]] matches no note",
                lv.name, broken.title
            );
        }
        results.push((lv.name.clone(), lv.report.note_count));
    }
    Ok(results)
}

fn repair(apply: bool, create_stubs: bool, vault_filter: Option<&str>) -> anyhow::Result<()> {
    let config = Config::load()?;
    perform_repair(&config, apply, create_stubs, vault_filter)
}

/// Reports every broken wikilink across every mounted vault (same
/// detection as `reindex`, via `load_and_reindex_mounted`), and — if
/// `apply`/`create_stubs` are set — fixes what it confidently can.
/// `vault_filter`, if given, must name a vault that's actually mounted
/// (only mounted vaults get loaded at all) and narrows which vault's own
/// broken links get reported/fixed; detection still considers every
/// mounted vault's titles as suggestion candidates either way, for
/// accurate cross-vault matches. Never forces a second reindex pass after
/// fixing — the Markdown files are the source of truth and are now
/// correct; the disposable SQLite index catches up on the next natural
/// reindex, same as every other write this crate makes outside a
/// `reindex`/`--watch` call.
fn perform_repair(
    config: &Config,
    apply: bool,
    create_stubs: bool,
    vault_filter: Option<&str>,
) -> anyhow::Result<()> {
    let mut loaded = load_and_reindex_mounted(config)?;

    if let Some(filter) = vault_filter
        && !loaded.iter().any(|lv| lv.name == filter)
    {
        bail!("no mounted vault named \"{filter}\"");
    }

    // Every note's title across every loaded vault, snapshotted before any
    // fix is applied — suggestions are computed against the original
    // state, so a stub created for one broken link never becomes a
    // candidate match for another processed later in this same run.
    let all_titles: Vec<String> = loaded
        .iter()
        .flat_map(|lv| lv.tree.iter().map(|(_, note)| note.title.clone()))
        .collect();

    let mut total_broken = 0usize;
    let mut total_retargeted = 0usize;
    let mut total_stubs = 0usize;
    let mut total_unresolved = 0usize;

    for lv in &mut loaded {
        if vault_filter.is_some_and(|filter| filter != lv.name) {
            continue;
        }

        // Distinct (source note, broken title) pairs — a title repeated
        // several times in one note's body is one thing to fix, not one
        // per occurrence (`extract_wikilink_titles`/`write_links` don't
        // dedup, so `broken_links` can contain duplicates).
        let mut seen: HashSet<(NoteId, String)> = HashSet::new();
        let mut fixes: Vec<(NoteId, String, repair::Suggestion)> = Vec::new();
        let mut missing_titles: HashSet<String> = HashSet::new();
        let mut missing_pair_count = 0usize;

        for broken in &lv.report.broken_links {
            if !seen.insert((broken.source, broken.title.clone())) {
                continue;
            }
            total_broken += 1;

            let source_title = lv
                .tree
                .get(broken.source)
                .map(|note| note.title.clone())
                .unwrap_or_else(|| "?".to_string());

            match repair::suggest(&broken.title, &all_titles) {
                Some(suggestion) => {
                    let reason = match suggestion.confidence {
                        Confidence::Certain => "case difference",
                        Confidence::Likely => "similar title",
                    };
                    println!(
                        "mycora: [{}] broken link in \"{source_title}\": [[{}]] matches no \
                         note — maybe [[{}]] ({reason})",
                        lv.name, broken.title, suggestion.title
                    );
                    fixes.push((broken.source, broken.title.clone(), suggestion));
                }
                None => {
                    println!(
                        "mycora: [{}] broken link in \"{source_title}\": [[{}]] matches no \
                         note (no similar title found)",
                        lv.name, broken.title
                    );
                    missing_titles.insert(broken.title.clone());
                    missing_pair_count += 1;
                }
            }
        }

        if apply {
            let mut touched: HashSet<NoteId> = HashSet::new();
            for (source, old_title, suggestion) in &fixes {
                let Some(note) = lv.tree.get(*source) else {
                    continue;
                };
                let new_body = link::rewrite_wikilink_title(&note.body, old_title, &suggestion.title);
                lv.tree.set_body(*source, new_body);
                touched.insert(*source);
            }
            for id in touched {
                let note = lv.tree.get(id).expect("just set its body above");
                lv.vault.save_note(id, note)?;
            }
            total_retargeted += fixes.len();
        } else {
            total_unresolved += fixes.len();
        }

        if create_stubs {
            for title in &missing_titles {
                let new_id = lv.tree.create_note(title.clone(), None);
                let note = lv.tree.get(new_id).expect("just created it");
                lv.vault.save_note(new_id, note)?;
                total_stubs += 1;
            }
        } else {
            total_unresolved += missing_pair_count;
        }
    }

    println!(
        "mycora: repair complete — {total_broken} broken link(s), {total_retargeted} \
         retargeted, {total_stubs} stub(s) created, {total_unresolved} still unresolved"
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
