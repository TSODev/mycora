/// How confident `suggest` is in a candidate replacement title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// A case-insensitive exact match — Mycora's own title matching is
    /// case-sensitive, so this is the single most likely real cause of a
    /// broken link (e.g. `[[commandes]]` vs a note titled "Commandes").
    Certain,
    /// No exact match (case-insensitive or otherwise), but a single
    /// candidate scored clearly above every other one.
    Likely,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub title: String,
    pub confidence: Confidence,
}

/// Below this `strsim::jaro_winkler` score (on lowercased titles), a
/// candidate isn't considered close enough to guess at all.
const SIMILARITY_THRESHOLD: f64 = 0.85;
/// The best candidate must beat the second-best by at least this much, or
/// the match is ambiguous (two similarly-named notes) and `suggest`
/// returns `None` rather than picking one.
const AMBIGUITY_MARGIN: f64 = 0.05;

/// Best-guess replacement for `broken_title` among `candidates` (every
/// note title across every loaded vault, snapshotted before any fix is
/// applied — see `main.rs`'s `perform_repair`). Never guesses when it
/// isn't reasonably sure: below `SIMILARITY_THRESHOLD`, or within
/// `AMBIGUITY_MARGIN` of a second candidate, returns `None`.
pub fn suggest(broken_title: &str, candidates: &[String]) -> Option<Suggestion> {
    let lower_broken = broken_title.to_lowercase();

    if let Some(exact) = candidates
        .iter()
        .find(|candidate| candidate.to_lowercase() == lower_broken)
    {
        return Some(Suggestion {
            title: exact.clone(),
            confidence: Confidence::Certain,
        });
    }

    let mut scored: Vec<(f64, &String)> = candidates
        .iter()
        .map(|candidate| {
            (
                strsim::jaro_winkler(&lower_broken, &candidate.to_lowercase()),
                candidate,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    let &(best_score, best_title) = scored.first()?;
    if best_score < SIMILARITY_THRESHOLD {
        return None;
    }
    let runner_up = scored.get(1).map(|&(score, _)| score).unwrap_or(0.0);
    if best_score - runner_up < AMBIGUITY_MARGIN {
        return None;
    }
    Some(Suggestion {
        title: best_title.clone(),
        confidence: Confidence::Likely,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn finds_a_case_insensitive_exact_match() {
        let candidates = titles(&["Commandes", "Other"]);
        let suggestion = suggest("commandes", &candidates).unwrap();
        assert_eq!(suggestion.title, "Commandes");
        assert_eq!(suggestion.confidence, Confidence::Certain);
    }

    #[test]
    fn finds_a_close_typo_as_likely() {
        let candidates = titles(&["Commandes", "Unrelated Note"]);
        let suggestion = suggest("Comandes", &candidates).unwrap();
        assert_eq!(suggestion.title, "Commandes");
        assert_eq!(suggestion.confidence, Confidence::Likely);
    }

    #[test]
    fn returns_none_when_nothing_is_close() {
        let candidates = titles(&["Commandes", "Layout", "Undo and redo"]);
        assert!(suggest("rowdy-db", &candidates).is_none());
    }

    #[test]
    fn returns_none_when_two_candidates_are_equally_close() {
        let candidates = titles(&["Commande", "Commandes"]);
        assert!(suggest("Commandex", &candidates).is_none());
    }

    #[test]
    fn returns_none_with_no_candidates() {
        assert!(suggest("anything", &[]).is_none());
    }
}
