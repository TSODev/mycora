use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::note::{Note, NoteId};
use crate::tree::Tree;

/// Reads an Obsidian-style vault directory (nested folders, optional YAML
/// frontmatter, `[[wikilink]]`-shaped links, no `parent` field of any
/// kind) into a `Tree`, mirroring `Vault::load`'s own `(Tree, warnings)`
/// shape for a *foreign* source format instead of Mycora's own.
///
/// Folder structure becomes tree structure — confirmed with the user
/// before implementing, rather than importing everything flat: a
/// subdirectory `Foo/` becomes a parent note, reusing a sibling `Foo.md`
/// as that note if one exists (so a real Obsidian "folder note" keeps its
/// own content) or synthesizing an empty note titled `Foo` if not, and
/// everything inside `Foo/` becomes its children. `.obsidian/` and
/// anything that isn't a `.md` file (images, canvases, plugin data) are
/// skipped.
pub fn import_obsidian_vault(source: &Path) -> Result<(Tree, Vec<String>)> {
    let mut tree = Tree::new();
    let mut warnings = Vec::new();
    import_dir(source, None, &mut tree, &mut warnings)?;
    tree.rebuild_hierarchy();
    Ok((tree, warnings))
}

fn import_dir(
    dir: &Path,
    parent: Option<NoteId>,
    tree: &mut Tree,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .collect::<std::io::Result<_>>()
        .with_context(|| format!("reading {}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(".obsidian") {
            continue;
        }
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path);
        }
    }

    let mut order = 0i64;
    for dir_path in &dirs {
        let dir_name = dir_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let matching_file = files
            .iter()
            .position(|f| f.file_stem().and_then(|s| s.to_str()) == Some(dir_name.as_str()));

        let folder_note_id = if let Some(idx) = matching_file {
            let file_path = files.remove(idx);
            import_file(&file_path, parent, order, tree, warnings)?
        } else {
            let id = NoteId::new();
            let now = OffsetDateTime::now_utc();
            tree.insert_loaded(
                id,
                Note {
                    title: dir_name,
                    body: String::new(),
                    tags: Vec::new(),
                    parent,
                    children: Vec::new(),
                    order,
                    created: now,
                    updated: now,
                },
            );
            id
        };
        order += 1;
        import_dir(dir_path, Some(folder_note_id), tree, warnings)?;
    }

    for file_path in &files {
        import_file(file_path, parent, order, tree, warnings)?;
        order += 1;
    }

    Ok(())
}

fn import_file(
    path: &Path,
    parent: Option<NoteId>,
    order: i64,
    tree: &mut Tree,
    warnings: &mut Vec<String>,
) -> Result<NoteId> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let title = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".to_string());

    let (frontmatter, body) = split_frontmatter(&raw);
    let tags = frontmatter
        .and_then(|fm| match serde_yaml::from_str::<ObsidianFrontmatter>(fm) {
            Ok(parsed) => parsed.tags.map(TagsField::into_vec),
            Err(err) => {
                warnings.push(format!("{}: unparseable frontmatter, tags dropped: {err}", path.display()));
                None
            }
        })
        .unwrap_or_default();

    let body = strip_wikilink_targets(body);
    let now = OffsetDateTime::now_utc();
    let id = NoteId::new();
    tree.insert_loaded(
        id,
        Note {
            title,
            body,
            tags,
            parent,
            children: Vec::new(),
            order,
            created: now,
            updated: now,
        },
    );
    Ok(id)
}

/// Obsidian's frontmatter is optional and arbitrary-shaped, unlike
/// `vault.rs`'s own fixed `id`/`parent`/`order`/`tags`/`created`/`updated`
/// schema — this only cares about `tags`, ignoring every other field
/// rather than requiring the whole block to match a strict shape.
#[derive(Debug, Deserialize)]
struct ObsidianFrontmatter {
    #[serde(default)]
    tags: Option<TagsField>,
}

/// Obsidian accepts `tags: foo` (single string) or `tags: [foo, bar]`/a
/// YAML list — both are common in the wild.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TagsField {
    Single(String),
    Many(Vec<String>),
}

impl TagsField {
    fn into_vec(self) -> Vec<String> {
        match self {
            TagsField::Single(tag) => vec![tag],
            TagsField::Many(tags) => tags,
        }
    }
}

/// Splits `raw` into `(Some(frontmatter_yaml), body)` if it starts with a
/// `---`-delimited block, else `(None, raw)` unchanged. Unlike
/// `vault.rs`'s `split_frontmatter`, missing or malformed delimiters are
/// not an error — plenty of real Obsidian notes have no frontmatter at
/// all, which is completely normal there, just treated as "no tags" here.
fn split_frontmatter(raw: &str) -> (Option<&str>, &str) {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return (None, raw);
    };
    let Some(end) = rest.find("\n---\n") else {
        return (None, raw);
    };
    let frontmatter = &rest[..end];
    let body = rest[end + 5..].trim_start_matches('\n');
    (Some(frontmatter), body)
}

/// Rewrites every `[[Title|Alias]]` or `[[Title#Heading]]` down to plain
/// `[[Title]]` — Mycora's own wikilink scanner (`link.rs`) only
/// understands bare `[[Title]]`, so without this every aliased or
/// heading-anchored link (extremely common in real Obsidian vaults)
/// would silently become a broken link the moment it's resolved. Same
/// small hand-rolled bracket-scanning style as `link.rs` itself — finds
/// `[[`, finds the matching `]]`, keeps only the text before the first
/// `|` or `#` inside. An unclosed `[[` stops rewriting and passes the
/// rest of the body through unchanged, same "malformed syntax shouldn't
/// break anything" stance as `link.rs`'s own scanner.
fn strip_wikilink_targets(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    loop {
        let Some(start) = rest.find("[[") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("]]") else {
            out.push_str(&rest[start..]);
            break;
        };
        let inner = &after_open[..end];
        let title = inner
            .split(['|', '#'])
            .next()
            .unwrap_or(inner)
            .trim();
        out.push_str("[[");
        out.push_str(title);
        out.push_str("]]");
        rest = &after_open[end + 2..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn scratch_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mycora-import-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn strips_a_pipe_alias() {
        assert_eq!(
            strip_wikilink_targets("see [[Real Title|shown as this]] here"),
            "see [[Real Title]] here"
        );
    }

    #[test]
    fn strips_a_heading_anchor() {
        assert_eq!(
            strip_wikilink_targets("see [[Real Title#Some Heading]] here"),
            "see [[Real Title]] here"
        );
    }

    #[test]
    fn leaves_a_plain_wikilink_untouched() {
        assert_eq!(
            strip_wikilink_targets("see [[Plain Title]] here"),
            "see [[Plain Title]] here"
        );
    }

    #[test]
    fn stops_cleanly_at_an_unclosed_bracket() {
        assert_eq!(
            strip_wikilink_targets("[[Fine]] then [[unterminated"),
            "[[Fine]] then [[unterminated"
        );
    }

    #[test]
    fn split_frontmatter_returns_none_when_absent() {
        let (fm, body) = split_frontmatter("# Just a note\nNo frontmatter here.");
        assert!(fm.is_none());
        assert_eq!(body, "# Just a note\nNo frontmatter here.");
    }

    #[test]
    fn split_frontmatter_extracts_a_real_block() {
        let raw = "---\ntags: [a, b]\n---\nBody text.";
        let (fm, body) = split_frontmatter(raw);
        assert_eq!(fm, Some("tags: [a, b]"));
        assert_eq!(body, "Body text.");
    }

    #[test]
    fn imports_a_flat_vault_with_no_subfolders() {
        let dir = scratch_dir();
        fs::write(dir.join("One.md"), "First note.").unwrap();
        fs::write(dir.join("Two.md"), "---\ntags: solo\n---\nSecond note.").unwrap();

        let (tree, warnings) = import_obsidian_vault(&dir).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(tree.roots().len(), 2);

        let mut titles: Vec<&str> = tree.roots().iter().map(|&id| tree.get(id).unwrap().title.as_str()).collect();
        titles.sort();
        assert_eq!(titles, vec!["One", "Two"]);

        let two = tree
            .roots()
            .iter()
            .find(|&&id| tree.get(id).unwrap().title == "Two")
            .unwrap();
        assert_eq!(tree.get(*two).unwrap().tags, vec!["solo".to_string()]);
        assert_eq!(tree.get(*two).unwrap().body, "Second note.");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reuses_a_same_named_file_as_the_folder_note() {
        let dir = scratch_dir();
        fs::write(dir.join("Projects.md"), "Folder note content.").unwrap();
        let projects_dir = dir.join("Projects");
        fs::create_dir_all(&projects_dir).unwrap();
        fs::write(projects_dir.join("Child.md"), "Child content.").unwrap();

        let (tree, _warnings) = import_obsidian_vault(&dir).unwrap();
        assert_eq!(tree.roots().len(), 1);
        let root = tree.roots()[0];
        assert_eq!(tree.get(root).unwrap().title, "Projects");
        assert_eq!(tree.get(root).unwrap().body, "Folder note content.");
        assert_eq!(tree.children(root).len(), 1);
        let child = tree.children(root)[0];
        assert_eq!(tree.get(child).unwrap().title, "Child");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn synthesizes_an_empty_folder_note_when_none_matches() {
        let dir = scratch_dir();
        let archive_dir = dir.join("Archive");
        fs::create_dir_all(&archive_dir).unwrap();
        fs::write(archive_dir.join("Old.md"), "Old content.").unwrap();

        let (tree, _warnings) = import_obsidian_vault(&dir).unwrap();
        assert_eq!(tree.roots().len(), 1);
        let root = tree.roots()[0];
        assert_eq!(tree.get(root).unwrap().title, "Archive");
        assert_eq!(tree.get(root).unwrap().body, "");
        assert_eq!(tree.children(root).len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skips_the_obsidian_config_directory() {
        let dir = scratch_dir();
        let obsidian_dir = dir.join(".obsidian");
        fs::create_dir_all(&obsidian_dir).unwrap();
        fs::write(obsidian_dir.join("workspace.json"), "{}").unwrap();
        fs::write(dir.join("Real.md"), "Real content.").unwrap();

        let (tree, _warnings) = import_obsidian_vault(&dir).unwrap();
        assert_eq!(tree.roots().len(), 1);
        assert_eq!(tree.get(tree.roots()[0]).unwrap().title, "Real");

        fs::remove_dir_all(&dir).ok();
    }
}
