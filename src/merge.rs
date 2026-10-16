//! Merge: dedup by normalized URL, round-robin across engines (preserves
//! configured engine order/weight). No result cap: the full fan-out is
//! returned like the reference (40-115/q typical). Positions assigned
//! in merged order (SearXNG-compatible `positions` field).

use crate::engine::{normalize_url, SearchResult};
use std::collections::HashSet;

pub fn merge(lists: Vec<Vec<SearchResult>>) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut idx = vec![0usize; lists.len()];
    loop {
        let mut progressed = false;
        for (li, list) in lists.iter().enumerate() {
            while idx[li] < list.len() {
                let r = &list[idx[li]];
                idx[li] += 1;
                let key = normalize_url(&r.url);
                if key.is_empty() || !seen.insert(key) {
                    continue;
                }
                let mut r = r.clone();
                r.positions = vec![out.len() + 1];
                out.push(r);
                progressed = true;
                break;
            }
        }
        if !progressed {
            break;
        }
    }
    out
}

/// Query terms: lowercase alphanumeric tokens, len>=2 (drops "a"/"is",
/// keeps "go"/"3"). Pure function of the query string.
fn terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

/// Relevance score: +2 per query term present in the title, +1 per term
/// in the content (presence, not frequency — smoother across snippet
/// lengths). Empty title+content scores 0 and sinks naturally; nothing
/// is ever dropped, only reordered.
fn score(r: &SearchResult, terms: &[String]) -> u32 {
    if terms.is_empty() {
        return 0;
    }
    let title = r.title.to_lowercase();
    let content = r.content.to_lowercase();
    terms
        .iter()
        .map(|t| {
            (if title.contains(t) { 2 } else { 0 }) + (if content.contains(t) { 1 } else { 0 })
        })
        .sum()
}

/// Relevance re-rank over the merged list. STABLE sort (ties keep merge
/// order = conf weights), then positions renumbered 1..n so the
/// `positions` field matches the served order.
pub fn rank(mut results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
    let ts = terms(query);
    if ts.is_empty() {
        return results;
    }
    results.sort_by_key(|r| std::cmp::Reverse(score(r, &ts)));
    for (i, r) in results.iter_mut().enumerate() {
        r.positions = vec![i + 1];
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(url: &str, title: &str, content: &str) -> SearchResult {
        SearchResult::basic("t", url.into(), title.into(), content.into())
    }

    #[test]
    fn normalizer_collapses_trackers() {
        let a = normalize_url("https://Example.com/Article/?utm_source=x#sec");
        let b = normalize_url("http://www.example.com/article?fbclid=1");
        let c = normalize_url("https://example.com/article");
        assert_eq!(a, b);
        assert_eq!(b, c);
        // Meaningful params survive: ?v= is a different video.
        assert_ne!(normalize_url("https://yt.com/w?v=1"), normalize_url("https://yt.com/w?v=2"));
    }

    #[test]
    fn rank_title_beats_content() {
        let rows = vec![
            row("http://a/1", "unrelated", "flask request context everywhere"),
            row("http://a/2", "flask request context", "other words"),
        ];
        let ranked = rank(rows, "flask request context");
        assert_eq!(ranked[0].url, "http://a/2");
        assert_eq!(ranked[0].positions, vec![1]);
        assert_eq!(ranked[1].positions, vec![2]);
    }

    #[test]
    fn rank_ties_keep_merge_order_and_empties_sink() {
        let rows = vec![
            row("http://a/1", "flask guide", "flask"),
            row("http://a/2", "", ""),
            row("http://a/3", "flask guide", "flask"),
        ];
        let ranked = rank(rows, "flask");
        assert_eq!(ranked[0].url, "http://a/1"); // tie: merge order held
        assert_eq!(ranked[1].url, "http://a/3");
        assert_eq!(ranked[2].url, "http://a/2"); // empty sank
    }

    #[test]
    fn rank_empty_query_is_identity() {
        let rows = vec![row("http://a/1", "b", "c")];
        assert_eq!(rank(rows, "?!").len(), 1);
    }
}
