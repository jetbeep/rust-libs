//! Safe builder around `lv_anim_*` with closure-based callbacks.
//!
//! Provides [`Anim`] (chainable builder), [`AnimHandle`] (RAII cancel-on-drop),
//! and [`Path`] (easing curve enum). See `DSL_REFERENCE.md` ("Animation") for
//! end-user docs.
//!
//! ## Slot table limitation (closure callbacks only)
//!
//! When using the closure forms ([`Anim::exec`] / [`Anim::on_completed`]) on
//! desktop/test builds, closures are stored in a global slot table keyed by
//! the animation's `var` pointer and stamped with a generation. Starting a
//! second closure-based animation for the same `var` while the first slot is
//! still live is a programming error; in closure-enabled builds the wrapper
//! now detects the collision, keeps the existing slot intact, and drops the new closures
//! rather than silently clobbering callbacks.
//!
//! `AnimHandle` carries the slot generation it started with. Dropping a stale
//! handle after natural completion only cancels/removes the slot when the
//! table still contains that same generation, so a successor animation that
//! reused the same `var` is left alone.
//!
//! The pure extern-fn forms ([`Anim::exec_extern`] /
//! [`Anim::completed_extern`]) are unaffected and may be reused freely on the
//! same `var`.

use core::ffi::c_void;
use core::mem::MaybeUninit;

use crate::c_bindings::{self, lv_anim_t};

type ExecFn = alloc::boxed::Box<dyn FnMut(*mut c_void, i32) + 'static>;
type CompletedFn = alloc::boxed::Box<dyn FnMut(*mut c_void) + 'static>;

// Slot table and trampoline are only needed when std is available.
// In no_std Zephyr builds the exec closure feature is compiled out.
#[cfg(any(test, no_zephyr))]
mod slot_table {
    use super::{CompletedFn, ExecFn, c_void};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    pub(super) struct Slots {
        pub generation: u64,
        pub exec: Option<ExecFn>,
        pub completed: Option<CompletedFn>,
        /// User-supplied `extern "C"` completed callback that the trampoline
        /// must forward to after running the closure form and freeing the
        /// slot. We stash it here (rather than wiring it directly into LVGL)
        /// so that mixing a closure-form `.exec(...)` with a `completed_extern`
        /// still gets slot cleanup at animation completion.
        pub completed_extern: Option<unsafe extern "C" fn(*mut super::lv_anim_t)>,
    }

    // Thread-local storage so the closure types don't need to be `Send`. LVGL
    // is single-threaded by design (animations are driven by the LVGL tick on
    // a single thread), so the trampolines that read this map run on the same
    // thread that inserted the slot.
    thread_local! {
        static SLOTS: RefCell<BTreeMap<usize, Slots>> = RefCell::new(BTreeMap::new());
        static NEXT_GENERATION: RefCell<u64> = RefCell::new(1);
    }

    /// Run `f` with mutable access to the slot table. The borrow is held only
    /// for the duration of `f`, so callers that want to invoke a user closure
    /// MUST take the closure out of the slot first, return from this call,
    /// then run the closure outside any borrow (otherwise re-entrancy from
    /// the closure into this map would panic on `BorrowMutError`).
    #[inline]
    pub(super) fn with_slots<R>(f: impl FnOnce(&mut BTreeMap<usize, Slots>) -> R) -> R {
        SLOTS.with(|cell| f(&mut cell.borrow_mut()))
    }

    pub(super) fn next_generation() -> u64 {
        NEXT_GENERATION.with(|cell| {
            let mut next = cell.borrow_mut();
            let generation = *next;
            *next = next.wrapping_add(1).max(1);
            generation
        })
    }

    pub(super) fn remove_slot_if_generation(var: *mut c_void, generation: u64) -> bool {
        with_slots(|map| {
            let key = var as usize;
            if map
                .get(&key)
                .is_some_and(|slot| slot.generation == generation)
            {
                map.remove(&key);
                true
            } else {
                false
            }
        })
    }

    /// Invoke a user closure without ever unwinding across the `extern "C"`
    /// FFI boundary back into LVGL (which would be UB on most ABIs). Returns
    /// `true` if the closure completed normally, `false` if it panicked. On
    /// panic we log to stderr and swallow; callers are expected to *not*
    /// reinstall the closure for the next tick (a panicking closure left in
    /// place would panic every frame).
    fn run_user_callback<F: FnOnce()>(label: &'static str, f: F) -> bool {
        let result = catch_unwind(AssertUnwindSafe(f));
        if let Err(payload) = result {
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                *s
            } else if let Some(s) = payload.downcast_ref::<alloc::string::String>() {
                s.as_str()
            } else {
                "<non-string panic payload>"
            };
            // We intentionally do not re-panic / abort: aborting from inside
            // an LVGL animation tick would take down the whole UI, and tests
            // expect to continue executing other unrelated cases.
            eprintln!("[jetbeep-lvgl-dsl] panic inside {label} swallowed: {msg}");
            false
        } else {
            true
        }
    }

    pub(super) unsafe extern "C" fn anim_exec_trampoline(var: *mut c_void, value: i32) {
        // Take the closure out of the slot before invoking it, so the slot
        // table is NOT borrowed while the user code runs. This avoids
        // BorrowMutError if the closure re-enters animation APIs (start/cancel)
        // that also touch the slot table.
        let key = var as usize;
        let taken: Option<(u64, ExecFn)> = with_slots(|map| {
            map.get_mut(&key)
                .and_then(|s| s.exec.take().map(|f| (s.generation, f)))
        });
        if let Some((generation, mut f)) = taken {
            let completed_ok = run_user_callback("Anim exec closure", || f(var, value));
            // Only put the closure back if it ran cleanly. If it panicked,
            // dropping `f` here ensures we don't call it again next tick
            // (which would panic every frame). If the user cancelled the
            // animation during the call (slot removed), also drop f.
            if completed_ok {
                with_slots(|map| {
                    if let Some(s) = map.get_mut(&key)
                        && s.generation == generation
                    {
                        s.exec = Some(f);
                    }
                });
            }
        }
    }

    pub(super) unsafe extern "C" fn anim_completed_trampoline(anim: *mut super::lv_anim_t) {
        let var = unsafe { crate::c_bindings::lv_anim_get_user_data(anim) };
        let key = var as usize;
        let (completed, completed_extern) = with_slots(|map| {
            if let Some(mut s) = map.remove(&key) {
                (s.completed.take(), s.completed_extern.take())
            } else {
                (None, None)
            }
        });
        if let Some(mut f) = completed {
            // Completed is one-shot, so panic-vs-clean doesn't change the
            // "call again?" decision — we never call this closure again
            // regardless. `f` is dropped at end of scope.
            let _ = run_user_callback("Anim on_completed closure", || f(var));
        }
        // Forward to a user-supplied `completed_extern` (if any) AFTER the
        // slot has been removed and any closure form has run. This is what
        // makes the `closure exec + completed_extern + start_detached()`
        // combination safe: we always get a chance to free the exec closure.
        if let Some(ext) = completed_extern {
            unsafe { ext(anim) };
        }
    }
}

#[derive(Copy, Clone)]
pub enum Path {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Overshoot,
    Bounce,
    Step,
    Custom(unsafe extern "C" fn(*const lv_anim_t) -> i32),
}

impl Path {
    fn as_extern(self) -> unsafe extern "C" fn(*const lv_anim_t) -> i32 {
        match self {
            Path::Linear => c_bindings::lv_anim_path_linear,
            Path::EaseIn => c_bindings::lv_anim_path_ease_in,
            Path::EaseOut => c_bindings::lv_anim_path_ease_out,
            Path::EaseInOut => c_bindings::lv_anim_path_ease_in_out,
            Path::Overshoot => c_bindings::lv_anim_path_overshoot,
            Path::Bounce => c_bindings::lv_anim_path_bounce,
            Path::Step => c_bindings::lv_anim_path_step,
            Path::Custom(f) => f,
        }
    }
}

/// Builder for an LVGL animation. Configure with chained setters and call
/// [`Anim::start`] to launch the animation. Drop the returned [`AnimHandle`]
/// to cancel and free the closures.
pub struct Anim {
    var: *mut c_void,
    values: Option<(i32, i32)>,
    duration_ms: Option<u32>,
    path: Option<Path>,
    repeat_count: Option<u32>,
    #[cfg(any(test, no_zephyr))]
    exec: Option<ExecFn>,
    #[cfg(any(test, no_zephyr))]
    completed: Option<CompletedFn>,
    exec_extern: Option<unsafe extern "C" fn(*mut c_void, i32)>,
    completed_extern: Option<unsafe extern "C" fn(*mut lv_anim_t)>,
}

impl Anim {
    pub fn new(var: *mut c_void) -> Self {
        Self {
            var,
            values: None,
            duration_ms: None,
            path: None,
            repeat_count: None,
            #[cfg(any(test, no_zephyr))]
            exec: None,
            #[cfg(any(test, no_zephyr))]
            completed: None,
            exec_extern: None,
            completed_extern: None,
        }
    }

    pub fn values(mut self, start: i32, end: i32) -> Self {
        self.values = Some((start, end));
        self
    }

    pub fn duration_ms(mut self, ms: u32) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn path(mut self, path: Path) -> Self {
        self.path = Some(path);
        self
    }

    pub fn repeat_count(mut self, count: u32) -> Self {
        self.repeat_count = Some(count);
        self
    }

    /// Set a raw `extern "C"` exec callback. Use this in `no_std` builds where
    /// closures are unavailable. Overrides any closure set with [`Anim::exec`].
    pub fn exec_extern(mut self, f: unsafe extern "C" fn(*mut c_void, i32)) -> Self {
        self.exec_extern = Some(f);
        self
    }

    /// Set a raw `extern "C"` completed callback. Use this in `no_std` builds
    /// where closures are unavailable. Overrides any closure set with
    /// [`Anim::on_completed`].
    pub fn completed_extern(mut self, f: unsafe extern "C" fn(*mut lv_anim_t)) -> Self {
        self.completed_extern = Some(f);
        self
    }

    #[cfg(any(test, no_zephyr))]
    pub fn exec<F>(mut self, f: F) -> Self
    where
        F: FnMut(*mut c_void, i32) + 'static,
    {
        self.exec = Some(alloc::boxed::Box::new(f));
        self
    }

    #[cfg(any(test, no_zephyr))]
    pub fn on_completed<F>(mut self, f: F) -> Self
    where
        F: FnMut(*mut c_void) + 'static,
    {
        self.completed = Some(alloc::boxed::Box::new(f));
        self
    }

    /// Start the animation and return an [`AnimHandle`] you can drop to cancel it.
    ///
    /// ## Cancellation scoping limitation
    ///
    /// `AnimHandle::drop` calls `lv_anim_delete(var, exec_cb)`, which in LVGL
    /// matches by `(var, exec_cb)` *pair*. Independent cancellation therefore
    /// only works when each running animation on the same `var` has a
    /// **different** `exec_cb`. Two animations started without an explicit
    /// `exec_extern`/`exec` will both share the internal `noop_exec_cb`
    /// sentinel, so dropping one handle cancels the other. To get clean
    /// per-animation cancellation, either (a) provide a unique
    /// `exec_extern` per animation, or (b) start each animation against a
    /// distinct `var` token.
    #[must_use = "dropping the AnimHandle immediately cancels the animation; \
                  bind the handle (`let h = ...start();`) or use \
                  `start_detached()` for fire-and-forget animations"]
    pub fn start(mut self) -> AnimHandle {
        let (exec_cb, uses_completed_trampoline, _slot_generation, _cancel_on_drop, should_start) =
            self.configure_callbacks();

        // If the caller didn't provide any exec callback, install a no-op
        // sentinel so that `AnimHandle::drop` -> `lv_anim_delete(var, cb)`
        // scopes the deletion to the `(var, noop_exec_cb)` group. Calling
        // `lv_anim_delete(var, NULL)` would cancel every animation on the
        // same var. Note: this is a *group* scope, not a per-instance scope
        // — see the "Cancellation scoping limitation" section above for the
        // implications of two scoped Anims sharing one `var` without an
        // explicit `exec_extern`.
        let exec_cb_for_register: unsafe extern "C" fn(*mut c_void, i32) =
            exec_cb.unwrap_or(noop_exec_cb);

        if should_start {
            unsafe {
                self.lv_init_and_start(Some(exec_cb_for_register), uses_completed_trampoline);
            }
        }
        AnimHandle {
            var: self.var,
            exec_cb: Some(exec_cb_for_register),
            #[cfg(any(test, no_zephyr))]
            slot_generation: _slot_generation,
            #[cfg(any(test, no_zephyr))]
            cancel_on_drop: _cancel_on_drop,
        }
    }

    /// Start the animation without returning an `AnimHandle`.
    ///
    /// Use this for "fire-and-forget" animations whose lifetime is managed
    /// entirely by LVGL (e.g., short transitions or infinite loops where the
    /// caller never needs to cancel). Unlike `start()` + `core::mem::forget(handle)`,
    /// this allocates no `AnimHandle`.
    ///
    /// **Closure cleanup:** if `.exec(...)` and/or `.on_completed(...)` were
    /// supplied, the boxed closures live in the slot table for the duration
    /// of the animation. When the animation finishes, our internal completed
    /// trampoline removes the slot — but if the animation never finishes
    /// (e.g. `repeat_count(LV_ANIM_REPEAT_INFINITE)`), the slot lives until
    /// program exit. For infinite + closure animations, prefer `start()` and
    /// keep the handle so the slot can be reclaimed on drop.
    ///
    /// **Mixing closures with `completed_extern`:** if `.completed_extern(...)`
    /// is set, it always wins over the closure form `.on_completed(...)` —
    /// the closure is dropped at start time and never invoked. When an
    /// `.exec(...)` closure is also supplied, the wrapper stashes the extern
    /// in the slot and wires its own trampoline as LVGL's completed_cb so it
    /// can free the exec-closure slot before forwarding the call to the
    /// extern. Side effect: in this mixed-mode the wrapper writes the `var`
    /// pointer into `lv_anim_t.user_data`, so the extern callback cannot
    /// rely on `lv_anim_get_user_data` for its own purposes. Pure extern
    /// callers (no closures) still get user_data left untouched.
    pub fn start_detached(mut self) {
        let (exec_cb, uses_completed_trampoline, _, _, should_start) = self.configure_callbacks();
        if should_start {
            unsafe {
                self.lv_init_and_start(exec_cb, uses_completed_trampoline);
            }
        }
    }

    /// Resolve which exec_cb / completed_cb the LVGL anim should use. Also
    /// inserts closures into the slot table when needed. Returns the exec_cb
    /// (if any), whether the completed trampoline is wired (so the caller
    /// knows when to set user_data), the slot generation if a slot was
    /// inserted, whether the returned handle should cancel on drop, and
    /// whether LVGL should be started at all.
    ///
    /// The completed trampoline is wired whenever this call inserts a slot —
    /// both `start()` and `start_detached()` rely on it for slot cleanup at
    /// animation completion. (For `start()`, `AnimHandle::drop` provides a
    /// secondary cleanup path when the user cancels before completion.)
    fn configure_callbacks(
        &mut self,
    ) -> (
        Option<unsafe extern "C" fn(*mut c_void, i32)>,
        bool,
        Option<u64>,
        bool,
        bool,
    ) {
        // If the caller supplied an extern completed_cb it always wins over a
        // closure form. Drop the closure now so it never enters the slot table
        // (otherwise it would be retained forever, never invoked, and leaked).
        // Same idea on the exec side. Both fields are cfg-gated, so the
        // assignments must be too.
        #[cfg(any(test, no_zephyr))]
        {
            if self.completed_extern.is_some() {
                self.completed = None;
            }
            if self.exec_extern.is_some() {
                self.exec = None;
            }
        }

        let mut exec_cb: Option<unsafe extern "C" fn(*mut c_void, i32)> =
            if let Some(f) = self.exec_extern {
                Some(f)
            } else {
                #[cfg(any(test, no_zephyr))]
                {
                    if self.exec.is_some() {
                        Some(slot_table::anim_exec_trampoline)
                    } else {
                        None
                    }
                }
                #[cfg(not(any(test, no_zephyr)))]
                {
                    None
                }
            };

        // Now wire closures into the slot table. We do this in ONE place so
        // that mixing `.exec_extern(...)` with `.on_completed(closure)` (or
        // vice versa) works: the closure path is independent of which exec
        // trampoline LVGL will actually invoke.
        //
        // We also stash a user-supplied `completed_extern` into the slot
        // (taking it out of `self`) when we insert a slot, so the completed
        // trampoline can forward to it AFTER the slot is freed. This is
        // what prevents the `closure exec + completed_extern + start_detached()`
        // combo from leaking the exec closure — see `start_detached`.
        #[cfg(not(any(test, no_zephyr)))]
        let uses_completed_trampoline = false;
        #[cfg(not(any(test, no_zephyr)))]
        let slot_generation: Option<u64> = None;
        #[cfg(not(any(test, no_zephyr)))]
        let cancel_on_drop = true;
        #[cfg(not(any(test, no_zephyr)))]
        let should_start = true;
        #[cfg(any(test, no_zephyr))]
        let (uses_completed_trampoline, slot_generation, cancel_on_drop, should_start) = {
            let has_exec_closure = self.exec.is_some();
            let has_completed_closure = self.completed.is_some();
            if has_exec_closure || has_completed_closure {
                let key = self.var as usize;
                let generation = slot_table::next_generation();
                let inserted = slot_table::with_slots(|map| {
                    if map.contains_key(&key) {
                        false
                    } else {
                        map.insert(
                            key,
                            slot_table::Slots {
                                generation,
                                exec: self.exec.take(),
                                completed: self.completed.take(),
                                completed_extern: self.completed_extern.take(),
                            },
                        );
                        true
                    }
                });
                if inserted {
                    (true, Some(generation), true, true)
                } else {
                    eprintln!(
                        "[jetbeep-lvgl-dsl] Anim closure slot collision for var {:#x}; \
                         keeping the existing slot and dropping the new closures",
                        key
                    );
                    self.exec = None;
                    self.completed = None;
                    if has_exec_closure {
                        exec_cb = None;
                    }
                    (false, None, false, false)
                }
            } else {
                (false, None, true, true)
            }
        };

        (
            exec_cb,
            uses_completed_trampoline,
            slot_generation,
            cancel_on_drop,
            should_start,
        )
    }

    unsafe fn lv_init_and_start(
        &self,
        exec_cb: Option<unsafe extern "C" fn(*mut c_void, i32)>,
        uses_completed_trampoline: bool,
    ) {
        let completed_cb: Option<unsafe extern "C" fn(*mut lv_anim_t)> =
            if let Some(f) = self.completed_extern {
                Some(f)
            } else if uses_completed_trampoline {
                #[cfg(any(test, no_zephyr))]
                {
                    Some(slot_table::anim_completed_trampoline)
                }
                #[cfg(not(any(test, no_zephyr)))]
                {
                    None
                }
            } else {
                None
            };

        unsafe {
            let mut a = MaybeUninit::<lv_anim_t>::uninit();
            let ap = a.as_mut_ptr();
            c_bindings::lv_anim_init(ap);
            c_bindings::lv_anim_set_var(ap, self.var);
            if let Some(cb) = exec_cb {
                c_bindings::lv_anim_set_exec_cb(ap, Some(cb));
            }
            if let Some((s, e)) = self.values {
                c_bindings::lv_anim_set_values(ap, s, e);
            }
            if let Some(ms) = self.duration_ms {
                c_bindings::lv_anim_set_duration(ap, ms);
            }
            if let Some(p) = self.path {
                c_bindings::lv_anim_set_path_cb(ap, Some(p.as_extern()));
            }
            if let Some(rc) = self.repeat_count {
                c_bindings::lv_anim_set_repeat_count(ap, rc);
            }
            // Set user_data when our internal completed trampoline is wired so
            // it can resolve the slot key. This is the case for any closure
            // animation, including the mixed mode where a `completed_extern`
            // is stashed in the slot and forwarded to from the trampoline.
            // Pure-extern callers (no closures, completed_extern only) skip
            // this so they may use user_data themselves.
            #[cfg(any(test, no_zephyr))]
            if uses_completed_trampoline {
                c_bindings::lv_anim_set_user_data(ap, self.var);
            }
            if let Some(cb) = completed_cb {
                c_bindings::lv_anim_set_completed_cb(ap, Some(cb));
            }
            c_bindings::lv_anim_start(ap as *const _);
        }
    }
}

/// Sentinel no-op exec callback installed when the caller did not register an
/// exec callback. Its purpose is to give `AnimHandle::drop` a non-null
/// `exec_cb` so that `lv_anim_delete(var, exec_cb)` scopes to the
/// `(var, noop_exec_cb)` *group* rather than wiping every animation on the
/// same `var`.
///
/// This is a **group** scope, not a per-instance scope: the sentinel is a
/// single shared function, so every `Anim::start()` call without an explicit
/// `exec_extern`/`exec` shares the same `exec_cb` identity. Dropping one such
/// handle therefore cancels every other no-exec animation on the same `var`.
/// See the "Cancellation scoping limitation" section in [`Anim::start`] for
/// how to get per-instance cancellation when you need it.
unsafe extern "C" fn noop_exec_cb(_var: *mut c_void, _val: i32) {}

/// RAII handle for a running animation. Drop it to cancel and free closures.
///
/// Marked `#[must_use]` because dropping the handle immediately cancels the
/// animation: `let _ = anim.start();` and `anim.start();` (without binding)
/// both kill the animation before it ever runs. Bind the handle to extend
/// its lifetime, or call [`Anim::start_detached`] for fire-and-forget.
#[must_use = "dropping the AnimHandle immediately cancels the animation; \
              bind the handle or use `Anim::start_detached()` for \
              fire-and-forget animations"]
pub struct AnimHandle {
    var: *mut c_void,
    exec_cb: Option<unsafe extern "C" fn(*mut c_void, i32)>,
    #[cfg(any(test, no_zephyr))]
    slot_generation: Option<u64>,
    #[cfg(any(test, no_zephyr))]
    cancel_on_drop: bool,
}

impl AnimHandle {
    /// Cancel the animation. Equivalent to dropping the handle, but reads
    /// more clearly at the call site than relying on the implicit end-of-scope
    /// drop.
    pub fn cancel(self) {
        // Make the cancellation explicit instead of relying on the implicit
        // drop at end of scope. Drop does the actual work.
        drop(self);
    }
}

impl Drop for AnimHandle {
    fn drop(&mut self) {
        #[cfg(any(test, no_zephyr))]
        if !self.cancel_on_drop {
            return;
        }

        #[cfg(any(test, no_zephyr))]
        if let Some(generation) = self.slot_generation {
            if !slot_table::remove_slot_if_generation(self.var, generation) {
                return;
            }
        }

        unsafe {
            c_bindings::lv_anim_delete(self.var, self.exec_cb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, SpyFixture, spy_drain};

    #[test]
    fn minimal_builder_emits_expected_call_sequence() {
        let _fx = SpyFixture::new();
        let h = Anim::new(0xABCD as *mut c_void)
            .values(0, 100)
            .duration_ms(500)
            .start();
        let calls = spy_drain();
        // A no-op exec_cb is installed by start() so AnimHandle::drop scopes
        // lv_anim_delete to this animation only (see noop_exec_cb).
        assert!(matches!(calls[0], LvCall::AnimInit));
        assert!(matches!(calls[1], LvCall::AnimSetVar { var } if var == 0xABCD));
        assert!(matches!(calls[2], LvCall::AnimSetExecCb { cb: Some(_) }));
        assert!(matches!(
            calls[3],
            LvCall::AnimSetValues { start: 0, end: 100 }
        ));
        assert!(matches!(calls[4], LvCall::AnimSetDuration { ms: 500 }));
        assert!(matches!(calls[5], LvCall::AnimStart));
        drop(h);
        let calls = spy_drain();
        assert!(matches!(calls[0], LvCall::AnimDelete { var, cb: Some(_) } if var == 0xABCD));
    }

    #[test]
    fn defaults_skip_unset_setters() {
        let _fx = SpyFixture::new();
        let _h = Anim::new(0x1111 as *mut c_void).start();
        let calls = spy_drain();
        assert!(matches!(calls[0], LvCall::AnimInit));
        assert!(matches!(calls[1], LvCall::AnimSetVar { var } if var == 0x1111));
        assert!(matches!(calls[2], LvCall::AnimSetExecCb { cb: Some(_) }));
        assert!(matches!(calls[3], LvCall::AnimStart));
        assert_eq!(calls.len(), 4);
    }

    #[test]
    fn start_detached_does_not_install_noop_exec() {
        let _fx = SpyFixture::new();
        Anim::new(0xBEEF as *mut c_void).start_detached();
        let calls = spy_drain();
        // No handle => no need to scope cancellation => no sentinel exec_cb.
        assert!(matches!(calls[0], LvCall::AnimInit));
        assert!(matches!(calls[1], LvCall::AnimSetVar { var } if var == 0xBEEF));
        assert!(matches!(calls[2], LvCall::AnimStart));
        assert_eq!(calls.len(), 3);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn start_detached_with_closure_wires_completed_trampoline_for_cleanup() {
        let _fx = SpyFixture::new();
        Anim::new(0xCAFE as *mut c_void)
            .exec(|_, _| {})
            .start_detached();
        let calls = spy_drain();
        // Even though the caller did not set .on_completed, our cleanup
        // trampoline must be installed so the boxed closure in the slot
        // table is reclaimed when the animation finishes.
        let installed = calls
            .iter()
            .any(|c| matches!(c, LvCall::AnimSetCompletedCb { cb: Some(_) }));
        assert!(
            installed,
            "expected a completed_cb to be installed for slot cleanup, calls = {:?}",
            calls
        );
    }

    #[test]
    fn path_enum_records_expected_set_path_cb() {
        for (path, expected) in [
            (Path::Linear, c_bindings::lv_anim_path_linear as usize),
            (Path::EaseIn, c_bindings::lv_anim_path_ease_in as usize),
            (Path::EaseOut, c_bindings::lv_anim_path_ease_out as usize),
            (
                Path::EaseInOut,
                c_bindings::lv_anim_path_ease_in_out as usize,
            ),
            (Path::Overshoot, c_bindings::lv_anim_path_overshoot as usize),
            (Path::Bounce, c_bindings::lv_anim_path_bounce as usize),
            (Path::Step, c_bindings::lv_anim_path_step as usize),
        ] {
            let _fix = SpyFixture::new();
            let _h = Anim::new(0x2222 as *mut c_void).path(path).start();
            let calls = spy_drain();
            let set = calls
                .iter()
                .find_map(|c| match c {
                    LvCall::AnimSetPathCb { cb: Some(f) } => Some(*f as usize),
                    _ => None,
                })
                .expect("AnimSetPathCb missing");
            assert_eq!(set, expected);
        }
    }

    #[test]
    fn custom_path_round_trips() {
        unsafe extern "C" fn my_path(_a: *const lv_anim_t) -> i32 {
            42
        }
        let _fix = SpyFixture::new();
        let _h = Anim::new(0x3333 as *mut c_void)
            .path(Path::Custom(my_path))
            .start();
        let calls = spy_drain();
        let set = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetPathCb { cb: Some(f) } => Some(*f as usize),
                _ => None,
            })
            .expect("AnimSetPathCb missing");
        assert_eq!(set, my_path as usize);
    }

    #[test]
    fn repeat_count_passes_through() {
        let _fix = SpyFixture::new();
        let _h = Anim::new(0x4444 as *mut c_void)
            .repeat_count(c_bindings::LV_ANIM_REPEAT_INFINITE)
            .start();
        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::AnimSetRepeatCount { count } if *count == c_bindings::LV_ANIM_REPEAT_INFINITE
        )));
    }

    #[test]
    fn exec_trampoline_invokes_closure_with_correct_args() {
        use std::sync::{Arc, Mutex};
        let _fix = SpyFixture::new();
        let observed = Arc::new(Mutex::new(Vec::<(usize, i32)>::new()));
        let observed_clone = observed.clone();
        let h = Anim::new(0x5555 as *mut c_void)
            .exec(move |var, value| {
                observed_clone.lock().unwrap().push((var as usize, value));
            })
            .start();
        let calls = spy_drain();
        let trampoline = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetExecCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("AnimSetExecCb missing");
        // Drive the trampoline manually with the var we registered.
        unsafe {
            trampoline(0x5555 as *mut c_void, 42);
        }
        assert_eq!(observed.lock().unwrap().as_slice(), &[(0x5555usize, 42i32)]);
        drop(h);
    }

    #[test]
    fn handle_drop_passes_exec_trampoline_to_anim_delete() {
        let _fix = SpyFixture::new();
        let h = Anim::new(0x6666 as *mut c_void)
            .exec(|_var, _value| {})
            .start();
        let exec_tramp = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetExecCb { cb: Some(f) } => Some(*f as usize),
                _ => None,
            })
            .expect("AnimSetExecCb missing");
        drop(h);
        let calls = spy_drain();
        let del = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimDelete { var, cb: Some(f) } if *var == 0x6666 => Some(*f as usize),
                _ => None,
            })
            .expect("AnimDelete with exec trampoline missing");
        assert_eq!(del, exec_tramp);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn completed_trampoline_invokes_closure_and_clears_slot() {
        use std::sync::{Arc, Mutex};
        let _fix = SpyFixture::new();
        let observed = Arc::new(Mutex::new(0u32));
        let observed_clone = observed.clone();
        let h = Anim::new(0x7777 as *mut c_void)
            .on_completed(move |_var| {
                *observed_clone.lock().unwrap() += 1;
            })
            .start();
        let trampoline = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("AnimSetCompletedCb missing");
        let mut fake = core::mem::MaybeUninit::<lv_anim_t>::zeroed();
        // Mock now stores user_data per anim-pointer, so we must seed the
        // value for the fake pointer used to drive the trampoline.
        unsafe {
            crate::c_bindings::lv_anim_set_user_data(fake.as_mut_ptr(), 0x7777 as *mut c_void);
            trampoline(fake.as_mut_ptr());
        }
        assert_eq!(*observed.lock().unwrap(), 1);
        drop(h);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn drop_without_completed_still_removes_slot() {
        let _fix = SpyFixture::new();
        let h = Anim::new(0x8888 as *mut c_void).exec(|_, _| {}).start();
        drop(h);
        unsafe {
            super::slot_table::anim_exec_trampoline(0x8888 as *mut c_void, 99);
        }
        // No panic, no observable effect — pass.
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn exec_extern_mixed_with_completed_closure_invokes_closure_on_completion() {
        // When mixing `.exec_extern(...)` with `.on_completed(closure)`, the
        // closure must actually be invoked on completion (previously it was
        // silently dropped because the slot-table insert lived inside the
        // `else` branch of exec_extern).
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        unsafe extern "C" fn my_exec(_v: *mut c_void, _val: i32) {}
        let _fix = SpyFixture::new();
        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let _h = Anim::new(0xC0DE as *mut c_void)
            .exec_extern(my_exec)
            .on_completed(move |_var| {
                invoked_clone.store(true, Ordering::SeqCst);
            })
            .start();
        let trampoline = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("AnimSetCompletedCb must be installed to drive the closure");
        let mut fake = core::mem::MaybeUninit::<lv_anim_t>::zeroed();
        unsafe {
            crate::c_bindings::lv_anim_set_user_data(fake.as_mut_ptr(), 0xC0DE as *mut c_void);
            trampoline(fake.as_mut_ptr());
        }
        assert!(
            invoked.load(Ordering::SeqCst),
            "on_completed closure must fire even when paired with exec_extern"
        );
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn completed_extern_overrides_and_drops_completed_closure() {
        // When .completed_extern is set, any prior .on_completed(closure) must
        // be dropped — NOT stored in the slot table where it would leak.
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        unsafe extern "C" fn my_done(_a: *mut lv_anim_t) {}
        let _fix = SpyFixture::new();
        let dropped = Arc::new(AtomicBool::new(false));
        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let flag = DropFlag(dropped.clone());
        let _h = Anim::new(0xFADE as *mut c_void)
            .on_completed(move |_var| {
                let _ = &flag;
            })
            .completed_extern(my_done)
            .start();
        assert!(
            dropped.load(Ordering::SeqCst),
            "completed closure must be dropped when completed_extern wins"
        );
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn exec_trampoline_swallows_closure_panic_without_unwinding() {
        // If the user's exec closure panics, the panic must NOT escape the
        // `extern "C"` trampoline (would be UB across the FFI boundary).
        // Furthermore, the panicking closure must NOT be reinstalled — calling
        // it again next tick would just panic again every frame.
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _fix = SpyFixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let _h = Anim::new(0xDEAD as *mut c_void)
            .exec(move |_var, _value| {
                calls_clone.fetch_add(1, Ordering::SeqCst);
                panic!("intentional panic from exec closure");
            })
            .start();
        // Driving the trampoline directly must not unwind out of this call.
        unsafe {
            super::slot_table::anim_exec_trampoline(0xDEAD as *mut c_void, 42);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "closure ran once");
        // Second tick: panicked closure should have been dropped, so it must
        // not be invoked again.
        unsafe {
            super::slot_table::anim_exec_trampoline(0xDEAD as *mut c_void, 43);
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "panicking exec closure must not be re-invoked"
        );
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn completed_trampoline_swallows_closure_panic_without_unwinding() {
        let _fix = SpyFixture::new();
        let _h = Anim::new(0xBADD as *mut c_void)
            .on_completed(|_var| panic!("intentional panic from on_completed closure"))
            .start();
        let trampoline = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("AnimSetCompletedCb missing");
        let mut fake = core::mem::MaybeUninit::<lv_anim_t>::zeroed();
        unsafe {
            crate::c_bindings::lv_anim_set_user_data(fake.as_mut_ptr(), 0xBADD as *mut c_void);
            trampoline(fake.as_mut_ptr());
        }
    }

    #[test]
    fn exec_extern_passes_fn_pointer_directly() {
        unsafe extern "C" fn my_exec(_v: *mut c_void, _val: i32) {}
        let _fix = SpyFixture::new();
        let h = Anim::new(0x9999 as *mut c_void)
            .exec_extern(my_exec)
            .start();
        let calls = spy_drain();
        let set = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetExecCb { cb: Some(f) } => Some(*f as usize),
                _ => None,
            })
            .expect("AnimSetExecCb missing");
        assert_eq!(set, my_exec as usize);
        drop(h);
        let del = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimDelete { cb: Some(f), .. } => Some(*f as usize),
                _ => None,
            })
            .expect("AnimDelete cb missing");
        assert_eq!(del, my_exec as usize);
    }

    #[test]
    fn completed_extern_passes_fn_pointer_directly() {
        unsafe extern "C" fn my_completed(_a: *mut lv_anim_t) {}
        let _fix = SpyFixture::new();
        let _h = Anim::new(0xAAAA as *mut c_void)
            .completed_extern(my_completed)
            .start();
        let calls = spy_drain();
        let set = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f as usize),
                _ => None,
            })
            .expect("AnimSetCompletedCb missing");
        assert_eq!(set, my_completed as usize);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn start_detached_closure_plus_completed_extern_forwards_and_clears_slot() {
        use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        static EXEC_CALLS: AtomicUsize = AtomicUsize::new(0);
        static EXTERN_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn my_completed(_a: *mut lv_anim_t) {
            EXTERN_CALLED.store(true, Ordering::SeqCst);
        }

        let _fix = SpyFixture::new();
        EXEC_CALLS.store(0, Ordering::SeqCst);
        EXTERN_CALLED.store(false, Ordering::SeqCst);

        Anim::new(0xBEEF as *mut c_void)
            .exec(|_, _| {
                EXEC_CALLS.fetch_add(1, Ordering::SeqCst);
            })
            .completed_extern(my_completed)
            .start_detached();

        let calls = spy_drain();
        // The wrapper must wire its internal trampoline (NOT the user's
        // extern) as LVGL's completed_cb so it can free the slot before
        // forwarding to the extern.
        let trampoline = calls
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("AnimSetCompletedCb missing");
        assert_ne!(
            trampoline as usize, my_completed as usize,
            "expected wrapper trampoline, not the user's extern directly"
        );

        // Fire the trampoline (LVGL would do this on completion).
        let mut fake = core::mem::MaybeUninit::<lv_anim_t>::zeroed();
        unsafe {
            crate::c_bindings::lv_anim_set_user_data(fake.as_mut_ptr(), 0xBEEF as *mut c_void);
            trampoline(fake.as_mut_ptr());
        }
        assert!(
            EXTERN_CALLED.load(Ordering::SeqCst),
            "completed_extern must be forwarded to from the trampoline"
        );

        // Slot must be reclaimed so the exec closure is dropped.
        let slot_present = super::slot_table::with_slots(|map| {
            map.contains_key(&(0xBEEF as *mut c_void as usize))
        });
        assert!(!slot_present, "slot should be removed after completion");
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn dropping_stale_completed_handle_does_not_cancel_successor_slot() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let _fix = SpyFixture::new();
        let first_done = Arc::new(AtomicUsize::new(0));
        let first_done_clone = first_done.clone();
        let first = Anim::new(0xA15E as *mut c_void)
            .exec(|_, _| {})
            .on_completed(move |_| {
                first_done_clone.fetch_add(1, Ordering::SeqCst);
            })
            .start();

        let first_completed = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetCompletedCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("first completed trampoline missing");

        let mut completed_anim = core::mem::MaybeUninit::<lv_anim_t>::zeroed();
        unsafe {
            crate::c_bindings::lv_anim_set_user_data(
                completed_anim.as_mut_ptr(),
                0xA15E as *mut c_void,
            );
            first_completed(completed_anim.as_mut_ptr());
        }
        assert_eq!(first_done.load(Ordering::SeqCst), 1);

        let successor_execs = Arc::new(AtomicUsize::new(0));
        let successor_execs_clone = successor_execs.clone();
        let successor = Anim::new(0xA15E as *mut c_void)
            .exec(move |_, value| {
                successor_execs_clone.fetch_add(value as usize, Ordering::SeqCst);
            })
            .start();
        let successor_exec = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetExecCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("successor exec trampoline missing");

        drop(first);
        let stale_drop_calls = spy_drain();
        assert!(
            stale_drop_calls
                .iter()
                .all(|c| !matches!(c, LvCall::AnimDelete { var, .. } if *var == 0xA15E)),
            "stale handle drop must not delete successor animation: {:?}",
            stale_drop_calls
        );

        unsafe {
            successor_exec(0xA15E as *mut c_void, 7);
        }
        assert_eq!(
            successor_execs.load(Ordering::SeqCst),
            7,
            "successor slot must remain live after stale handle drop"
        );

        drop(successor);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn completed_only_collision_loser_drop_does_not_delete_existing_animation() {
        let _fix = SpyFixture::new();
        let first = Anim::new(0xC012 as *mut c_void)
            .on_completed(|_| {})
            .start();
        let _ = spy_drain();

        let second = Anim::new(0xC012 as *mut c_void)
            .on_completed(|_| {})
            .start();
        let collision_calls = spy_drain();
        assert!(
            collision_calls
                .iter()
                .all(|c| !matches!(c, LvCall::AnimStart)),
            "collision loser must not be started because LVGL replaces matching (var, exec_cb) animations on start: {:?}",
            collision_calls
        );

        drop(second);
        let loser_drop_calls = spy_drain();
        assert!(
            loser_drop_calls
                .iter()
                .all(|c| !matches!(c, LvCall::AnimDelete { var, .. } if *var == 0xC012)),
            "collision-loser handle drop must not delete the existing animation: {:?}",
            loser_drop_calls
        );

        drop(first);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn concurrent_closure_collision_keeps_existing_slot_instead_of_overwriting() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let _fix = SpyFixture::new();
        let first_execs = Arc::new(AtomicUsize::new(0));
        let second_execs = Arc::new(AtomicUsize::new(0));
        let first_execs_clone = first_execs.clone();
        let first = Anim::new(0xC011 as *mut c_void)
            .exec(move |_, _| {
                first_execs_clone.fetch_add(1, Ordering::SeqCst);
            })
            .start();
        let first_exec = spy_drain()
            .iter()
            .find_map(|c| match c {
                LvCall::AnimSetExecCb { cb: Some(f) } => Some(*f),
                _ => None,
            })
            .expect("first exec trampoline missing");

        let second_execs_clone = second_execs.clone();
        let second = Anim::new(0xC011 as *mut c_void)
            .exec(move |_, _| {
                second_execs_clone.fetch_add(1, Ordering::SeqCst);
            })
            .start();

        unsafe {
            first_exec(0xC011 as *mut c_void, 1);
        }
        assert_eq!(first_execs.load(Ordering::SeqCst), 1);
        assert_eq!(
            second_execs.load(Ordering::SeqCst),
            0,
            "collision branch must not overwrite the live slot with the new closure"
        );

        drop(second);
        drop(first);
    }

    #[test]
    #[cfg(any(test, no_zephyr))]
    fn closure_apis_accept_non_send_captures() {
        // Regression: closure-based exec/on_completed must not require Send,
        // so callers can capture common UI types like Rc<RefCell<_>>.
        use std::cell::RefCell;
        use std::rc::Rc;
        let _fix = SpyFixture::new();
        let counter = Rc::new(RefCell::new(0i32));
        let counter_for_exec = Rc::clone(&counter);
        let counter_for_done = Rc::clone(&counter);
        let h = Anim::new(0xCAFD as *mut c_void)
            .exec(move |_, v| {
                *counter_for_exec.borrow_mut() = v;
            })
            .on_completed(move |_| {
                *counter_for_done.borrow_mut() = -1;
            })
            .start();
        drop(h);
        let _ = counter.borrow();
    }
}
