//! Selection model (§5). Selection lives as a `Vec<u64>` of row IDs in
//! InnerState.selected. We never emit `Select(_, false)` for IDs lost
//! to `set_results`/`append_results` — those are silently reconciled.
use super::action::{Action, Callback};
use super::inner::InnerState;
use super::row::SearchRow;
use alloc::vec::Vec;

pub fn select(s: &mut InnerState, row_id: u64) {
    if s.selected.contains(&row_id) {
        return;
    }
    s.selected.push(row_id);
    s.selection_dirty = true;
    s.queue.push(Action::EmitCallback(Callback::Select {
        row_id,
        selected: true,
    }));
}

pub fn deselect(s: &mut InnerState, row_id: u64) {
    let before = s.selected.len();
    s.selected.retain(|id| *id != row_id);
    if s.selected.len() != before {
        s.selection_dirty = true;
        s.queue.push(Action::EmitCallback(Callback::Select {
            row_id,
            selected: false,
        }));
    }
}

pub fn toggle(s: &mut InnerState, row_id: u64) {
    if s.selected.contains(&row_id) {
        deselect(s, row_id);
    } else {
        select(s, row_id);
    }
}

pub fn is_selected_id(s: &InnerState, row_id: u64) -> bool {
    s.selected.contains(&row_id)
}

pub fn selected_row_ids(s: &InnerState) -> Vec<u64> {
    s.selected.clone()
}
pub fn selected_count(s: &InnerState) -> usize {
    s.selected.len()
}

/// Silent clear — internal state only, no callback (§5). Marks
/// `selection_dirty` only if there was something to clear.
pub fn clear_selection(s: &mut InnerState) {
    if !s.selected.is_empty() {
        s.selected.clear();
        s.selection_dirty = true;
    }
}

/// Reconcile selection against `new_rows`. IDs not present in the union
/// of (existing rows, new rows) are silently dropped. Used after
/// `set_results` (replace) and `append_results` (extend).
pub fn reconcile(s: &mut InnerState, _: &[SearchRow]) {
    let valid: alloc::collections::BTreeSet<u64> = s.rows.iter().map(|r| r.id).collect();
    s.selected.retain(|id| valid.contains(id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::SpyFixture;

    fn st() -> InnerState {
        InnerState::new(true, 0, 200)
    }

    fn drain_queue(s: &mut InnerState) -> Vec<Action> {
        core::iter::from_fn(|| s.queue.pop_front()).collect()
    }

    #[test]
    fn select_emits_callback_once() {
        let _fx = SpyFixture::new();
        let mut s = st();
        select(&mut s, 7);
        select(&mut s, 7); // dup → no-op
        assert_eq!(s.selected, alloc::vec![7]);
        let drained = drain_queue(&mut s);
        assert_eq!(drained.len(), 1);
    }
    #[test]
    fn toggle_round_trip() {
        let _fx = SpyFixture::new();
        let mut s = st();
        toggle(&mut s, 1);
        toggle(&mut s, 1);
        assert!(s.selected.is_empty());
        assert_eq!(drain_queue(&mut s).len(), 2);
    }
    #[test]
    fn clear_selection_silent() {
        let _fx = SpyFixture::new();
        let mut s = st();
        select(&mut s, 1);
        select(&mut s, 2);
        let _ = drain_queue(&mut s);
        clear_selection(&mut s);
        assert!(s.selected.is_empty());
        assert_eq!(s.queue.len(), 0); // no callbacks
    }
    #[test]
    fn select_marks_selection_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.rows = alloc::vec![SearchRow::new(7, "Alice")];
        s.selection_dirty = false;
        select(&mut s, 7);
        assert!(s.selection_dirty);
    }

    #[test]
    fn clear_selection_marks_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.selected = alloc::vec![1];
        s.selection_dirty = false;
        clear_selection(&mut s);
        assert!(s.selection_dirty);
    }

    #[test]
    fn select_noop_does_not_mark_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.rows = alloc::vec![SearchRow::new(7, "Alice")];
        select(&mut s, 7);
        s.selection_dirty = false;
        select(&mut s, 7);
        assert!(!s.selection_dirty, "re-selecting same id is a no-op");
    }

    #[test]
    fn deselect_noop_does_not_mark_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.selection_dirty = false;
        deselect(&mut s, 999);
        assert!(!s.selection_dirty);
    }

    #[test]
    fn clear_empty_does_not_mark_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.selection_dirty = false;
        clear_selection(&mut s);
        assert!(!s.selection_dirty);
    }

    #[test]
    fn reconcile_does_not_mark_dirty() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::test_default();
        s.rows = alloc::vec![SearchRow::new(1, "a")];
        select(&mut s, 1);
        s.selected.push(99);
        s.selection_dirty = false;
        let rows_snapshot = s.rows.clone();
        reconcile(&mut s, &rows_snapshot);
        assert!(!s.selection_dirty, "reconcile must be silent");
    }

    #[test]
    fn reconcile_drops_missing_silently() {
        let _fx = SpyFixture::new();
        let mut s = st();
        s.rows = alloc::vec![SearchRow::new(1, "a"), SearchRow::new(2, "b")];
        select(&mut s, 1);
        select(&mut s, 2);
        select(&mut s, 99);
        let _ = drain_queue(&mut s); // discard select-emits
        s.rows = alloc::vec![SearchRow::new(1, "a")];
        let rows_snapshot = s.rows.clone();
        reconcile(&mut s, &rows_snapshot);
        assert_eq!(s.selected, alloc::vec![1]);
        assert_eq!(s.queue.len(), 0); // §5: no callback for silent drop
    }
}
