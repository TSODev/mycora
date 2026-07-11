use crate::note::NoteId;
use crate::tree::Tree;

/// Flattens `root`'s subtree (itself and every descendant, depth-first,
/// respecting sibling order) into a single Markdown document. Each note's
/// title becomes a heading at a level matching its depth within the
/// subtree (`root` itself is `#`, its children `##`, and so on); any ATX
/// headings already inside a note's own body are shifted deeper by that
/// same amount, via a line-by-line scan (no full Markdown parse — same
/// "small hand-rolled scanner" spirit as `link.rs`'s wikilink extraction),
/// so a note's own internal structure nests *under* its title rather than
/// competing with it. No frontmatter, and `[[wikilinks]]` are left as
/// literal text — both deliberately out of scope for this first pass, see
/// ROADMAP.md.
pub fn flatten_subtree(tree: &Tree, root: NoteId) -> String {
    let mut out = String::new();
    push_flattened(tree, root, 0, &mut out);
    out
}

fn push_flattened(tree: &Tree, id: NoteId, depth: usize, out: &mut String) {
    let Some(note) = tree.get(id) else { return };
    let level = depth + 1;

    out.push_str(&"#".repeat(level));
    out.push(' ');
    out.push_str(&note.title);
    out.push_str("\n\n");

    if !note.body.trim().is_empty() {
        out.push_str(&shift_headings(&note.body, level));
        out.push_str("\n\n");
    }

    for &child in tree.children(id) {
        push_flattened(tree, child, depth + 1, out);
    }
}

/// Prepends `shift` extra `#` characters to every ATX heading line.
/// `starts_with('#')` alone would also match something like `#tag`-shaped
/// text, so this checks for the trailing space (or end-of-line)
/// CommonMark itself requires of a real ATX heading — everything else
/// passes through unchanged.
fn shift_headings(body: &str, shift: usize) -> String {
    let prefix = "#".repeat(shift);
    body.lines()
        .map(|line| {
            if is_atx_heading(line) {
                format!("{prefix}{line}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_atx_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    (1..=6).contains(&hashes) && matches!(line.as_bytes().get(hashes), None | Some(b' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(tree: &mut Tree, title: &str, body: &str, parent: Option<NoteId>) -> NoteId {
        let id = tree.create_note(title, parent);
        tree.set_body(id, body);
        id
    }

    #[test]
    fn flattens_a_single_leaf_note() {
        let mut tree = Tree::new();
        let root = note(&mut tree, "Solo", "Just a body.", None);

        assert_eq!(
            flatten_subtree(&tree, root),
            "# Solo\n\nJust a body.\n\n"
        );
    }

    #[test]
    fn nests_children_at_increasing_heading_depths() {
        let mut tree = Tree::new();
        let root = note(&mut tree, "Root", "Root body.", None);
        let child = note(&mut tree, "Child", "Child body.", Some(root));
        let _grandchild = note(&mut tree, "Grandchild", "Deep body.", Some(child));

        let out = flatten_subtree(&tree, root);
        assert_eq!(
            out,
            "# Root\n\nRoot body.\n\n\
             ## Child\n\nChild body.\n\n\
             ### Grandchild\n\nDeep body.\n\n"
        );
    }

    #[test]
    fn shifts_atx_headings_already_inside_a_body() {
        let mut tree = Tree::new();
        let root = note(
            &mut tree,
            "Root",
            "# Overview\nsome text\n## Details\nmore text",
            None,
        );
        let _child = note(&mut tree, "Child", "# Notes", Some(root));

        let out = flatten_subtree(&tree, root);
        // Root is level 1, so its body's headings shift by 1: "#" -> "##", "##" -> "###".
        assert!(out.contains("## Overview\nsome text\n### Details\nmore text"));
        // Child is level 2, so its body's "#" heading shifts by 2 -> "###".
        assert!(out.contains("## Child\n\n### Notes"));
    }

    #[test]
    fn does_not_treat_a_hash_without_a_trailing_space_as_a_heading() {
        let mut tree = Tree::new();
        let root = note(&mut tree, "Root", "#nothashtag stays put", None);

        let out = flatten_subtree(&tree, root);
        assert!(out.contains("#nothashtag stays put"));
        assert!(!out.contains("##nothashtag"));
    }

    #[test]
    fn skips_the_body_section_entirely_when_empty() {
        let mut tree = Tree::new();
        let root = note(&mut tree, "Empty", "", None);

        assert_eq!(flatten_subtree(&tree, root), "# Empty\n\n");
    }

    #[test]
    fn only_includes_the_given_root_and_its_own_descendants() {
        let mut tree = Tree::new();
        let root = note(&mut tree, "Root", "", None);
        let _sibling_root = note(&mut tree, "Unrelated root", "should not appear", None);
        let _child = note(&mut tree, "Child", "included", Some(root));

        let out = flatten_subtree(&tree, root);
        assert!(out.contains("Child"));
        assert!(out.contains("included"));
        assert!(!out.contains("Unrelated root"));
        assert!(!out.contains("should not appear"));
    }
}
