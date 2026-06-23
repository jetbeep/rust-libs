//! Model A actions (§7). The dispatch loop in `inner.rs` enqueues these
//! while holding the InnerState borrow, then drains them after dropping
//! the borrow — guaranteeing user callbacks NEVER re-enter the borrow.
use super::state::Token;
use alloc::collections::VecDeque;
use alloc::string::String;

#[derive(Clone, Debug)]
pub enum Callback {
    QueryChanged { token: Token, query: String },
    LoadMore { token: Token, page_index: u32 },
    Select { row_id: u64, selected: bool },
    QueryCleared,
    Retry { token: Token, query: String },
}

#[derive(Clone, Debug)]
pub enum Action {
    /// Emit a user-visible callback after the borrow is released.
    EmitCallback(Callback),
    /// Cancel a load-more request that was queued but not yet fired.
    CancelPendingLoadMore,
}

/// Spec §7 (risk #39): `VecDeque` preallocated `with_capacity(QUEUE_CAP)`
/// so the hot path performs zero allocations under bounded re-entrancy.
/// When full, NEW actions are dropped and `overflow_count` increments.
/// Production never overflows because every operation enqueues at most
/// ~3 actions and the drain happens before the next operation can start.
pub const QUEUE_CAP: usize = 16;

#[derive(Debug)]
pub struct ActionQueue {
    inner: VecDeque<Action>,
    pub overflow_count: u64,
}

impl Default for ActionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionQueue {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::with_capacity(QUEUE_CAP),
            overflow_count: 0,
        }
    }
    pub fn push(&mut self, a: Action) {
        if self.inner.len() >= QUEUE_CAP {
            self.overflow_count += 1;
            debug_assert!(
                false,
                "SearchBar Action queue overflow (>{QUEUE_CAP}); risk #39"
            );
            return;
        }
        self.inner.push_back(a);
    }
    pub fn pop_front(&mut self) -> Option<Action> {
        self.inner.pop_front()
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_preallocates_capacity() {
        let q = ActionQueue::new();
        assert!(q.capacity() >= QUEUE_CAP, "VecDeque must be preallocated");
        assert!(q.is_empty());
    }
    #[test]
    fn queue_pop_front_is_fifo() {
        let mut q = ActionQueue::default();
        q.push(Action::EmitCallback(Callback::QueryCleared));
        q.push(Action::CancelPendingLoadMore);
        match q.pop_front() {
            Some(Action::EmitCallback(Callback::QueryCleared)) => {}
            other => panic!("expected QueryCleared first, got {:?}", other),
        }
        match q.pop_front() {
            Some(Action::CancelPendingLoadMore) => {}
            other => panic!("expected CancelPendingLoadMore second, got {:?}", other),
        }
        assert!(q.pop_front().is_none());
        assert!(q.is_empty());
    }
    #[test]
    #[cfg(not(debug_assertions))]
    fn queue_overflow_increments_counter_and_drops_release_only() {
        let mut q = ActionQueue::default();
        for _ in 0..QUEUE_CAP {
            q.push(Action::CancelPendingLoadMore);
        }
        q.push(Action::CancelPendingLoadMore);
        assert_eq!(q.len(), QUEUE_CAP);
        assert_eq!(q.overflow_count, 1);
    }
}
