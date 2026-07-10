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
}
