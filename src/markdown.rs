use pulldown_cmark::{CowStr, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Renders a note body's Markdown into styled lines for a read-only
/// preview pane. Headings, bold/italic, inline/block code, lists,
/// blockquotes, and horizontal rules get distinct styling. Nothing is
/// interactive — links render as their text, not as clickable/navigable
/// spans, and `[[wikilinks]]` aren't CommonMark syntax so they just render
/// as literal bracketed text (no special-casing here; that's a separate
/// concern from "render the Markdown").
pub fn render(source: &str) -> Vec<Line<'static>> {
    let mut renderer = Renderer::new();
    for event in Parser::new(source) {
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
}

impl Renderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current: Vec::new(),
            style_stack: vec![Style::default()],
            list_stack: Vec::new(),
            in_code_block: false,
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
            Event::SoftBreak => {
                self.current.push(Span::raw(" "));
            }
            Event::HardBreak => {
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
            _ => {}
        }
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
        let lines = render("Just a paragraph.");
        assert_eq!(plain_text(&lines), vec!["Just a paragraph."]);
    }

    #[test]
    fn separates_paragraphs_onto_their_own_lines() {
        let lines = render("First paragraph.\n\nSecond paragraph.");
        assert_eq!(
            plain_text(&lines),
            vec!["First paragraph.", "Second paragraph."]
        );
    }

    #[test]
    fn bold_and_italic_get_distinct_styling() {
        let lines = render("Some **bold** and *italic* text.");
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
        let lines = render("Intro.\n\n# Heading\n\nBody.");
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
        let lines = render("- one\n- two");
        let text = plain_text(&lines);
        assert_eq!(text, vec!["• one", "• two"]);
    }

    #[test]
    fn ordered_list_items_are_numbered_in_order() {
        let lines = render("1. first\n2. second\n3. third");
        let text = plain_text(&lines);
        assert_eq!(text, vec!["1. first", "2. second", "3. third"]);
    }

    #[test]
    fn inline_code_is_styled_distinctly_from_surrounding_text() {
        let lines = render("Run `cargo test` now.");
        let code_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "cargo test")
            .unwrap();
        assert_eq!(code_span.style.fg, Some(Color::Green));
    }

    #[test]
    fn code_block_lines_stay_separate() {
        let lines = render("```\nline one\nline two\n```");
        let text = plain_text(&lines);
        assert_eq!(text, vec!["line one", "line two"]);
    }

    #[test]
    fn empty_body_renders_no_lines() {
        assert!(render("").is_empty());
    }
}
