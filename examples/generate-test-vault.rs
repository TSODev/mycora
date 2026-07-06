//! Generates a synthetic vault for TUI load-testing: a flat directory of
//! Markdown files with real Mycora frontmatter, organized into a few levels
//! of category/sub-category notes, with cross-referencing [[wikilinks]] and
//! random tags. Reuses `mycora::vault::Vault` directly so the output is
//! guaranteed to match the app's actual on-disk format.
//!
//! Usage: generate-test-vault [output_dir] [leaf_note_count]

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use fake::faker::lorem::raw::{Paragraphs, Words};
use fake::locales::EN;
use fake::Fake;
use rand::rng;
use rand::seq::IndexedRandom;

use mycora::note::{Note, NoteId};
use mycora::vault::Vault;

const CATEGORIES: &[&str] = &["Rust", "Architecture", "TUI Design", "Memory Management"];
const SUB_CATEGORIES: &[&str] = &["Concepts", "Advanced", "Debugging"];
const TAG_POOL: &[&str] = &["draft", "reviewed", "idea", "reference", "archive"];

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let output_dir: PathBuf = args.next().unwrap_or_else(|| "test-vault".to_string()).into();
    let leaf_count: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);

    let mut vault = Vault::open(output_dir.clone())?;
    let mut rng = rng();

    // Category and sub-category notes form the browsable tree structure,
    // linked by `parent`/`order` rather than by nested directories.
    let mut sub_category_ids = Vec::new();
    for (cat_order, category) in CATEGORIES.iter().enumerate() {
        let cat_id = NoteId::new();
        vault.save_note(cat_id, &new_note(category, None, cat_order as i64))?;

        for (sub_order, sub_category) in SUB_CATEGORIES.iter().enumerate() {
            let sub_id = NoteId::new();
            vault.save_note(
                sub_id,
                &new_note(sub_category, Some(cat_id), sub_order as i64),
            )?;
            sub_category_ids.push(sub_id);
        }
    }

    // Pre-generate leaf titles so notes can [[wikilink]] to a real sibling.
    let leaf_titles: Vec<String> = (1..=leaf_count)
        .map(|n| {
            let words: Vec<String> = Words(EN, 2..4).fake();
            format!("{} {n}", words.join(" "))
        })
        .collect();

    // Distribute leaf notes round-robin across sub-categories, each with
    // lorem ipsum content, a few random tags, and a wikilink to another
    // randomly chosen leaf note.
    let mut next_order: HashMap<NoteId, i64> = HashMap::new();
    for (i, title) in leaf_titles.iter().enumerate() {
        let parent_id = sub_category_ids[i % sub_category_ids.len()];
        let order = next_order.entry(parent_id).or_insert(0);

        let linked_title = leaf_titles
            .choose(&mut rng)
            .filter(|candidate| *candidate != title)
            .cloned()
            .unwrap_or_else(|| "Index".to_string());

        let mut note = new_note(title, Some(parent_id), *order);
        note.body = generate_body(&linked_title);
        note.tags = random_tags(&mut rng);

        vault.save_note(NoteId::new(), &note)?;
        *order += 1;
    }

    println!(
        "Generated {} category notes, {} sub-category notes, and {leaf_count} leaf notes in {}",
        CATEGORIES.len(),
        sub_category_ids.len(),
        output_dir.display()
    );

    Ok(())
}

fn new_note(title: &str, parent: Option<NoteId>, order: i64) -> Note {
    let mut note = Note::new(title, parent);
    note.order = order;
    note
}

fn generate_body(linked_title: &str) -> String {
    let intro: Vec<String> = Paragraphs(EN, 1..2).fake();
    let body: Vec<String> = Paragraphs(EN, 2..4).fake();

    format!(
        "> Fake note generated for TUI load-testing.\n\n\
        ## Introduction\n{}\n\n\
        ## Body\n{}\n\n\
        ### Tasks\n\
        - [x] Set up the ratatui shell\n\
        - [ ] Parse Markdown with pulldown-cmark\n\
        - [ ] Handle vertical scrolling\n\n\
        ### Related\n\
        See also [[{linked_title}]].\n",
        intro.join("\n\n"),
        body.join("\n\n"),
    )
}

fn random_tags(rng: &mut rand::rngs::ThreadRng) -> Vec<String> {
    let count = *[0usize, 1, 2, 3].choose(rng).unwrap_or(&0);
    let mut tags: Vec<String> = TAG_POOL
        .sample(rng, count)
        .map(|s| s.to_string())
        .collect();
    tags.sort();
    tags
}
