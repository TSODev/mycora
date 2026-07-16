use pulldown_cmark::{Alignment, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Renders a note body's Markdown into styled lines for a read-only
/// preview pane. Headings, bold/italic, inline/block code, lists,
/// blockquotes, horizontal rules, and GFM tables get distinct styling.
/// Nothing is interactive — links render as their text, not as
/// clickable/navigable spans, and `[[wikilinks]]` aren't CommonMark syntax
/// so they just render as literal bracketed text (no special-casing here;
/// that's a separate concern from "render the Markdown").
///
/// `width` is the caller's pane width, used only to size table columns
/// (see `Renderer::allocate_column_widths`/`wrap_cell`) — everything else
/// renders at its natural width and relies on the caller wrapping with
/// ratatui's own `Wrap` widget. Tables can't use that: `Wrap` breaks
/// lines at arbitrary points to fit the pane, which mid-line-shreds a
/// box-drawn table's borders. Sizing the table to fit *before* `Wrap`
/// ever sees it (every rendered table line is exactly the same width)
/// keeps `Wrap` a no-op for table rows.
pub fn render(source: &str, width: u16) -> Vec<Line<'static>> {
    let mut renderer = Renderer::new(width as usize);
    for event in Parser::new_ext(source, Options::ENABLE_TABLES) {
        renderer.handle(event);
    }
    renderer.finish()
}

struct Renderer {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    /// One entry per nesting level of `[[List]]`; `Some(n)` is an ordered
    /// list's next item number, `None` is an unordered (bulleted) list.
    list_stack: Vec<Option<u64>>,
    in_code_block: bool,
    /// Per-column alignment for the table currently being collected, set
    /// from `Tag::Table` and read back once the whole table (every row's
    /// cells) has been gathered — column widths depend on every row, so
    /// nothing can be emitted to `lines` until `TagEnd::Table`.
    table_alignments: Vec<Alignment>,
    table_rows: Vec<Vec<Vec<Span<'static>>>>,
    current_row: Vec<Vec<Span<'static>>>,
    available_width: usize,
}

impl Renderer {
    fn new(available_width: usize) -> Self {
        Self {
            lines: Vec::new(),
            current: Vec::new(),
            style_stack: vec![Style::default()],
            list_stack: Vec::new(),
            in_code_block: false,
            table_alignments: Vec::new(),
            table_rows: Vec::new(),
            current_row: Vec::new(),
            available_width,
        }
    }

    fn style(&self) -> Style {
        *self.style_stack.last().unwrap_or(&Style::default())
    }

    fn flush_line(&mut self) {
        self.lines.push(Line::from(std::mem::take(&mut self.current)));
    }

    fn flush_line_if_nonempty(&mut self) {
        if !self.current.is_empty() {
            self.flush_line();
        }
    }

    fn push_text(&mut self, text: CowStr<'_>, style: Style) {
        self.current.push(Span::styled(text.into_string(), style));
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag_end) => self.end(tag_end),
            Event::Text(text) => {
                if self.in_code_block {
                    let style = self.style();
                    let mut first = true;
                    for part in text.split('\n') {
                        if !first {
                            self.flush_line();
                        }
                        first = false;
                        if !part.is_empty() {
                            self.current.push(Span::styled(part.to_string(), style));
                        }
                    }
                } else {
                    let style = self.style();
                    self.push_text(text, style);
                }
            }
            Event::Code(text) => {
                self.push_text(text, Style::default().fg(Color::Green));
            }
            // CommonMark treats a single newline inside a paragraph as a
            // "soft break" — conventionally rendered as a space, folding
            // the line into the paragraph around it (needs a blank line,
            // not just Enter, to start a new paragraph). Deliberately not
            // followed here: for a note-taking tool where notes are often
            // short fragments (commands, lists) typed one Enter at a
            // time rather than hard-wrapped prose, "what you typed is
            // what you see" is a friendlier default than requiring a
            // blank line for every line break. The vault file on disk is
            // untouched either way — this only changes how the body
            // preview pane renders it.
            Event::SoftBreak | Event::HardBreak => {
                self.flush_line();
            }
            Event::Rule => {
                self.flush_line_if_nonempty();
                self.lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.flush_line_if_nonempty(),
            Tag::Heading { level, .. } => {
                self.flush_line_if_nonempty();
                if !self.lines.is_empty() {
                    self.lines.push(Line::default());
                }
                let color = match level {
                    HeadingLevel::H1 => Color::Cyan,
                    HeadingLevel::H2 => Color::Blue,
                    _ => Color::Magenta,
                };
                self.style_stack
                    .push(Style::default().fg(color).add_modifier(Modifier::BOLD));
            }
            Tag::Emphasis => {
                let style = self.style().add_modifier(Modifier::ITALIC);
                self.style_stack.push(style);
            }
            Tag::Strong => {
                let style = self.style().add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
            }
            Tag::CodeBlock(_) => {
                self.flush_line_if_nonempty();
                self.in_code_block = true;
                self.style_stack.push(Style::default().fg(Color::Green));
            }
            Tag::BlockQuote(_) => {
                self.flush_line_if_nonempty();
                let style = self
                    .style()
                    .add_modifier(Modifier::DIM)
                    .add_modifier(Modifier::ITALIC);
                self.style_stack.push(style);
            }
            Tag::Table(alignments) => {
                self.flush_line_if_nonempty();
                self.table_alignments = alignments;
                self.table_rows.clear();
            }
            // Header cells are direct `TableCell` children of `TableHead`,
            // not wrapped in their own `TableRow` (see pulldown-cmark's
            // `TableHead` doc comment) — reset here too, not just on
            // `TableRow`.
            Tag::TableHead | Tag::TableRow => self.current_row = Vec::new(),
            Tag::List(start) => self.list_stack.push(start),
            Tag::Item => {
                self.flush_line_if_nonempty();
                let marker = match self.list_stack.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{n}. ");
                        *n += 1;
                        m
                    }
                    _ => "• ".to_string(),
                };
                let indent = "  ".repeat(self.list_stack.len().saturating_sub(1));
                let style = self.style();
                self.current
                    .push(Span::styled(format!("{indent}{marker}"), style));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Paragraph | TagEnd::Item => self.flush_line_if_nonempty(),
            TagEnd::Heading(_) => {
                self.flush_line_if_nonempty();
                self.style_stack.pop();
            }
            TagEnd::Emphasis | TagEnd::Strong => {
                self.style_stack.pop();
            }
            TagEnd::CodeBlock => {
                self.flush_line_if_nonempty();
                self.style_stack.pop();
                self.in_code_block = false;
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line_if_nonempty();
                self.style_stack.pop();
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.current);
                self.current_row.push(cell);
            }
            TagEnd::TableHead => {
                let header: Vec<Vec<Span<'static>>> = std::mem::take(&mut self.current_row)
                    .into_iter()
                    .map(|cell| {
                        cell.into_iter()
                            .map(|s| Span::styled(s.content, s.style.add_modifier(Modifier::BOLD)))
                            .collect()
                    })
                    .collect();
                self.table_rows.push(header);
            }
            TagEnd::TableRow => {
                let row = std::mem::take(&mut self.current_row);
                self.table_rows.push(row);
            }
            TagEnd::Table => self.render_table(),
            _ => {}
        }
    }

    fn table_border(widths: &[usize], left: char, mid: char, right: char) -> Line<'static> {
        let style = Style::default().add_modifier(Modifier::DIM);
        let mut spans = vec![Span::styled(left.to_string(), style)];
        for (i, width) in widths.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(mid.to_string(), style));
            }
            spans.push(Span::styled("─".repeat(width + 2), style));
        }
        spans.push(Span::styled(right.to_string(), style));
        Line::from(spans)
    }

    /// `row` here is already one visual line per column — a cell that
    /// wrapped to several lines contributes one call per sub-line, each
    /// with the other columns' cells padded with an empty entry (see
    /// `render_table`). `wrap_cell` guarantees every sub-line already
    /// fits within `width`, so padding here is never negative.
    fn table_row_line(
        row: &[Vec<Span<'static>>],
        widths: &[usize],
        alignments: &[Alignment],
    ) -> Line<'static> {
        let border_style = Style::default().add_modifier(Modifier::DIM);
        let mut spans = vec![Span::styled("│".to_string(), border_style)];
        for (i, &width) in widths.iter().enumerate() {
            let cell = row.get(i).cloned().unwrap_or_default();
            let text_len = Self::cell_width(&cell);
            let pad = width.saturating_sub(text_len);
            let (left_pad, right_pad) = match alignments.get(i) {
                Some(Alignment::Right) => (pad, 0),
                Some(Alignment::Center) => (pad / 2, pad - pad / 2),
                _ => (0, pad),
            };
            spans.push(Span::raw(" ".repeat(1 + left_pad)));
            spans.extend(cell);
            spans.push(Span::raw(" ".repeat(right_pad + 1)));
            spans.push(Span::styled("│".to_string(), border_style));
        }
        Line::from(spans)
    }

    /// Shrinks `ideal` (every column's natural, unwrapped content width)
    /// to fit `available_width`, proportionally by each column's share of
    /// the total — a wide "Notes" column gives up more space than a
    /// narrow "Status" one. Every column's floor-divided share sums to
    /// at most `budget`, so the whole table is guaranteed to fit
    /// `available_width` — `wrap_cell` below hard-breaks any word too
    /// long for its column, so there's no minimum column width to
    /// respect here (unlike a version of this that left long words
    /// unbroken). Left alone (table already fits, or the pane is too
    /// narrow to do anything useful with), returns `ideal` unchanged,
    /// which is what keeps existing narrow tables pixel-for-pixel
    /// identical to before this shrinking existed.
    fn allocate_column_widths(ideal: &[usize], available_width: usize) -> Vec<usize> {
        let col_count = ideal.len();
        if col_count == 0 {
            return Vec::new();
        }
        // Every column costs 3 chars of border/padding overhead (`│ ` on
        // the left, ` ` on the right) plus the table's own left border.
        let overhead = 3 * col_count + 1;
        let ideal_total: usize = ideal.iter().sum();
        if available_width <= overhead || ideal_total + overhead <= available_width {
            return ideal.to_vec();
        }
        let budget = available_width - overhead;
        ideal
            .iter()
            .map(|&w| ((w * budget) / ideal_total.max(1)).max(1))
            .collect()
    }

    /// Terminal display width of a cell's text — *not* `str::chars().count()`,
    /// which undercounts anything the terminal renders double-wide (CJK
    /// characters, most emoji: `❌`/`✅` are each a single `char` but occupy
    /// two columns). Using char count here was the original bug: it sized
    /// columns and padding for a table that only matched the terminal's
    /// actual layout when every cell was single-width ASCII — a table with
    /// emoji or CJK content silently drifted the borders out of alignment
    /// row by row. `ratatui`'s own `Wrap` widget measures width the same
    /// way (it depends on `unicode-width` too), so this keeps our notion
    /// of "how wide is this line" consistent with what actually lands on
    /// screen.
    fn cell_width(cell: &[Span<'static>]) -> usize {
        cell.iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum()
    }

    /// Word-wrap of one cell's spans to `width` display columns, styling
    /// preserved per word/fragment. A word wider than `width` on its own
    /// is hard-broken (no hyphenation) rather than left to overflow —
    /// that's what guarantees every line `render_table` emits is exactly
    /// `width` columns wide, however narrow the pane or long a single
    /// token (e.g. a URL) is. The break point is chosen by accumulated
    /// *display* width, not character count, so a double-wide character
    /// never gets split across the boundary in a way that overshoots
    /// `width` by one column. Always returns at least one (possibly
    /// empty) line, so every cell contributes at least one row to the
    /// table.
    fn wrap_cell(cell: &[Span<'static>], width: usize) -> Vec<Vec<Span<'static>>> {
        let width = width.max(1);
        // (text, style, glued) — `glued` marks a continuation fragment
        // of a hard-broken word, which never gets a separating space.
        let mut tokens: Vec<(String, Style, bool)> = Vec::new();
        for span in cell {
            for word in span.content.split_whitespace() {
                if UnicodeWidthStr::width(word) <= width {
                    tokens.push((word.to_string(), span.style, false));
                    continue;
                }
                let mut chunk = String::new();
                let mut chunk_width = 0usize;
                let mut glued = false;
                for c in word.chars() {
                    let cw = UnicodeWidthChar::width(c).unwrap_or(0);
                    if !chunk.is_empty() && chunk_width + cw > width {
                        tokens.push((std::mem::take(&mut chunk), span.style, glued));
                        glued = true;
                        chunk_width = 0;
                    }
                    chunk.push(c);
                    chunk_width += cw;
                }
                if !chunk.is_empty() {
                    tokens.push((chunk, span.style, glued));
                }
            }
        }

        let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
        let mut current: Vec<Span<'static>> = Vec::new();
        let mut current_len = 0usize;
        for (text, style, glued) in tokens {
            let token_len = UnicodeWidthStr::width(text.as_str());
            let sep = usize::from(!glued && !current.is_empty());
            if !current.is_empty() && current_len + sep + token_len > width {
                lines.push(std::mem::take(&mut current));
                current_len = 0;
            }
            let sep = usize::from(!glued && !current.is_empty());
            if sep == 1 {
                current.push(Span::raw(" "));
                current_len += 1;
            }
            current.push(Span::styled(text, style));
            current_len += token_len;
        }
        if !current.is_empty() {
            lines.push(current);
        }
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        lines
    }

    /// Column widths depend on every cell in the table, so nothing here
    /// can be emitted until the whole table has been collected — unlike
    /// every other block, which streams straight to `self.lines`.
    fn render_table(&mut self) {
        let rows = std::mem::take(&mut self.table_rows);
        let alignments = std::mem::take(&mut self.table_alignments);
        if rows.is_empty() {
            return;
        }
        let col_count = rows
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(0)
            .max(alignments.len());
        let mut ideal = vec![0usize; col_count];
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                ideal[i] = ideal[i].max(Self::cell_width(cell));
            }
        }
        let widths = Self::allocate_column_widths(&ideal, self.available_width);
        self.lines.push(Self::table_border(&widths, '┌', '┬', '┐'));
        for (i, row) in rows.iter().enumerate() {
            let empty = Vec::new();
            let wrapped: Vec<Vec<Vec<Span<'static>>>> = (0..col_count)
                .map(|c| Self::wrap_cell(row.get(c).unwrap_or(&empty), widths[c]))
                .collect();
            let row_height = wrapped.iter().map(|c| c.len()).max().unwrap_or(1).max(1);
            for sub in 0..row_height {
                let line_cells: Vec<Vec<Span<'static>>> = wrapped
                    .iter()
                    .map(|c| c.get(sub).cloned().unwrap_or_default())
                    .collect();
                self.lines
                    .push(Self::table_row_line(&line_cells, &widths, &alignments));
            }
            if i == 0 {
                self.lines.push(Self::table_border(&widths, '├', '┼', '┤'));
            }
        }
        self.lines.push(Self::table_border(&widths, '└', '┴', '┘'));
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line_if_nonempty();
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn renders_a_plain_paragraph_as_one_line() {
        let lines = render("Just a paragraph.", 80);
        assert_eq!(plain_text(&lines), vec!["Just a paragraph."]);
    }

    #[test]
    fn separates_paragraphs_onto_their_own_lines() {
        let lines = render("First paragraph.\n\nSecond paragraph.", 80);
        assert_eq!(
            plain_text(&lines),
            vec!["First paragraph.", "Second paragraph."]
        );
    }

    #[test]
    fn a_single_newline_within_a_paragraph_becomes_its_own_line_too() {
        // CommonMark's own rule treats this as a "soft break" — folded
        // into the same line as a space, not a real line break —
        // deliberately not followed here (see `Renderer::handle`'s
        // `SoftBreak` arm): a note-taking body is more often short
        // fragments typed one Enter at a time than hard-wrapped prose,
        // so what you typed is what you see takes priority.
        let lines = render("First line.\nSecond line.", 80);
        assert_eq!(plain_text(&lines), vec!["First line.", "Second line."]);
    }

    #[test]
    fn bold_and_italic_get_distinct_styling() {
        let lines = render("Some **bold** and *italic* text.", 80);
        assert_eq!(lines.len(), 1);
        let bold_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "bold")
            .unwrap();
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
        let italic_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "italic")
            .unwrap();
        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn heading_gets_bold_and_a_leading_blank_line_when_not_first() {
        let lines = render("Intro.\n\n# Heading\n\nBody.", 80);
        let text = plain_text(&lines);
        assert!(text.contains(&"Heading".to_string()));
        let heading_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.as_ref() == "Heading"))
            .unwrap();
        assert!(heading_line.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn bullet_list_items_get_a_marker() {
        let lines = render("- one\n- two", 80);
        let text = plain_text(&lines);
        assert_eq!(text, vec!["• one", "• two"]);
    }

    #[test]
    fn ordered_list_items_are_numbered_in_order() {
        let lines = render("1. first\n2. second\n3. third", 80);
        let text = plain_text(&lines);
        assert_eq!(text, vec!["1. first", "2. second", "3. third"]);
    }

    #[test]
    fn inline_code_is_styled_distinctly_from_surrounding_text() {
        let lines = render("Run `cargo test` now.", 80);
        let code_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "cargo test")
            .unwrap();
        assert_eq!(code_span.style.fg, Some(Color::Green));
    }

    #[test]
    fn code_block_lines_stay_separate() {
        let lines = render("```\nline one\nline two\n```", 80);
        let text = plain_text(&lines);
        assert_eq!(text, vec!["line one", "line two"]);
    }

    #[test]
    fn empty_body_renders_no_lines() {
        assert!(render("", 80).is_empty());
    }

    #[test]
    fn table_renders_as_a_bordered_grid_with_a_header_separator() {
        let lines = render("| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |", 80);
        let text = plain_text(&lines);
        assert_eq!(
            text,
            vec![
                "┌───────┬─────┐",
                "│ Name  │ Age │",
                "├───────┼─────┤",
                "│ Alice │ 30  │",
                "│ Bob   │ 25  │",
                "└───────┴─────┘",
            ]
        );
    }

    #[test]
    fn table_header_cells_are_bold() {
        let lines = render("| Name |\n| --- |\n| Alice |", 80);
        let header_span = lines[1]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "Name")
            .unwrap();
        assert!(header_span.style.add_modifier.contains(Modifier::BOLD));
        let body_span = lines[3]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "Alice")
            .unwrap();
        assert!(!body_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn table_columns_respect_alignment_markers() {
        let lines = render(
            "| Left | Right | Center |\n| :--- | ---: | :---: |\n| a | b | c |",
            80,
        );
        let text = plain_text(&lines);
        assert_eq!(text[3], "│ a    │     b │   c    │");
    }

    #[test]
    fn table_wraps_cell_text_to_fit_a_narrow_pane_without_breaking_borders() {
        let lines = render(
            "| Name | Description |\n| --- | --- |\n\
             | Widget | A fairly long description that needs wrapping |",
            30,
        );
        let text = plain_text(&lines);
        // Every rendered line here — borders and wrapped cell content
        // alike — is exactly as wide as the table itself. That's what
        // keeps ratatui's `Wrap` widget (applied by the caller on top of
        // this) from ever having to split a table line mid-border: as
        // long as every line reports the same width, `Wrap` treats the
        // whole table as already fitting and leaves it alone.
        let table_width = text[0].chars().count();
        assert!(text.iter().all(|line| line.chars().count() == table_width));
        // The long description really did wrap into more than one line.
        assert!(text.len() > 5);
    }

    #[test]
    fn table_hard_breaks_a_word_too_long_for_its_column() {
        // A single unbroken word longer than any reasonable column width
        // (e.g. a URL) must not blow past the pane — every line's width
        // is the invariant `ui.rs` relies on to keep ratatui's `Wrap`
        // widget from ever touching a table line.
        let lines = render(
            "| Col |\n| --- |\n| Supercalifragilisticexpialidocious |",
            12,
        );
        let text = plain_text(&lines);
        let table_width = text[0].chars().count();
        assert!(table_width <= 12);
        assert!(text.iter().all(|line| line.chars().count() == table_width));
        assert!(text.len() > 4);
    }

    #[test]
    fn table_columns_stay_aligned_with_double_width_emoji_cells() {
        // ❌/✅ are each a single `char` but occupy two terminal columns —
        // sizing columns by `chars().count()` (the original bug) rendered
        // this table's borders visibly out of alignment starting on the
        // first emoji row. `display_width` below mirrors `cell_width` in
        // production code (both back onto `unicode_width`), so this
        // assertion is checking real on-screen alignment, not just that
        // every rendered `Line`'s `char` count happens to match.
        fn display_width(s: &str) -> usize {
            unicode_width::UnicodeWidthStr::width(s)
        }
        let lines = render(
            "| Fonctionnalité | `cat` | `bat` |\n| --- | --- | --- |\n\
             | Coloration syntaxique | ❌ | ✅ |\n\
             | Numéros de ligne | ❌ | ✅ |",
            80,
        );
        let text = plain_text(&lines);
        let table_width = display_width(&text[0]);
        assert!(text.iter().all(|line| display_width(line) == table_width));
    }
}
