//! Pagination / load-more (§3.3 + §4).
use super::action::{Action, Callback};
use super::inner::InnerState;
use super::state::State;

/// Threshold (px from bottom) under which a scroll triggers load-more.
pub const LOAD_MORE_THRESHOLD_PX: i32 = 24;

pub fn should_trigger(scroll_bottom_px: i32) -> bool {
    scroll_bottom_px <= LOAD_MORE_THRESHOLD_PX
}

/// Enqueues a `LoadMore(token, page_index+1)` callback iff state=Results
/// and no load-more is already pending. Sets `snap.pending_load_more=true`
/// so the footer-loading slot becomes visible (§4 visibility table). The
/// `inner.pending_load_more: Option<u32>` tracks the page number for replay.
pub fn request_load_more(s: &mut InnerState) -> bool {
    if s.snap.state != State::Results {
        return false;
    }
    if s.snap.pending_load_more {
        return false;
    }
    let next = s.page_index + 1;
    s.pending_load_more = Some(next);
    s.snap.pending_load_more = true;
    s.queue.push(Action::EmitCallback(Callback::LoadMore {
        token: s.snap.current_token,
        page_index: next,
    }));
    true
}

/// Discards a queued/pending load-more BEFORE its callback fires.
/// Clears the visibility flag and the page tracker. No on_load_more
/// is emitted to the user. Pushes an internal CancelPendingLoadMore
/// action which the dispatcher consumes silently (§7).
pub fn cancel_pending(s: &mut InnerState) {
    if s.pending_load_more.take().is_some() {
        s.snap.pending_load_more = false;
        s.queue.push(Action::CancelPendingLoadMore);
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::Token;
    use super::*;
    use crate::c_bindings::SpyFixture;
    use alloc::vec::Vec;

    fn drain_queue(s: &mut InnerState) -> Vec<Action> {
        core::iter::from_fn(|| s.queue.pop_front()).collect()
    }

    #[test]
    fn threshold_boundary() {
        assert!(should_trigger(0));
        assert!(should_trigger(LOAD_MORE_THRESHOLD_PX));
        assert!(!should_trigger(LOAD_MORE_THRESHOLD_PX + 1));
    }

    #[test]
    fn request_emits_callback_once_per_page() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::new(true, 0, 200);
        s.snap.state = State::Results;
        s.snap.current_token = Token(3);
        assert!(request_load_more(&mut s));
        assert!(!request_load_more(&mut s)); // pending — second call no-ops
        assert!(s.snap.pending_load_more);
        assert_eq!(s.snap.state, State::Results); // state unchanged
        assert_eq!(s.queue.len(), 1);
    }

    #[test]
    fn request_rejected_outside_results_state() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::new(true, 0, 200);
        for st in [State::Empty, State::Loading, State::NoResults, State::Error] {
            s.snap.state = st;
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            assert!(!request_load_more(&mut s), "{:?}", st);
        }
    }

    #[test]
    fn cancel_clears_flag_no_user_callback() {
        let _fx = SpyFixture::new();
        let mut s = InnerState::new(true, 0, 200);
        s.snap.state = State::Results;
        request_load_more(&mut s);
        let _ = drain_queue(&mut s);
        cancel_pending(&mut s);
        assert!(!s.snap.pending_load_more);
        assert!(s.pending_load_more.is_none());
        let drained = drain_queue(&mut s);
        // The CancelPendingLoadMore action is internal; it does NOT
        // become a user callback.
        assert!(
            drained
                .iter()
                .all(|a| matches!(a, Action::CancelPendingLoadMore))
        );
    }
}
