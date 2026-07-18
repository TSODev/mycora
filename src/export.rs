use crate::note::NoteId;
use crate::tree::Tree;
use markdown2pdf::fonts::{FontConfig, FontSource};
use std::path::Path;

/// Embedded so PDF export stays self-contained (see
/// `pdf-export-renders-through-a-pure-rust-crate.md` in the showcase
/// vault) rather than depending on whatever's installed on the host —
/// `markdown2pdf`'s own default (no font configured) falls back to the
/// 14 standard PDF fonts, which only support WinAnsi/Latin-1-ish
/// punctuation and replace *everything* else, accented Latin letters
/// included, with a literal `?` (see its own `to_win1252` doc comment).
/// DejaVu Sans/Sans Mono (Bitstream Vera License, `assets/fonts/`) cover
/// Latin Extended, Greek, and Cyrillic — not CJK or emoji, which would
/// need a much larger font; picked as the point covering the common
/// case (French/European accented text, the actual bug report this
/// fixes) without ballooning the binary.
static DEJAVU_SANS: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
static DEJAVU_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");

/// `markdown2pdf` only auto-discovers a bold sibling file next to an
/// on-disk font (`FontSource::File`/`System`) by filename convention —
/// an embedded `FontSource::Bytes` has no path for that, so bold text
/// (every heading, since `flatten_subtree` turns note titles into
/// headings, plus any `**bold**` in a note body) falls back to this
/// same regular-weight font rather than a bold one. Visually flatter
/// than true bold, but still correctly-rendered Unicode text — the
/// actual bug this exists to fix — rather than reintroducing the `?`
/// problem for every heading by falling back further to a builtin bold
/// font. Not worth the added complexity of writing embedded bytes out
/// to a temp file just to hand `markdown2pdf` a path to discover a
/// sibling `-Bold.ttf` from, unless bold fidelity turns out to matter
/// in practice.
fn unicode_font_config() -> FontConfig {
    FontConfig::new()
        .with_default_font_source(FontSource::bytes(DEJAVU_SANS))
        .with_code_font_source(FontSource::bytes(DEJAVU_SANS_MONO))
}

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

/// Writes flattened Markdown `content` to `path`, rendering it to a
/// paginated PDF first if `path` ends in `.pdf` (case-insensitive) —
/// otherwise it's written verbatim as Markdown. The two CLI/TUI export
/// entry points share this so "same command, format inferred from the
/// output extension" is the only place that decision lives, rather than
/// picking a format explicitly.
pub fn write_output(content: &str, path: &Path) -> Result<(), String> {
    if is_pdf_path(path) {
        let font_config = unicode_font_config();
        markdown2pdf::parse_into_file(
            content.to_string(),
            path,
            markdown2pdf::config::ConfigSource::Default,
            Some(&font_config),
        )
        .map_err(|err| err.to_string())
    } else {
        std::fs::write(path, content).map_err(|err| err.to_string())
    }
}

fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
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

    #[test]
    fn recognizes_pdf_paths_case_insensitively() {
        assert!(is_pdf_path(Path::new("notes.pdf")));
        assert!(is_pdf_path(Path::new("notes.PDF")));
        assert!(!is_pdf_path(Path::new("notes.md")));
        assert!(!is_pdf_path(Path::new("notes")));
    }

    #[test]
    fn write_output_writes_markdown_verbatim_for_non_pdf_paths() {
        let dir = std::env::temp_dir().join(format!("mycora-export-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.md");

        write_output("# Hello\n\nBody.\n\n", &path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Hello\n\nBody.\n\n");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_output_renders_a_real_pdf_for_pdf_paths() {
        let dir = std::env::temp_dir().join(format!("mycora-export-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.pdf");

        write_output("# Hello\n\nBody.\n\n", &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Regression test for the bug this font config exists to fix: without
    /// it, non-ASCII text (accented Latin, Cyrillic, ...) rendered as a
    /// literal `?` in the PDF (`markdown2pdf`'s builtin-font fallback only
    /// transliterates a curated set of punctuation, see `to_win1252` in
    /// its own source). Can't easily assert the *rendered glyphs* without
    /// a PDF-parsing dependency — `markdown2pdf` compresses object
    /// streams, so even the font dictionary isn't visible to a plain byte
    /// search — but an embedded font subset makes the file meaningfully
    /// bigger than the builtin-only path for the same content, which is
    /// exactly the signal that would go quiet if `write_output` ever
    /// stopped passing a font config through.
    #[test]
    fn write_output_embeds_a_unicode_font_for_pdf_paths() {
        let dir = std::env::temp_dir().join(format!("mycora-export-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let content = "# Café à Zürich\n\nAccents : é è à ç ù ê î ô. Cyrillique : Привет мир.\n";

        let builtin_path = dir.join("builtin.pdf");
        markdown2pdf::parse_into_file(
            content.to_string(),
            &builtin_path,
            markdown2pdf::config::ConfigSource::Default,
            None,
        )
        .unwrap();

        let unicode_path = dir.join("unicode.pdf");
        write_output(content, &unicode_path).unwrap();

        let builtin_len = std::fs::metadata(&builtin_path).unwrap().len();
        let unicode_len = std::fs::metadata(&unicode_path).unwrap().len();
        assert!(
            unicode_len > builtin_len + 1000,
            "expected the embedded-font PDF ({unicode_len} bytes) to be \
             meaningfully bigger than the builtin-font one ({builtin_len} \
             bytes) — a subsetted TrueType font adds several KB"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
