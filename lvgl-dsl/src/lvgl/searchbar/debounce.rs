//! Debounce timer (§4). The underlying LVGL timer is created with
//! `repeat_count = -1` (recurring) and gated by the SearchBar's
//! canonical-query dedupe in the fire callback: only the first fire after
//! a real query change emits `Callback::QueryChanged`. Each keystroke
//! `kick()`s the timer (reset + resume) so the dedupe window slides
//! forward `period_ms` from the latest input.
use crate::c_bindings::{
    lv_timer_create, lv_timer_delete, lv_timer_pause, lv_timer_reset, lv_timer_resume,
    lv_timer_set_period, lv_timer_set_repeat_count, lv_timer_t,
};
use core::ffi::c_void;

pub struct Debounce {
    pub handle: *mut lv_timer_t,
    pub period_ms: u32,
}

impl Debounce {
    /// # Safety
    /// `cb` runs in LVGL timer context; it must only schedule work, not
    /// re-enter SearchBar APIs synchronously (use Model A drain pattern).
    pub unsafe fn new(
        period_ms: u32,
        cb: unsafe extern "C" fn(*mut lv_timer_t),
        user_data: *mut c_void,
    ) -> Self {
        unsafe {
            let h = lv_timer_create(Some(cb), period_ms, user_data);
            lv_timer_set_repeat_count(h, -1);
            lv_timer_pause(h);
            Self {
                handle: h,
                period_ms,
            }
        }
    }

    /// Restart the debounce window. Safe to call on every keystroke.
    /// No-op if the timer has been deleted (post-`delete()`).
    pub unsafe fn kick(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            lv_timer_set_period(self.handle, self.period_ms);
            lv_timer_reset(self.handle);
            lv_timer_resume(self.handle);
        }
    }

    /// No-op if the timer has been deleted (post-`delete()`).
    pub unsafe fn pause(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            lv_timer_pause(self.handle);
        }
    }

    pub unsafe fn delete(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                lv_timer_delete(self.handle);
            }
            self.handle = core::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, SPY, SpyFixture, spy_fire_timer, spy_live_timer_handles};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static FIRES: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn cb(_t: *mut lv_timer_t) {
        FIRES.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn new_creates_paused_timer() {
        let _fx = SpyFixture::new();
        FIRES.store(0, Ordering::SeqCst);
        let _d = unsafe { Debounce::new(200, cb, core::ptr::null_mut()) };
        let pauses = SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| matches!(c, LvCall::TimerPause { .. }))
                .count()
        });
        assert!(pauses >= 1);
    }

    #[test]
    fn kick_resumes_and_resets() {
        let _fx = SpyFixture::new();
        FIRES.store(0, Ordering::SeqCst);
        let mut d = unsafe { Debounce::new(150, cb, core::ptr::null_mut()) };
        unsafe {
            d.kick();
        }
        let resumes = SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| matches!(c, LvCall::TimerResume { .. }))
                .count()
        });
        let resets = SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| matches!(c, LvCall::TimerReset { .. }))
                .count()
        });
        assert_eq!(resumes, 1);
        assert_eq!(resets, 1);
        // Fire once: callback runs.
        spy_fire_timer(d.handle);
        assert_eq!(FIRES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn delete_removes_handle_and_is_idempotent() {
        let _fx = SpyFixture::new();
        let mut d = unsafe { Debounce::new(150, cb, core::ptr::null_mut()) };
        unsafe {
            d.delete();
        }
        unsafe {
            d.delete();
        } // idempotent — null guard
        assert!(spy_live_timer_handles().is_empty());
    }
}
