//! SearchBar finite-state machine (§4).
use alloc::string::String;

/// The five SearchBar states from spec §4.
///
/// Note: `Loading` covers both the initial-load case (no rows yet) and
/// the load-more case (rows present + footer spinner) — the visibility
/// table in §4 distinguishes these by the `pending_load_more` flag, NOT
/// by adding a separate state. `Empty` covers both literally-empty
/// queries and queries shorter than `min_query_len` (TOO_SHORT bucket
/// per §4 normalization rules).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Empty,
    Loading,
    Results,
    NoResults,
    Error,
}

/// A request token. Monotonically incremented every time a NEW query is
/// fired (after dedupe + min_query_len gates) OR the query is cleared.
/// Late callbacks are dropped if their token does not match
/// `current_token` (gate condition 1, §4).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Token(pub u64);

#[derive(Clone, Debug)]
pub struct StateSnapshot {
    pub state: State,
    pub current_token: Token,
    /// Canonical form of the query that was last *fired* (callback emitted).
    /// Reset to "" on clear or empty/too-short pivot. Acceptance gate
    /// condition 2 (§4); fixes risk #41 (clear-then-retype-same-string
    /// must re-fire the callback).
    pub last_fired_canonical: String,
    /// Set when `on_load_more` has been emitted but no `append_results`
    /// has resolved it yet. NOT a state — a flag (§7).
    pub pending_load_more: bool,
    /// Source state recorded the moment `set_error(token, true)` is
    /// accepted, so `set_error(token, false)` can deterministically
    /// restore (§4 normalization rule for set_error).
    pub pre_error_state: Option<State>,
    /// Liveness flag set to false in `LV_EVENT_DELETE` step 0; every
    /// public setter checks it before touching the RefCell (risk #52).
    pub alive: bool,
    /// Number of replies discarded by the gate. Observable for tests
    /// (`searchbar.stale_drop_count()`).
    pub stale_drop_count: u64,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self {
            state: State::Empty,
            current_token: Token(0),
            last_fired_canonical: String::new(),
            pending_load_more: false,
            pre_error_state: None,
            alive: true,
            stale_drop_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_state_is_empty() {
        let s = StateSnapshot::default();
        assert_eq!(s.state, State::Empty);
        assert_eq!(s.current_token, Token(0));
        assert_eq!(s.stale_drop_count, 0);
        assert!(s.last_fired_canonical.is_empty());
        assert!(!s.pending_load_more);
        assert!(s.pre_error_state.is_none());
        assert!(s.alive);
    }
    #[test]
    fn token_equality() {
        assert_eq!(Token(5), Token(5));
        assert_ne!(Token(5), Token(6));
    }
    #[test]
    fn states_are_distinct() {
        let all = [
            State::Empty,
            State::Loading,
            State::Results,
            State::NoResults,
            State::Error,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                assert_eq!(
                    a == b,
                    i == j,
                    "state equality failed for {:?} vs {:?}",
                    a,
                    b
                );
            }
        }
    }
}
