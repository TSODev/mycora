use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// A heading found in a note body, in document order. Backs both the `t`
/// table-of-contents overlay (level + title) and section extraction
/// (`start`/`end`, byte offsets into the source).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingRef {
    /// 1..=6, i.e. H1..H6.
    pub level: u8,
    pub title: String,
    /// Byte offset of the heading's start (the `#` for ATX headings).
    pub start: usize,
    /// Byte offset just past the whole heading construct — where the
    /// section's body begins.
    pub end: usize,
}

/// Every heading in `source`, in document order. Uses the same parser
/// options as `markdown::render` (`Options::ENABLE_TABLES`) so a `#`
/// inside a table cell or fenced code block is never mistaken for a
/// heading here either.
pub fn headings(source: &str) -> Vec<HeadingRef> {
    let mut result = Vec::new();
    // Headings never nest, so a single pending slot (not a stack) is
    // enough to accumulate one heading's title text across its Text/Code
    // events.
    let mut pending: Option<(u8, usize, String)> = None;
    for (event, range) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                pending = Some((level as u8, range.start, String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, title)) = &mut pending {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start, title)) = pending.take() {
                    result.push(HeadingRef {
                        level,
                        title: title.trim().to_string(),
                        start,
                        end: range.end,
                    });
                }
            }
            _ => {}
        }
    }
    result
}

/// Byte range owned by `headings[index]`'s section: from its own start up
/// to the next heading at the same or a shallower level, or `source_len`
/// if none follows. Deeper headings inside are part of this range, not a
/// boundary — that's what makes extraction of this range non-recursive.
pub fn section_range(headings: &[HeadingRef], index: usize, source_len: usize) -> Range<usize> {
    let heading = &headings[index];
    let end = headings[index + 1..]
        .iter()
        .find(|next| next.level <= heading.level)
        .map(|next| next.start)
        .unwrap_or(source_len);
    heading.start..end
}

/// Result of extracting `headings[index]`'s section out of `source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction {
    /// The extracted heading's plain text — the new child note's title.
    pub title: String,
    /// The section's content after the heading line, trimmed — the new
    /// child note's body. Any deeper sub-headings inside stay verbatim
    /// here as plain Markdown, never split into their own notes.
    pub body: String,
    /// `source` with the whole section replaced by a single `[[title]]`
    /// wikilink line at the position the section used to occupy.
    pub new_source: String,
}

pub fn extract_section(source: &str, headings: &[HeadingRef], index: usize) -> Extraction {
    let heading = &headings[index];
    let range = section_range(headings, index, source.len());
    let body = source[heading.end..range.end].trim().to_string();
    let new_source = format!(
        "{}[[{}]]\n{}",
        &source[..heading.start],
        heading.title,
        &source[range.end..]
    );
    Extraction {
        title: heading.title.clone(),
        body,
        new_source,
    }
}

/// Fixed width used to compute a heading's scroll target — `App` never
/// knows the live body-preview pane width (`ui.rs` is pure rendering, see
/// its own doc comment), so this reuses a reasonable constant rather than
/// the real one. See `scroll_offset_for`.
pub const SCROLL_RENDER_WIDTH: u16 = 80;

/// The line index into `markdown::render`'s output at which the heading
/// starting at byte offset `heading_start` begins — i.e. the
/// `App::body_scroll` value that brings it to the top of the preview.
/// Renders only the prefix of `source` up to the heading and counts the
/// lines produced. Exact for every block type except tables, whose
/// rendered height is width-sensitive: if a table precedes the heading
/// and the live pane width differs from `SCROLL_RENDER_WIDTH`, the jump
/// can land a few rows off — the same imprecision `App::scroll_body_down`
/// already accepts ("recovers with Ctrl+u/d"). Slicing at `heading_start`
/// is always lexically safe: a `Heading` event is only ever emitted for a
/// heading recognized outside any open fence/table, so every such prefix
/// is already balanced.
pub fn scroll_offset_for(source: &str, heading_start: usize) -> u16 {
    crate::markdown::render(&source[..heading_start], SCROLL_RENDER_WIDTH).len() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_lists_every_level_in_order() {
        let found = headings("# One\n\n## Two\n\n### Three");
        assert_eq!(
            found.iter().map(|h| (h.level, h.title.as_str())).collect::<Vec<_>>(),
            vec![(1, "One"), (2, "Two"), (3, "Three")]
        );
    }

    #[test]
    fn section_range_stops_at_a_same_level_heading() {
        let source = "## A\nbody\n### A.1\nnested\n## B\nbody b";
        let found = headings(source);
        let range = section_range(&found, 0, source.len());
        assert_eq!(&source[range], "## A\nbody\n### A.1\nnested\n");
    }

    #[test]
    fn section_range_stops_at_a_shallower_heading() {
        let source = "## A\nbody\n# Top\nbody top";
        let found = headings(source);
        let range = section_range(&found, 0, source.len());
        assert_eq!(&source[range], "## A\nbody\n");
    }

    #[test]
    fn section_range_runs_to_the_end_when_last() {
        let source = "# A\nbody\n## B\ntail content";
        let found = headings(source);
        let range = section_range(&found, 1, source.len());
        assert_eq!(&source[range], "## B\ntail content");
    }

    #[test]
    fn extract_section_is_non_recursive() {
        let source = "## Beta\nbeta text\n### Beta One\nnested text";
        let found = headings(source);
        let ex = extract_section(source, &found, 0);
        assert_eq!(ex.title, "Beta");
        assert_eq!(ex.body, "beta text\n### Beta One\nnested text");
    }

    #[test]
    fn extract_section_replaces_with_a_wikilink() {
        let source = "# Alpha\nintro\n## Beta\nbeta text\n## Gamma\ngamma text";
        let found = headings(source);
        let ex = extract_section(source, &found, 1);
        assert_eq!(
            ex.new_source,
            "# Alpha\nintro\n[[Beta]]\n## Gamma\ngamma text"
        );
    }

    #[test]
    fn headings_ignores_a_hash_inside_a_fenced_code_block() {
        let found = headings("```\n# not a heading\n```\n\n## Real Heading");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Real Heading");
    }

    #[test]
    fn scroll_offset_for_is_zero_for_the_first_heading() {
        let source = "# First\nbody";
        let found = headings(source);
        assert_eq!(scroll_offset_for(source, found[0].start), 0);
    }

    #[test]
    fn scroll_offset_for_counts_lines_before_a_later_heading() {
        let source = "# First\nbody\n\n## Second\nmore";
        let found = headings(source);
        let offset = scroll_offset_for(source, found[1].start);
        assert!(offset > 0);
    }
}
