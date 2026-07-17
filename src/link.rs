/// Extracts the raw title text inside every `[[title]]` occurrence in
/// `body`, in order. Resolving those titles to actual notes (and deciding
/// what to do with ambiguous or missing targets) is `Index::reindex`'s job
/// — this function only does syntax extraction, no lookup.
///
/// An unclosed `[[` (no matching `]]` before the end of the body) stops
/// parsing at that point rather than erroring: malformed wikilink syntax
/// in a note body should never break indexing.
pub fn extract_wikilink_titles(body: &str) -> Vec<String> {
    let mut titles = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("]]") else {
            break;
        };
        let title = after_open[..end].trim();
        if !title.is_empty() {
            titles.push(title.to_string());
        }
        rest = &after_open[end + 2..];
    }
    titles
}

/// If there's an unclosed `[[` before `cursor_col` on `line` (no `]]`
/// between it and the cursor), returns the character index right after
/// it — the start of the in-progress wikilink title. Backs the body
/// editor's autocomplete popup: whether to show it at all, what to
/// filter suggestions by, and how many characters to remove when one is
/// accepted. Scoped to a single line, unlike `extract_wikilink_titles`'s
/// whole-body scan — a title being typed is always still on the line it
/// was opened on, and restricting the scan avoids matching a stray,
/// already-abandoned `[[` from an earlier paragraph. `cursor_col` and
/// the returned index are both character counts (not byte offsets),
/// matching `ratatui-textarea`'s own cursor addressing. If more than one
/// `[[` opens before the cursor with no `]]` in between, the most recent
/// one wins — the same "latest unclosed one" instinct as leaving an
/// earlier bracket open by mistake and continuing to type.
pub fn unclosed_wikilink_start(line: &str, cursor_col: usize) -> Option<usize> {
    let prefix: Vec<char> = line.chars().take(cursor_col).collect();
    let mut open_at: Option<usize> = None;
    let mut i = 0;
    while i + 1 < prefix.len() {
        if prefix[i] == '[' && prefix[i + 1] == '[' {
            open_at = Some(i + 2);
            i += 2;
        } else if prefix[i] == ']' && prefix[i + 1] == ']' {
            open_at = None;
            i += 2;
        } else {
            i += 1;
        }
    }
    open_at
}

/// Rewrites every `[[old_title]]` occurrence in `body` to `[[new_title]]`
/// — same trimmed-title matching `extract_wikilink_titles` uses, so this
/// only ever touches a link that function would also report. Backs
/// `mycora repair --apply`'s retargeting of a broken link to its resolved
/// title. Same naive scan-and-rebuild idiom as `extract_wikilink_titles`
/// itself: an unclosed `[[` stops rewriting and passes the rest of the
/// body through unchanged rather than erroring.
pub fn rewrite_wikilink_title(body: &str, old_title: &str, new_title: &str) -> String {
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
        let title = after_open[..end].trim();
        out.push_str("[[");
        out.push_str(if title == old_title { new_title } else { title });
        out.push_str("]]");
        rest = &after_open[end + 2..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_a_single_wikilink() {
        assert_eq!(
            extract_wikilink_titles("see [[Other Note]] for more"),
            vec!["Other Note"]
        );
    }

    #[test]
    fn extracts_multiple_wikilinks_in_order() {
        assert_eq!(
            extract_wikilink_titles("[[A]] then [[B]] then [[C]]"),
            vec!["A", "B", "C"]
        );
    }

    #[test]
    fn ignores_text_without_wikilinks() {
        assert!(extract_wikilink_titles("just plain text").is_empty());
    }

    #[test]
    fn stops_at_an_unclosed_bracket_without_panicking() {
        assert_eq!(
            extract_wikilink_titles("[[A]] then [[unterminated"),
            vec!["A"]
        );
    }

    #[test]
    fn trims_whitespace_and_skips_empty_titles() {
        assert_eq!(
            extract_wikilink_titles("[[  Spaced Title  ]] and [[]]"),
            vec!["Spaced Title"]
        );
    }

    #[test]
    fn extracts_adjacent_wikilinks_with_no_separator() {
        assert_eq!(
            extract_wikilink_titles("[[A]][[B]][[C]]"),
            vec!["A", "B", "C"]
        );
    }

    #[test]
    fn ignores_a_stray_closing_bracket_before_the_first_wikilink() {
        assert_eq!(
            extract_wikilink_titles("see ]] over there, then [[Real Note]]"),
            vec!["Real Note"]
        );
    }

    #[test]
    fn a_nested_opening_bracket_is_swallowed_into_the_title_up_to_the_next_close() {
        // Documented, deliberate naive-scanner behavior (see this module's
        // doc comment and CLAUDE.md): once an unclosed `[[` is seen, the
        // scanner takes the *next* `]]` anywhere later as that link's
        // close, even past a second `[[` in between. Pinning this exact
        // shape so a future "smarter" rewrite doesn't silently change it
        // without the change being deliberate.
        assert_eq!(
            extract_wikilink_titles("[[Outer [[Inner]] tail]]"),
            vec!["Outer [[Inner"]
        );
    }

    #[test]
    fn rewrite_wikilink_title_replaces_a_single_occurrence() {
        assert_eq!(
            rewrite_wikilink_title("see [[commandes]] for details", "commandes", "Commandes"),
            "see [[Commandes]] for details"
        );
    }

    #[test]
    fn rewrite_wikilink_title_replaces_every_occurrence_of_the_same_title() {
        assert_eq!(
            rewrite_wikilink_title(
                "[[commandes]] then again [[commandes]]",
                "commandes",
                "Commandes"
            ),
            "[[Commandes]] then again [[Commandes]]"
        );
    }

    #[test]
    fn rewrite_wikilink_title_leaves_other_titles_untouched() {
        assert_eq!(
            rewrite_wikilink_title("[[commandes]] and [[Other Note]]", "commandes", "Commandes"),
            "[[Commandes]] and [[Other Note]]"
        );
    }

    #[test]
    fn rewrite_wikilink_title_stops_cleanly_at_an_unclosed_bracket() {
        assert_eq!(
            rewrite_wikilink_title("[[commandes]] then [[unterminated", "commandes", "Commandes"),
            "[[Commandes]] then [[unterminated"
        );
    }

    #[test]
    fn unclosed_wikilink_start_finds_the_title_start_right_after_double_bracket() {
        assert_eq!(unclosed_wikilink_start("[[", 2), Some(2));
        assert_eq!(unclosed_wikilink_start("[[Real", 6), Some(2));
    }

    #[test]
    fn unclosed_wikilink_start_is_none_once_the_link_is_closed() {
        assert_eq!(unclosed_wikilink_start("[[A]]", 5), None);
        assert_eq!(unclosed_wikilink_start("[[A]] typing after", 19), None);
    }

    #[test]
    fn unclosed_wikilink_start_is_none_with_no_brackets_at_all() {
        assert_eq!(unclosed_wikilink_start("just plain text", 10), None);
    }

    #[test]
    fn unclosed_wikilink_start_only_looks_before_the_cursor() {
        // A `]]` typed *after* the cursor shouldn't close a link the
        // cursor is still in front of.
        assert_eq!(unclosed_wikilink_start("[[Real]]", 6), Some(2));
    }

    #[test]
    fn unclosed_wikilink_start_prefers_the_most_recently_opened_bracket() {
        assert_eq!(unclosed_wikilink_start("[[A [[B", 7), Some(6));
    }
}
