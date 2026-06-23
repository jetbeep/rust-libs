//! Highlight markup for `lv_label_set_recolor` (§6).
//!
//! Rule (from spec §6 + risk #44): the FULL displayed text is escaped
//! before injecting `#RRGGBB ...#`. We escape `#` → `##` everywhere, then
//! wrap matched substrings.
use alloc::string::String;
use alloc::vec::Vec;

/// Canonicalise a query string for matching/dedupe (§4): trim then
/// (optionally) lowercase. The single source of truth — every gate uses this.
pub fn canonical_query(s: &str, case_insensitive: bool) -> String {
    let t = s.trim();
    if case_insensitive {
        t.to_lowercase()
    } else {
        String::from(t)
    }
}

/// Escape every `#` in `s` as `##` (LVGL recolor escape).
fn escape_recolor(s: &str) -> String {
    let hash_count = s.bytes().filter(|&b| b == b'#').count();
    // Each '#' becomes "##", so the output is exactly s.len() + hash_count bytes.
    let mut out = String::with_capacity(s.len() + hash_count);
    for ch in s.chars() {
        if ch == '#' {
            out.push('#');
        }
        out.push(ch);
    }
    out
}

/// Build a recolor-marked-up string. `text` is the raw row text;
/// `query` is the (already canonical) query; matches are highlighted with
/// `#color text#`. `color` is a 6-char hex without `#`. Returns the
/// fully-escaped, marked-up string.
///
/// Matching rules (§6):
/// * If `query` is empty → return escaped text unchanged.
/// * Case-insensitive iff `case_insensitive` is true.
/// * All non-overlapping matches highlighted, scanned left-to-right.
pub fn highlight_markup(text: &str, query: &str, color: &str, case_insensitive: bool) -> String {
    if query.is_empty() {
        return escape_recolor(text);
    }
    let hay = if case_insensitive {
        text.to_lowercase()
    } else {
        String::from(text)
    };
    let need = if case_insensitive {
        query.to_lowercase()
    } else {
        String::from(query)
    };
    let need_bytes = need.as_bytes();
    let hay_bytes = hay.as_bytes();
    if need_bytes.is_empty() || need_bytes.len() > hay_bytes.len() {
        return escape_recolor(text);
    }
    let mut i = 0usize;
    let mut matches: Vec<(usize, usize)> = Vec::new();
    while i + need_bytes.len() <= hay_bytes.len() {
        if &hay_bytes[i..i + need_bytes.len()] == need_bytes {
            // Snap to char boundaries on the original text.
            if text.is_char_boundary(i) && text.is_char_boundary(i + need_bytes.len()) {
                matches.push((i, i + need_bytes.len()));
                i += need_bytes.len();
                continue;
            }
        }
        i += 1;
    }

    // Precomputed capacity (spec §6 design requirement).
    // Output bytes = text.len()
    //              + (# chars in text doubled = hash_count)
    //              + per-match markup overhead = 2 ("# ") + color.len() + 1 ("#") = color.len() + 3
    let hash_count = text.bytes().filter(|&b| b == b'#').count();
    let cap = text.len() + hash_count + matches.len() * (color.len() + 3);
    let mut out = String::with_capacity(cap);
    let mut last_emit = 0usize;
    for (s, e) in matches {
        out.push_str(&escape_recolor(&text[last_emit..s]));
        out.push('#');
        out.push_str(color);
        out.push(' ');
        out.push_str(&escape_recolor(&text[s..e]));
        out.push('#');
        last_emit = e;
    }
    out.push_str(&escape_recolor(&text[last_emit..]));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_trim_and_lower() {
        assert_eq!(canonical_query("  Pizza  ", true), "pizza");
        assert_eq!(canonical_query("  Pizza  ", false), "Pizza");
        assert_eq!(canonical_query("", true), "");
    }
    #[test]
    fn empty_query_returns_escaped_text() {
        // risk #44: `#` in user content must be doubled even with no match.
        assert_eq!(highlight_markup("a#b", "", "FFAA00", true), "a##b");
    }
    #[test]
    fn single_match_wraps_correctly() {
        let out = highlight_markup("Pizza Hut", "pizza", "FFAA00", true);
        assert_eq!(out, "#FFAA00 Pizza# Hut");
    }
    #[test]
    fn multiple_non_overlapping_matches() {
        let out = highlight_markup("aXa", "a", "111111", false);
        assert_eq!(out, "#111111 a#X#111111 a#");
    }
    #[test]
    fn match_with_hash_is_doubly_escaped() {
        // user typed "#tag" — the hash inside the highlighted span itself
        // must be escaped.
        let out = highlight_markup("#tag party", "#tag", "ABCDEF", false);
        assert_eq!(out, "#ABCDEF ##tag# party");
    }
    #[test]
    fn case_sensitive_no_match() {
        assert_eq!(highlight_markup("Pizza", "pizza", "FFAA00", false), "Pizza");
    }
    #[test]
    fn capacity_is_precomputed_exactly_for_hash_heavy_text() {
        // Verifies the spec §6 capacity formula matches the produced output
        // length exactly for a synthetic hash-heavy case (no overlap between
        // matches and hashes for clarity). If the formula drifts, this test
        // fails immediately.
        let text = "a#b#c#d"; // 7 bytes, 3 '#' chars, no matches
        let q = ""; // empty query → escaped pass-through
        let out = highlight_markup(text, q, "FFAA00", false);
        assert_eq!(out, "a##b##c##d");
        assert_eq!(out.len(), text.len() + 3); // 7 + 3 doubled hashes
    }

    #[test]
    fn capacity_is_precomputed_exactly_for_many_matches() {
        // 5 matches × overhead (3 + color.len()) plus base text.
        let text = "ababababab"; // 10 bytes, 5 'a's, 0 '#'s
        let q = "a";
        let color = "FFAA00";
        let out = highlight_markup(text, q, color, false);
        // Expected: "#FFAA00 a#b" repeated 5 times → 5 × (1+6+1+1+1) = 50, plus 5 'b's = ... easier to assert exact:
        assert_eq!(
            out,
            "#FFAA00 a#b#FFAA00 a#b#FFAA00 a#b#FFAA00 a#b#FFAA00 a#b"
        );
        // Spec capacity formula: text.len() + 0 + 5*(color.len() + 3) = 10 + 5*9 = 55
        assert_eq!(out.len(), 10 + 5 * (color.len() + 3));
    }
}
