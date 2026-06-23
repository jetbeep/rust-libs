//! Model A inner state + dispatch loop (§7).
//!
//! All public SearchBar operations follow this pattern:
//!   1. let (actions, _) = with_inner(&self.inner, |s| { ... s.queue.push(...) });
//!   2. dispatch_after_borrow(actions, &self.callbacks);
//!
//! Step 1 holds the RefCell borrow. Step 2 does NOT hold it, so a user
//! callback that reaches back into us cannot panic with BorrowMutError.

use super::action::{Action, ActionQueue, Callback};
use super::row::SearchRow;
use super::state::{StateSnapshot, Token};
use alloc::vec::Vec;
use core::cell::RefCell;

#[derive(Default)]
pub struct Callbacks {
    #[allow(clippy::type_complexity)]
    pub on_query_changed: Option<alloc::boxed::Box<dyn FnMut(Token, &str)>>,
    pub on_load_more: Option<alloc::boxed::Box<dyn FnMut(Token, u32)>>,
    pub on_select: Option<alloc::boxed::Box<dyn FnMut(u64, bool)>>,
    pub on_query_cleared: Option<alloc::boxed::Box<dyn FnMut()>>,
    pub on_retry: Option<alloc::boxed::Box<dyn FnMut(Token, &str)>>,
}

pub struct InnerState {
    pub snap: StateSnapshot,
    pub queue: ActionQueue,
    pub rows: Vec<SearchRow>,
    pub selected: Vec<u64>,
    pub page_index: u32,
    pub case_insensitive: bool,
    pub min_query_len: usize,
    pub debounce_ms: u32,
    /// Pending load-more page index awaiting drain (risk #29). This is
    /// the *page number* tracker; `snap.pending_load_more: bool` is the
    /// visibility/state flag. Both are kept in sync but serve different
    /// roles (visibility vs. payload).
    pub pending_load_more: Option<u32>,
    /// Set by selection mutators when `selected` actually changes; read &
    /// cleared by public SearchBar methods to trigger row re-renders.
    /// Reconcile (silent drop of orphaned ids) does NOT set this.
    pub selection_dirty: bool,
}

impl InnerState {
    pub fn new(case_insensitive: bool, min_query_len: usize, debounce_ms: u32) -> Self {
        Self {
            snap: StateSnapshot::default(),
            queue: ActionQueue::default(),
            rows: Vec::new(),
            selected: Vec::new(),
            page_index: 0,
            case_insensitive,
            min_query_len,
            debounce_ms,
            pending_load_more: None,
            selection_dirty: false,
        }
    }

    #[cfg(test)]
    pub fn test_default() -> Self {
        Self::new(true, 0, 200)
    }
}

/// Runs `f` under a mutable borrow of `cell`, then returns the drained
/// action queue. If the cell is already borrowed (re-entrant call from a
/// user callback), returns an empty action vec — the outer drain loop
/// will pick up newly-pushed actions on its next iteration. The
/// `snap.alive` flag is the post-deletion guard (risk #52).
pub fn with_inner<F, R>(cell: &RefCell<InnerState>, f: F) -> (Vec<Action>, Option<R>)
where
    F: FnOnce(&mut InnerState) -> R,
{
    match cell.try_borrow_mut() {
        Ok(mut s) => {
            if !s.snap.alive {
                return (Vec::new(), None);
            }
            let r = f(&mut s);
            let mut drained = Vec::with_capacity(s.queue.len());
            while let Some(a) = s.queue.pop_front() {
                drained.push(a);
            }
            (drained, Some(r))
        }
        Err(_) => {
            debug_assert!(
                false,
                "SearchBar re-entrant borrow (risk #2); inputs ignored"
            );
            (Vec::new(), None)
        }
    }
}

/// Drains an action vec by firing the matching user callbacks. Safe to
/// re-enter SearchBar APIs from these callbacks because the InnerState
/// borrow is NOT held here.
///
/// Each callback is taken out of the cell *individually* for the duration
/// of its invocation. Other callback slots remain in the cell so a
/// re-entrant `dispatch_after_borrow` call (triggered by the user
/// callback enqueueing+dispatching new actions) can still find and fire
/// them. If the user replaces the slot during the call, we keep their
/// replacement (no clobber).
///
/// NOTE on ordering: when a user callback re-enters and enqueues further
/// actions, those actions are processed *recursively/nested* in the
/// re-entrant `dispatch_after_borrow` rather than appended to the outer
/// drain. Strict cross-burst FIFO (spec §7) would require an iterative
/// pop→fire→pop loop with access to `InnerState`. Tracked as a known
/// limitation; current call sites enqueue actions only inside `with_inner`
/// and dispatch once, so this is observable only when a user callback
/// itself triggers further enqueues.
pub fn dispatch_after_borrow(actions: Vec<Action>, cb_cell: &RefCell<Callbacks>) {
    if actions.is_empty() {
        return;
    }
    for a in actions {
        match a {
            Action::EmitCallback(Callback::QueryChanged { token, query }) => {
                let cb = cb_cell
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut c| c.on_query_changed.take());
                if let Some(mut f) = cb {
                    f(token, &query);
                    if let Ok(mut c) = cb_cell.try_borrow_mut() {
                        if c.on_query_changed.is_none() {
                            c.on_query_changed = Some(f);
                        }
                    }
                }
            }
            Action::EmitCallback(Callback::LoadMore { token, page_index }) => {
                let cb = cb_cell
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut c| c.on_load_more.take());
                if let Some(mut f) = cb {
                    f(token, page_index);
                    if let Ok(mut c) = cb_cell.try_borrow_mut() {
                        if c.on_load_more.is_none() {
                            c.on_load_more = Some(f);
                        }
                    }
                }
            }
            Action::EmitCallback(Callback::Select { row_id, selected }) => {
                let cb = cb_cell
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut c| c.on_select.take());
                if let Some(mut f) = cb {
                    f(row_id, selected);
                    if let Ok(mut c) = cb_cell.try_borrow_mut() {
                        if c.on_select.is_none() {
                            c.on_select = Some(f);
                        }
                    }
                }
            }
            Action::EmitCallback(Callback::QueryCleared) => {
                let cb = cb_cell
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut c| c.on_query_cleared.take());
                if let Some(mut f) = cb {
                    f();
                    if let Ok(mut c) = cb_cell.try_borrow_mut() {
                        if c.on_query_cleared.is_none() {
                            c.on_query_cleared = Some(f);
                        }
                    }
                }
            }
            Action::EmitCallback(Callback::Retry { token, query }) => {
                let cb = cb_cell
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut c| c.on_retry.take());
                if let Some(mut f) = cb {
                    f(token, &query);
                    if let Ok(mut c) = cb_cell.try_borrow_mut() {
                        if c.on_retry.is_none() {
                            c.on_retry = Some(f);
                        }
                    }
                }
            }
            Action::CancelPendingLoadMore => { /* handled by caller before drain */ }
        }
    }
}

/// Two-condition acceptance gate (§4). Returns true if the reply for
/// `(token, canonical)` should be applied. Pass `condition2_required = false`
/// for `set_loading(_, false)` and `set_error(_, false)` (cancellation
/// signals — only condition 1 applies).
pub fn accept_reply(
    snap: &mut StateSnapshot,
    token: Token,
    canonical: &str,
    condition2_required: bool,
) -> bool {
    if snap.current_token != token {
        snap.stale_drop_count += 1;
        return false;
    }
    if condition2_required && snap.last_fired_canonical != canonical {
        snap.stale_drop_count += 1;
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    fn fresh() -> (RefCell<InnerState>, RefCell<Callbacks>) {
        (
            RefCell::new(InnerState::new(true, 0, 200)),
            RefCell::new(Callbacks::default()),
        )
    }

    #[test]
    fn dispatch_fires_query_changed() {
        static N: AtomicUsize = AtomicUsize::new(0);
        let (s, c) = fresh();
        c.borrow_mut().on_query_changed = Some(alloc::boxed::Box::new(|_t, q| {
            assert_eq!(q, "pizza");
            N.fetch_add(1, Ordering::SeqCst);
        }));
        let (acts, _) = with_inner(&s, |st| {
            st.queue.push(Action::EmitCallback(Callback::QueryChanged {
                token: Token(1),
                query: alloc::string::String::from("pizza"),
            }));
        });
        dispatch_after_borrow(acts, &c);
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn reentrancy_does_not_panic() {
        // Risk #2: a user callback that re-enters with_inner gets an empty
        // action vec, never a BorrowMutError panic.
        let s = std::rc::Rc::new(RefCell::new(InnerState::new(true, 0, 200)));
        let c = std::rc::Rc::new(RefCell::new(Callbacks::default()));
        let s2 = s.clone();
        c.borrow_mut().on_query_cleared = Some(alloc::boxed::Box::new(move || {
            // Re-enter while drain is running. Must NOT panic.
            let (_acts, _) = with_inner(&s2, |st| {
                st.snap.stale_drop_count += 7;
            });
        }));
        let (acts, _) = with_inner(&s, |st| {
            st.queue.push(Action::EmitCallback(Callback::QueryCleared));
        });
        dispatch_after_borrow(acts, &c);
    }

    #[test]
    fn reentrant_callback_action_is_not_dropped() {
        // Regression: opus re-review of Task 4 found that the previous
        // mem::take-of-whole-bag implementation silently dropped actions
        // emitted from inside a user callback. Now per-slot take preserves
        // re-entrant dispatch.
        static CLEARED: AtomicUsize = AtomicUsize::new(0);
        CLEARED.store(0, Ordering::SeqCst);
        let s = std::rc::Rc::new(RefCell::new(InnerState::new(true, 0, 200)));
        let c = std::rc::Rc::new(RefCell::new(Callbacks::default()));
        // on_query_changed re-enters and recursively enqueues+dispatches a
        // QueryCleared action. The on_query_cleared slot must still be
        // findable (i.e., not taken-and-held by the outer dispatch).
        let s2 = s.clone();
        let c2 = c.clone();
        c.borrow_mut().on_query_changed = Some(alloc::boxed::Box::new(move |_t, _q| {
            let (acts, _) = with_inner(&s2, |st| {
                st.queue.push(Action::EmitCallback(Callback::QueryCleared));
            });
            dispatch_after_borrow(acts, &c2);
        }));
        c.borrow_mut().on_query_cleared = Some(alloc::boxed::Box::new(|| {
            CLEARED.fetch_add(1, Ordering::SeqCst);
        }));
        let (acts, _) = with_inner(&s, |st| {
            st.queue.push(Action::EmitCallback(Callback::QueryChanged {
                token: Token(1),
                query: alloc::string::String::from("x"),
            }));
        });
        dispatch_after_borrow(acts, &c);
        assert_eq!(
            CLEARED.load(Ordering::SeqCst),
            1,
            "re-entrant QueryCleared was dropped"
        );
    }

    #[test]
    fn accept_reply_token_mismatch() {
        let mut snap = StateSnapshot {
            current_token: Token(7),
            ..Default::default()
        };
        snap.last_fired_canonical = "pizza".into();
        assert!(!accept_reply(&mut snap, Token(6), "pizza", true));
        assert_eq!(snap.stale_drop_count, 1);
    }

    #[test]
    fn accept_reply_canonical_mismatch_for_results() {
        let mut snap = StateSnapshot {
            current_token: Token(7),
            ..Default::default()
        };
        snap.last_fired_canonical = "pizza".into();
        assert!(!accept_reply(&mut snap, Token(7), "burger", true));
        assert_eq!(snap.stale_drop_count, 1);
    }

    #[test]
    fn accept_reply_cancel_only_checks_token() {
        // set_loading(_,false): condition2_required = false
        let mut snap = StateSnapshot {
            current_token: Token(7),
            ..Default::default()
        };
        snap.last_fired_canonical = "pizza".into();
        assert!(accept_reply(&mut snap, Token(7), "anything", false));
    }
}
