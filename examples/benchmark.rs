//! Timed dry-runs of the operations most likely to matter as a vault
//! grows: cold `Vault::load`, a full `Index::reindex`, an FTS5 `search`,
//! and `App::visible_rows` (the per-frame tree traversal every redraw
//! recomputes from scratch, see `app.rs`'s own note on why that's fine
//! at "current scale" — this is what checks whether it still is).
//!
//! Not a criterion-style statistical benchmark: no warm-up iterations,
//! no variance/outlier reporting, no new dependency. Just
//! `std::time::Instant` around one run of each operation per vault size,
//! printed as a table — enough to see whether something is linear,
//! quadratic, or fine, without adding permanent benchmarking
//! infrastructure on spec. See BENCHMARK.md for the latest captured
//! numbers, how to reproduce them, and what (if anything) they justify
//! changing — same "measure before committing" instinct ROADMAP.md's
//! v0.6 entry already applied to tantivy.
//!
//! Usage: cargo run --release --example benchmark [-- <sizes...>]
//!   e.g. cargo run --release --example benchmark -- 100 1000 5000 10000
//!
//! Always use `--release`: a debug build's timings are dominated by
//! unoptimized codegen, not the thing being measured.

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use mycora::app::App;
use mycora::index::Index;
use mycora::note::{Note, NoteId};
use mycora::vault::Vault;

const CATEGORIES: usize = 10;
const SUB_CATEGORIES_PER_CATEGORY: usize = 5;
const VISIBLE_ROWS_ITERATIONS: u32 = 100;

fn main() -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        eprintln!(
            "warning: this is a debug build — pass --release or the numbers won't mean anything"
        );
    }

    let sizes: Vec<usize> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let sizes = if sizes.is_empty() {
        vec![100, 1_000, 5_000, 10_000]
    } else {
        sizes
    };

    println!(
        "{:>8} | {:>10} | {:>12} | {:>10} | {:>16} | {:>18}",
        "notes", "generate", "cold load", "reindex", "search (1 query)", "visible_rows x100"
    );
    println!("{}", "-".repeat(90));

    for leaf_count in sizes {
        let row = bench_size(leaf_count)?;
        println!(
            "{:>8} | {:>10} | {:>12} | {:>10} | {:>16} | {:>18}",
            leaf_count,
            fmt(row.generate),
            fmt(row.load),
            fmt(row.reindex),
            fmt(row.search),
            fmt(row.visible_rows_100),
        );
    }

    Ok(())
}

struct Row {
    generate: Duration,
    load: Duration,
    reindex: Duration,
    search: Duration,
    visible_rows_100: Duration,
}

fn fmt(d: Duration) -> String {
    if d.as_secs_f64() >= 1.0 {
        format!("{:.2}s", d.as_secs_f64())
    } else {
        format!("{:.1}ms", d.as_secs_f64() * 1000.0)
    }
}

fn bench_size(leaf_count: usize) -> anyhow::Result<Row> {
    let scratch =
        std::env::temp_dir().join(format!("mycora-bench-{leaf_count}-{}", uuid::Uuid::new_v4()));
    let vault_dir = scratch.join("vault");
    let home_dir = scratch.join("home");
    std::fs::create_dir_all(&vault_dir)?;
    std::fs::create_dir_all(&home_dir)?;

    let t0 = Instant::now();
    generate_vault(&vault_dir, leaf_count)?;
    let generate = t0.elapsed();

    let t0 = Instant::now();
    let mut vault = Vault::open(vault_dir.clone())?;
    let (tree, _report) = vault.load()?;
    let load = t0.elapsed();

    let mut index = Index::open(&home_dir.join("index.sqlite3"))?;
    let t0 = Instant::now();
    index.reindex("default", &tree, &vault)?;
    let reindex = t0.elapsed();

    // "lorem" appears in every generated leaf's body — a worst case for
    // ranking/snippet cost (matches nearly everything), not a best case.
    let t0 = Instant::now();
    let _hits = index.search("default", "lorem")?;
    let search = t0.elapsed();

    let visible_rows_100 = bench_visible_rows(&vault_dir, &home_dir)?;

    std::fs::remove_dir_all(&scratch).ok();

    Ok(Row {
        generate,
        load,
        reindex,
        search,
        visible_rows_100,
    })
}

/// Times `App::visible_rows()` with every note expanded (the worst case —
/// the full note count is visible, not just root-level rows). Drives
/// `App::new()` through a real scratch `$HOME`/`config.toml`, the same
/// path a real launch takes, rather than reimplementing a parallel
/// construction path that could drift from it.
fn bench_visible_rows(vault_dir: &Path, home_dir: &Path) -> anyhow::Result<Duration> {
    let config_dir = home_dir.join(".config/mycora");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(
        config_dir.join("config.toml"),
        format!(
            "[[vaults]]\nname = \"default\"\npath = \"{}\"\nmounted = true\n",
            vault_dir.display()
        ),
    )?;
    // Safe here: this example is single-threaded and exits right after
    // finishing all sizes, so there's no concurrent reader of $HOME to
    // race with.
    unsafe {
        std::env::set_var("HOME", home_dir);
    }

    let (mut app, _warnings) = App::new()?;
    app.expanded = app.tree.iter().map(|(id, _)| id).collect::<HashSet<_>>();

    let t0 = Instant::now();
    for _ in 0..VISIBLE_ROWS_ITERATIONS {
        let rows = app.visible_rows();
        std::hint::black_box(rows);
    }
    Ok(t0.elapsed())
}

fn generate_vault(dir: &Path, leaf_count: usize) -> anyhow::Result<()> {
    let mut vault = Vault::open(dir.to_path_buf())?;

    let mut sub_category_ids = Vec::new();
    for cat in 0..CATEGORIES {
        let cat_id = NoteId::new();
        vault.save_note(cat_id, &branch_note(&format!("Category {cat}"), None, cat as i64))?;
        for sub in 0..SUB_CATEGORIES_PER_CATEGORY {
            let sub_id = NoteId::new();
            vault.save_note(
                sub_id,
                &branch_note(&format!("Sub {cat}-{sub}"), Some(cat_id), sub as i64),
            )?;
            sub_category_ids.push(sub_id);
        }
    }

    let mut prev_title: Option<String> = None;
    for i in 0..leaf_count {
        let parent = sub_category_ids[i % sub_category_ids.len()];
        let title = format!("Leaf note {i}");
        let mut note = Note::new(&title, Some(parent));
        note.order = (i / sub_category_ids.len()) as i64;
        note.tags = vec!["bench".to_string()];
        note.body = leaf_body(i, prev_title.as_deref());
        vault.save_note(NoteId::new(), &note)?;
        prev_title = Some(title);
    }

    Ok(())
}

fn branch_note(title: &str, parent: Option<NoteId>, order: i64) -> Note {
    let mut note = Note::new(title, parent);
    note.order = order;
    note
}

fn leaf_body(index: usize, link_target: Option<&str>) -> String {
    let mut body = format!(
        "Paragraph one of note {index}. Lorem ipsum dolor sit amet, consectetur \
         adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore \
         magna aliqua.\n\n\
         Paragraph two of note {index}. Ut enim ad minim veniam, quis nostrud \
         exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."
    );
    if let Some(target) = link_target {
        body.push_str(&format!("\n\nSee also [[{target}]] for a related note."));
    }
    body
}
