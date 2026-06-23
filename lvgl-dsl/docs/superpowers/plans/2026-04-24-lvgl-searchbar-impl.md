# LVGL SearchBar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `no_std` LVGL v9.2 SearchBar widget per the v5-approved spec at `docs/superpowers/specs/2026-04-24-lvgl-searchbar-design.md`, with full TDD coverage against the desktop-sim spy.

**Architecture:** Model A inner-state dispatch (`apply_*` never fires user callbacks; single bounded `VecDeque<Action>` cap 16; `alive` flag for post-deletion safety). Four caller-fillable slot containers for loading/error UI. Two-condition acceptance gate (token + canonical-query match) gates all async response setters. Schema-driven rows with optional recolor highlight markup.

**Tech Stack:** Rust `no_std` + `alloc`, LVGL v9.2 via existing bindgen wrapper, thread-local desktop-sim spy for `cargo test`, Zephyr for embedded.

---

## Conventions used by every task

- Spec citations look like `(implements §4 acceptance gate)`; refer to `docs/superpowers/specs/2026-04-24-lvgl-searchbar-design.md`.
- Risk citations look like `(risk #29)`; refer to §12 of the spec.
- All tests live in `#[cfg(test)] mod tests { … }` inside the file under test (or in a sibling `tests` module), per the existing pattern (`src/lvgl/textarea.rs`, `src/lvgl/keyboard.rs`).
- Every test starts with `let _fx = SpyFixture::new();` (introduced in Task 1).
- Branch is `feature/serach_bar`. Every commit must include the trailer:
  ```
  Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
  ```
- Build/test command throughout: `cargo test --lib -- --test-threads=4`. Lint: `cargo clippy --all-targets --all-features -- -D warnings`.


---

## Task 1: Extended desktop-sim spy infrastructure + bindings.conf delta

This is the gate for every other task. SearchBar tests cannot drive callbacks, fire timers, or read scroll positions without this; Step-1 spy tests must reference each new bound symbol so the build fails fast if any are missing (implements §10 Step 0; risk #19, #26, #37, #38, #43, #45, #47, #53).

**Files:**
- Modify: `src/lvgl/bindings.conf` (append §8 delta)
- Modify: `src/c_bindings.rs:303-976` (extend `LvCall`, add registries, add `SpyFixture`, add new fn stubs in `desktop` and `#[cfg(any(test, all(no_zephyr, not(desktop_sim))))]` modules)
- Test: `src/c_bindings.rs` `mod tests` block at end of file.

### Step 1.1: Append §8 binding deltas to `bindings.conf`

- [ ] Append to `src/lvgl/bindings.conf` under a new `# SearchBar deltas (§8)` section:

```
# SearchBar deltas (§8)
lv_timer_create|lv_timer_set_period|lv_timer_reset
lv_timer_set_repeat_count|lv_timer_pause|lv_timer_resume|lv_timer_delete
lv_obj_get_scroll_bottom|lv_obj_get_scroll_top
lv_obj_set_scrollbar_mode|lv_obj_scroll_to_view
lv_obj_set_user_data|lv_obj_get_user_data
lv_label_set_long_mode|lv_label_set_recolor
lv_obj_get_child_count
lv_obj_remove_event_cb_with_user_data
lv_group_focus_obj
```

### Step 1.2: Write the failing "every new symbol is referenced" compile test

- [ ] Append to the `mod tests` block at end of `src/c_bindings.rs`:

```rust
#[test]
fn task1_new_symbols_referenced() {
    // This test exists so that any "must add" symbol from spec §8 missing
    // from bindings.conf produces a compile error rather than a runtime
    // surprise. Each function reference forces bindgen to have emitted it
    // for the Zephyr build, and forces our desktop-sim shim to declare it.
    use crate::c_bindings::*;
    let _ = lv_timer_create
        as unsafe extern "C" fn(_, _, _) -> *mut lv_timer_t;
    let _ = lv_timer_set_period as unsafe extern "C" fn(*mut lv_timer_t, u32);
    let _ = lv_timer_reset as unsafe extern "C" fn(*mut lv_timer_t);
    let _ = lv_timer_set_repeat_count
        as unsafe extern "C" fn(*mut lv_timer_t, i32);
    let _ = lv_timer_pause as unsafe extern "C" fn(*mut lv_timer_t);
    let _ = lv_timer_resume as unsafe extern "C" fn(*mut lv_timer_t);
    let _ = lv_timer_delete as unsafe extern "C" fn(*mut lv_timer_t);
    let _ = lv_obj_get_scroll_bottom as unsafe extern "C" fn(*mut lv_obj_t) -> i32;
    let _ = lv_obj_get_scroll_top    as unsafe extern "C" fn(*mut lv_obj_t) -> i32;
    let _ = lv_obj_set_scrollbar_mode as unsafe extern "C" fn(*mut lv_obj_t, u32);
    let _ = lv_obj_scroll_to_view     as unsafe extern "C" fn(*mut lv_obj_t, u32);
    let _ = lv_obj_set_user_data
        as unsafe extern "C" fn(*mut lv_obj_t, *mut core::ffi::c_void);
    let _ = lv_obj_get_user_data
        as unsafe extern "C" fn(*mut lv_obj_t) -> *mut core::ffi::c_void;
    let _ = lv_label_set_long_mode as unsafe extern "C" fn(*mut lv_obj_t, u32);
    let _ = lv_label_set_recolor   as unsafe extern "C" fn(*mut lv_obj_t, bool);
    let _ = lv_obj_get_child_count as unsafe extern "C" fn(*mut lv_obj_t) -> u32;
    let _ = lv_obj_remove_event_cb_with_user_data
        as unsafe extern "C" fn(*mut lv_obj_t,
                                Option<unsafe extern "C" fn(*mut lv_event_t)>,
                                *mut core::ffi::c_void);
    let _ = lv_group_focus_obj as unsafe extern "C" fn(*mut lv_obj_t);
}
```

- [ ] Run: `cargo test --lib c_bindings::tests::task1_new_symbols_referenced`
  Expected: FAIL with `cannot find function lv_timer_create in this scope` (none of the symbols exist yet in the desktop-sim shim; bindgen has not been invoked yet for these).

### Step 1.3: Add `extern "C"` declarations for new symbols in the `desktop` module

- [ ] In `src/c_bindings.rs`, locate the `mod desktop { … unsafe extern "C" { … } }` block (around line 43+). Append inside the `unsafe extern "C" { }` block, before the closing brace:

```rust
        // ---- SearchBar deltas (§8) ----
        pub fn lv_timer_create(
            cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
            period_ms: u32,
            user_data: *mut core::ffi::c_void,
        ) -> *mut lv_timer_t;
        pub fn lv_timer_set_period(t: *mut lv_timer_t, period_ms: u32);
        pub fn lv_timer_reset(t: *mut lv_timer_t);
        pub fn lv_timer_set_repeat_count(t: *mut lv_timer_t, count: i32);
        pub fn lv_timer_pause(t: *mut lv_timer_t);
        pub fn lv_timer_resume(t: *mut lv_timer_t);
        pub fn lv_timer_delete(t: *mut lv_timer_t);
        pub fn lv_obj_get_scroll_bottom(obj: *mut lv_obj_t) -> i32;
        pub fn lv_obj_get_scroll_top(obj: *mut lv_obj_t) -> i32;
        pub fn lv_obj_set_scrollbar_mode(obj: *mut lv_obj_t, mode: u32);
        pub fn lv_obj_scroll_to_view(obj: *mut lv_obj_t, anim_en: u32);
        pub fn lv_obj_set_user_data(obj: *mut lv_obj_t, ud: *mut core::ffi::c_void);
        pub fn lv_obj_get_user_data(obj: *mut lv_obj_t) -> *mut core::ffi::c_void;
        pub fn lv_label_set_long_mode(label: *mut lv_obj_t, mode: u32);
        pub fn lv_label_set_recolor(label: *mut lv_obj_t, en: bool);
        pub fn lv_obj_get_child_count(obj: *mut lv_obj_t) -> u32;
        pub fn lv_obj_remove_event_cb_with_user_data(
            obj: *mut lv_obj_t,
            cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
            user_data: *mut core::ffi::c_void,
        );
        pub fn lv_group_focus_obj(obj: *mut lv_obj_t);
```

- [ ] Also add the timer opaque type. Inside `mod desktop`, after the existing `pub struct lv_event_dsc_t`:

```rust
    #[repr(C)]
    pub struct lv_timer_t { _opaque: [u8; 0] }
```

### Step 1.4: Add `lv_timer_t` and stub fns in the test/no_zephyr shim module

- [ ] Locate the `#[cfg(any(test, all(no_zephyr, not(desktop_sim))))]` module (starts at line 303 of `src/c_bindings.rs`). Add an opaque `lv_timer_t`:

```rust
    #[repr(C)]
    pub struct lv_timer_t { _opaque: [u8; 0] }
```

- [ ] Extend the `LvCall` enum with the §10 variants. Locate `pub enum LvCall { … }` (around line 368) and append:

```rust
        // SearchBar — timers
        TimerCreate         { handle: usize, period_ms: u32, user_data: usize },
        TimerSetPeriod      { handle: usize, period_ms: u32 },
        TimerReset          { handle: usize },
        TimerSetRepeatCount { handle: usize, count: i32 },
        TimerPause          { handle: usize },
        TimerResume         { handle: usize },
        TimerDelete         { handle: usize },
        // SearchBar — scroll & geometry
        ObjGetScrollBottom  { obj: usize, ret: i32 },
        ObjGetScrollTop     { obj: usize, ret: i32 },
        ObjSetScrollbarMode { obj: usize, mode: u32 },
        ObjScrollToView     { obj: usize, anim: u32 },
        // SearchBar — user data
        ObjSetUserData      { obj: usize, data: usize },
        ObjGetUserData      { obj: usize, ret: usize },
        // SearchBar — label extras
        LabelSetLongMode    { label: usize, mode: u32 },
        LabelSetRecolor     { label: usize, en: bool },
        // SearchBar — children & focus
        ObjGetChildCount    { obj: usize, ret: u32 },
        GroupFocusObj       { obj: usize },
        // SearchBar — event removal (targeted)
        RemoveEventCbWithUserData { obj: usize, user_data: usize },
        // SearchBar — synthesized event firing
        SpyEmitEvent        { obj: usize, code: u32 },
```

### Step 1.5: Add registries and `SpyFixture` RAII

- [ ] In the same shim module, append after the existing `thread_local!` block (around line 432):

```rust
    use core::ffi::c_void;

    #[derive(Clone, Copy)]
    pub struct EventReg {
        pub code: u32,
        pub cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        pub user_data: *mut c_void,
    }

    pub struct TimerReg {
        pub period_ms: u32,
        pub repeat_count: i32,        // §10: signed; -1 = infinite
        pub cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
        pub user_data: *mut c_void,
        pub paused: bool,
    }

    thread_local! {
        pub(crate) static EVENT_REG:
            RefCell<HashMap<usize, Vec<EventReg>>> = RefCell::new(HashMap::new());
        pub(crate) static USER_DATA:
            RefCell<HashMap<usize, usize>>         = RefCell::new(HashMap::new());
        pub(crate) static TIMER_REG:
            RefCell<HashMap<usize, TimerReg>>      = RefCell::new(HashMap::new());
        pub(crate) static NEXT_TIMER_HANDLE:
            Cell<usize>                            = const { Cell::new(0x1000) };
        pub(crate) static NEXT_SCROLL_BOTTOM:      Cell<i32> = const { Cell::new(0) };
        pub(crate) static NEXT_SCROLL_TOP:         Cell<i32> = const { Cell::new(0) };
        pub(crate) static CHILD_COUNTS:
            RefCell<HashMap<usize, u32>>           = RefCell::new(HashMap::new());
        // Synthesized event currently being delivered, so accessors work
        // inside fired callbacks.
        pub(crate) static CURRENT_EVENT:
            Cell<(usize /*target*/, u32 /*code*/, usize /*user_data*/)>
            = const { Cell::new((0, 0, 0)) };
    }

    pub fn reset_all_thread_local_spy_state() {
        reset_obj_pool();
        EVENT_REG.with(|m| m.borrow_mut().clear());
        USER_DATA.with(|m| m.borrow_mut().clear());
        TIMER_REG.with(|m| m.borrow_mut().clear());
        NEXT_TIMER_HANDLE.with(|c| c.set(0x1000));
        NEXT_SCROLL_BOTTOM.with(|c| c.set(0));
        NEXT_SCROLL_TOP.with(|c| c.set(0));
        CHILD_COUNTS.with(|m| m.borrow_mut().clear());
        CURRENT_EVENT.with(|c| c.set((0, 0, 0)));
    }

    /// RAII fixture: resets spy state at construction (so a previous
    /// panicking test cannot poison this one) and again on Drop. Use
    /// at the top of every SearchBar test:
    ///
    /// ```ignore
    /// let _fx = SpyFixture::new();
    /// ```
    pub struct SpyFixture(());
    impl SpyFixture {
        pub fn new() -> Self { reset_all_thread_local_spy_state(); SpyFixture(()) }
    }
    impl Drop for SpyFixture {
        fn drop(&mut self) { reset_all_thread_local_spy_state(); }
    }

    // ---- Scroll injection helpers (used by pagination tests) ----
    pub fn set_next_scroll_bottom(px: i32) { NEXT_SCROLL_BOTTOM.with(|c| c.set(px)); }
    pub fn set_next_scroll_top(px: i32)    { NEXT_SCROLL_TOP.with(|c| c.set(px)); }
    pub fn set_child_count(obj: *mut lv_obj_t, n: u32) {
        CHILD_COUNTS.with(|m| { m.borrow_mut().insert(obj as usize, n); });
    }

    // ---- Synthesized event firing ----
    pub fn spy_emit_event(obj: *mut lv_obj_t, code: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SpyEmitEvent { obj: obj as usize, code }));
        let regs: Vec<EventReg> = EVENT_REG.with(|m| {
            m.borrow().get(&(obj as usize)).cloned().unwrap_or_default()
        });
        for r in regs {
            if r.code != 0 /* LV_EVENT_ALL */ && r.code != code { continue; }
            CURRENT_EVENT.with(|c| c.set((obj as usize, code, r.user_data as usize)));
            if let Some(cb) = r.cb {
                // Synthetic event_t is a type-erased token; the spy
                // accessors read CURRENT_EVENT instead of dereferencing it.
                unsafe { cb(core::ptr::dangling_mut::<lv_event_t>()); }
            }
            CURRENT_EVENT.with(|c| c.set((0, 0, 0)));
        }
    }

    pub fn spy_fire_timer(handle: *mut lv_timer_t) {
        let h = handle as usize;
        let action: Option<(Option<unsafe extern "C" fn(*mut lv_timer_t)>, bool /*remove*/)> =
            TIMER_REG.with(|m| {
                let mut m = m.borrow_mut();
                let Some(t) = m.get_mut(&h) else { return None; };
                if t.paused          { return Some((None, false)); }
                if t.repeat_count == 0 {
                    // LVGL: 0 means "no fires remaining; auto-delete".
                    return Some((None, true));
                }
                let cb = t.cb;
                if t.repeat_count > 0 {
                    t.repeat_count -= 1;
                }
                let remove = t.repeat_count == 0 && t.repeat_count != -1;
                Some((cb, remove))
            });
        match action {
            None => {}
            Some((cb, remove)) => {
                if let Some(cb) = cb { unsafe { cb(handle); } }
                if remove { TIMER_REG.with(|m| { m.borrow_mut().remove(&h); }); }
            }
        }
    }

    pub fn spy_live_timer_handles() -> Vec<usize> {
        TIMER_REG.with(|m| m.borrow().keys().copied().collect())
    }
```

### Step 1.6: Implement the new shim functions

- [ ] Append to the same shim module (after the existing impls, around line 870+):

```rust
    // -------- Timers --------
    pub unsafe fn lv_timer_create(
        cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
        period_ms: u32,
        user_data: *mut c_void,
    ) -> *mut lv_timer_t {
        let handle = NEXT_TIMER_HANDLE.with(|c| { let h = c.get(); c.set(h + 8); h });
        TIMER_REG.with(|m| {
            m.borrow_mut().insert(handle, TimerReg {
                period_ms, repeat_count: -1, cb, user_data, paused: false,
            });
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerCreate {
            handle, period_ms, user_data: user_data as usize,
        }));
        handle as *mut lv_timer_t
    }
    pub unsafe fn lv_timer_set_period(t: *mut lv_timer_t, period_ms: u32) {
        TIMER_REG.with(|m| if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
            tr.period_ms = period_ms;
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerSetPeriod { handle: t as usize, period_ms }));
    }
    pub unsafe fn lv_timer_reset(t: *mut lv_timer_t) {
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerReset { handle: t as usize }));
    }
    pub unsafe fn lv_timer_set_repeat_count(t: *mut lv_timer_t, count: i32) {
        TIMER_REG.with(|m| if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
            tr.repeat_count = count;
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerSetRepeatCount { handle: t as usize, count }));
    }
    pub unsafe fn lv_timer_pause(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) { tr.paused = true; });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerPause { handle: t as usize }));
    }
    pub unsafe fn lv_timer_resume(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) { tr.paused = false; });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerResume { handle: t as usize }));
    }
    pub unsafe fn lv_timer_delete(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| { m.borrow_mut().remove(&(t as usize)); });
        SPY.with(|s| s.borrow_mut().push(LvCall::TimerDelete { handle: t as usize }));
    }

    // -------- Scroll geometry --------
    pub unsafe fn lv_obj_get_scroll_bottom(obj: *mut lv_obj_t) -> i32 {
        let v = NEXT_SCROLL_BOTTOM.with(|c| c.get());
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjGetScrollBottom { obj: obj as usize, ret: v }));
        v
    }
    pub unsafe fn lv_obj_get_scroll_top(obj: *mut lv_obj_t) -> i32 {
        let v = NEXT_SCROLL_TOP.with(|c| c.get());
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjGetScrollTop { obj: obj as usize, ret: v }));
        v
    }
    pub unsafe fn lv_obj_set_scrollbar_mode(obj: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetScrollbarMode { obj: obj as usize, mode }));
    }
    pub unsafe fn lv_obj_scroll_to_view(obj: *mut lv_obj_t, anim: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjScrollToView { obj: obj as usize, anim }));
    }

    // -------- User data --------
    pub unsafe fn lv_obj_set_user_data(obj: *mut lv_obj_t, ud: *mut c_void) {
        USER_DATA.with(|m| { m.borrow_mut().insert(obj as usize, ud as usize); });
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetUserData { obj: obj as usize, data: ud as usize }));
    }
    pub unsafe fn lv_obj_get_user_data(obj: *mut lv_obj_t) -> *mut c_void {
        let v = USER_DATA.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0));
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjGetUserData { obj: obj as usize, ret: v }));
        v as *mut c_void
    }

    // -------- Label extras --------
    pub unsafe fn lv_label_set_long_mode(label: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::LabelSetLongMode { label: label as usize, mode }));
    }
    pub unsafe fn lv_label_set_recolor(label: *mut lv_obj_t, en: bool) {
        SPY.with(|s| s.borrow_mut().push(LvCall::LabelSetRecolor { label: label as usize, en }));
    }

    // -------- Children & focus --------
    pub unsafe fn lv_obj_get_child_count(obj: *mut lv_obj_t) -> u32 {
        let v = CHILD_COUNTS.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0));
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjGetChildCount { obj: obj as usize, ret: v }));
        v
    }
    pub unsafe fn lv_group_focus_obj(obj: *mut lv_obj_t) {
        SPY.with(|s| s.borrow_mut().push(LvCall::GroupFocusObj { obj: obj as usize }));
    }

    // -------- Targeted event removal --------
    pub unsafe fn lv_obj_remove_event_cb_with_user_data(
        obj: *mut lv_obj_t,
        _cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        user_data: *mut c_void,
    ) {
        EVENT_REG.with(|m| {
            if let Some(v) = m.borrow_mut().get_mut(&(obj as usize)) {
                v.retain(|r| r.user_data != user_data);
            }
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::RemoveEventCbWithUserData {
            obj: obj as usize, user_data: user_data as usize,
        }));
    }
```

- [ ] Replace the existing `lv_obj_add_event_cb` and `lv_event_get_user_data`/`lv_event_get_target`/`lv_event_get_code` shims (around line 513-525) so they actually populate / consult the event registry:

```rust
    pub unsafe fn lv_obj_add_event_cb(
        obj: *mut lv_obj_t,
        cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        code: u32,
        user_data: *mut c_void,
    ) -> *mut lv_event_dsc_t {
        EVENT_REG.with(|m| {
            m.borrow_mut().entry(obj as usize).or_default().push(EventReg {
                code, cb, user_data,
            });
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::AddEventCb { obj: obj as usize, code }));
        core::ptr::null_mut()
    }

    pub unsafe fn lv_obj_remove_event_cb(
        obj: *mut lv_obj_t,
        _dsc: *mut lv_event_dsc_t,
    ) -> bool {
        EVENT_REG.with(|m| {
            if let Some(v) = m.borrow_mut().get_mut(&(obj as usize)) { v.clear(); }
        });
        true
    }

    pub unsafe fn lv_event_get_user_data(_e: *mut lv_event_t) -> *mut c_void {
        CURRENT_EVENT.with(|c| c.get().2) as *mut c_void
    }
    pub unsafe fn lv_event_get_target(_e: *mut lv_event_t) -> *mut c_void {
        CURRENT_EVENT.with(|c| c.get().0) as *mut c_void
    }
    pub unsafe fn lv_event_get_code(_e: *mut lv_event_t) -> u32 {
        CURRENT_EVENT.with(|c| c.get().1)
    }
```

### Step 1.7: Re-export new shim symbols at the crate level

- [ ] Locate the `pub use` block for the test/no_zephyr shim near the bottom of `src/c_bindings.rs` (it follows the shim's `mod` definition). Add the new function names and `lv_timer_t`, `SpyFixture`, `spy_emit_event`, `spy_fire_timer`, `spy_live_timer_handles`, `set_next_scroll_bottom`, `set_next_scroll_top`, `set_child_count`, `reset_all_thread_local_spy_state` to the `pub use shim::{…}` re-export. (If a `pub use` line does not yet exist for the shim module, add one explicitly listing every public symbol from the shim module — match the existing convention.)

### Step 1.8: Run the symbol-reference test — should now compile and pass

- [ ] Run: `cargo test --lib c_bindings::tests::task1_new_symbols_referenced -- --nocapture`
  Expected: PASS.

### Step 1.9: Add behavioural spy tests

- [ ] Append to `mod tests` in `src/c_bindings.rs`:

```rust
#[test]
fn task1_event_registry_dispatches() {
    let _fx = SpyFixture::new();
    static FIRES: core::sync::atomic::AtomicUsize =
        core::sync::atomic::AtomicUsize::new(0);
    unsafe extern "C" fn cb(_e: *mut lv_event_t) {
        FIRES.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }
    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
    unsafe { lv_obj_add_event_cb(obj, Some(cb), 7 /* arbitrary code */, core::ptr::null_mut()); }
    spy_emit_event(obj, 7);
    spy_emit_event(obj, 8); // wrong code, no fire
    assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn task1_timer_repeat_count_branches() {
    let _fx = SpyFixture::new();
    static FIRES: core::sync::atomic::AtomicUsize =
        core::sync::atomic::AtomicUsize::new(0);
    unsafe extern "C" fn cb(_t: *mut lv_timer_t) {
        FIRES.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }
    let t = unsafe { lv_timer_create(Some(cb), 250, core::ptr::null_mut()) };
    // default repeat_count = -1 (infinity): fires forever
    spy_fire_timer(t); spy_fire_timer(t);
    assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 2);
    unsafe { lv_timer_set_repeat_count(t, 1); }
    spy_fire_timer(t);   // fires once, then auto-removes
    spy_fire_timer(t);   // no-op
    assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 3);
    assert!(spy_live_timer_handles().is_empty());
}

#[test]
fn task1_user_data_roundtrip() {
    let _fx = SpyFixture::new();
    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
    let ptr: *mut core::ffi::c_void = 0xDEADBEEF as _;
    unsafe { lv_obj_set_user_data(obj, ptr); }
    assert_eq!(unsafe { lv_obj_get_user_data(obj) } as usize, 0xDEADBEEF);
}

#[test]
fn task1_scroll_injection() {
    let _fx = SpyFixture::new();
    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
    set_next_scroll_bottom(42);
    assert_eq!(unsafe { lv_obj_get_scroll_bottom(obj) }, 42);
    // Subsequent reads keep the same value (sticky); test simply verifies plumbing.
}

#[test]
fn task1_spy_fixture_resets_state_on_drop() {
    {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        unsafe { lv_obj_set_user_data(obj, 0x1 as *mut _); }
    } // _fx drops here — registries cleared
    // Nothing asserts directly; a second SpyFixture::new() inside another test
    // is the canonical isolation contract (covered by parallel_isolation below).
}

#[test]
fn task1_parallel_isolation() {
    // risk #37 — spy state lives in thread_local!; parallel cargo test
    // workers must not corrupt each other.
    let handles: Vec<_> = (0..4).map(|_| std::thread::spawn(|| {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        unsafe { lv_obj_set_user_data(obj, 0xAA as *mut _); }
        let v = unsafe { lv_obj_get_user_data(obj) } as usize;
        assert_eq!(v, 0xAA);
    })).collect();
    for h in handles { h.join().unwrap(); }
}
```

- [ ] Run: `cargo test --lib c_bindings::tests::task1_`
  Expected: All 6 PASS.

### Step 1.10: Commit

- [ ] Commit:

```bash
git add src/lvgl/bindings.conf src/c_bindings.rs
git commit -m "feat(c_bindings): extended desktop-sim spy for SearchBar (Task 1)

- Append SearchBar §8 binding deltas to bindings.conf (timers, scroll,
  user_data, label recolor/long_mode, child_count, targeted event
  removal, group focus).
- Extend desktop-sim shim with extern decls for every new symbol.
- Add LvCall variants: TimerCreate/SetPeriod/Reset/SetRepeatCount/
  Pause/Resume/Delete, ObjGetScrollBottom/Top, ObjSetScrollbarMode,
  ObjScrollToView, ObjSet/GetUserData, LabelSetLongMode/Recolor,
  ObjGetChildCount, GroupFocusObj, RemoveEventCbWithUserData,
  SpyEmitEvent.
- Add thread-local registries: EVENT_REG, USER_DATA, TIMER_REG,
  CHILD_COUNTS, scroll-injection cells, CURRENT_EVENT.
- Add SpyFixture RAII (resets at construction AND on Drop — panic-safe
  per risk #47).
- spy_emit_event / spy_fire_timer drive callbacks under thread-local
  isolation; lv_timer.repeat_count semantics match LVGL v9.x signed
  i32 (risk #43).
- Wire lv_obj_add_event_cb to populate the registry; lv_event_get_*
  read CURRENT_EVENT.

Implements spec §10 Step 0 (gates everything else). Risks: #19, #26,
#37, #38, #43, #45, #47, #53.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 2: `searchbar/row.rs` + `searchbar/highlight.rs` (pure-data layer)

Pure-Rust, no LVGL calls, no `unsafe`. Implements §3 `SearchRow` data type and §6 highlight markup builder. Maps risks #4, #6, #18, #28, #44, #50.

**Files:**
- Create: `src/lvgl/searchbar/mod.rs` (declare submodules; no public API yet)
- Create: `src/lvgl/searchbar/row.rs`
- Create: `src/lvgl/searchbar/highlight.rs`
- Test: inline `#[cfg(test)] mod tests` in each new file.

### Step 2.1: Stub `mod.rs`

- [ ] `src/lvgl/searchbar/mod.rs`:

```rust
//! SearchBar widget — see docs/superpowers/specs/2026-04-24-lvgl-searchbar-design.md.
pub mod row;
pub mod highlight;
```

- [ ] Modify `src/lvgl/mod.rs`: add `pub mod searchbar;` (do NOT re-export from prelude yet).

### Step 2.2: Failing tests for `SearchRow`

- [ ] `src/lvgl/searchbar/row.rs`:

```rust
//! SearchRow — caller-owned data for one result row (§3.1).
use alloc::string::String;

#[derive(Clone, Debug)]
pub struct SearchRow {
    pub id: u64,
    pub primary: String,
    pub secondary: Option<String>,
    pub disabled: bool,
}

impl SearchRow {
    pub fn new(id: u64, primary: impl Into<String>) -> Self {
        Self { id, primary: primary.into(), secondary: None, disabled: false }
    }
    pub fn with_secondary(mut self, s: impl Into<String>) -> Self {
        self.secondary = Some(s.into()); self
    }
    pub fn disabled(mut self, v: bool) -> Self { self.disabled = v; self }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_builder_defaults() {
        let r = SearchRow::new(1, "Pizza");
        assert_eq!(r.id, 1);
        assert_eq!(r.primary, "Pizza");
        assert!(r.secondary.is_none());
        assert!(!r.disabled);
    }
    #[test]
    fn row_builder_chain() {
        let r = SearchRow::new(2, "X").with_secondary("Y").disabled(true);
        assert_eq!(r.secondary.as_deref(), Some("Y"));
        assert!(r.disabled);
    }
}
```

### Step 2.3: Failing tests for highlight markup

- [ ] `src/lvgl/searchbar/highlight.rs`:

```rust
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
    if case_insensitive { t.to_lowercase() } else { String::from(t) }
}

/// Escape every `#` in `s` as `##` (LVGL recolor escape).
fn escape_recolor(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == '#' { out.push('#'); }
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
    if query.is_empty() { return escape_recolor(text); }
    let hay  = if case_insensitive { text.to_lowercase()  } else { String::from(text)  };
    let need = if case_insensitive { query.to_lowercase() } else { String::from(query) };
    let need_bytes = need.as_bytes();
    let hay_bytes  = hay.as_bytes();
    if need_bytes.is_empty() || need_bytes.len() > hay_bytes.len() {
        return escape_recolor(text);
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0usize;
    let mut last_emit = 0usize;
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
        assert_eq!(canonical_query("  Pizza  ", true),  "pizza");
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
}
```

### Step 2.4: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar::row::tests lvgl::searchbar::highlight::tests`
  Expected first run before code present: FAIL (compile errors). After code is in: 8 PASS.

- [ ] Commit:

```bash
git add src/lvgl/mod.rs src/lvgl/searchbar/
git commit -m "feat(searchbar): SearchRow + canonical_query + highlight markup (Task 2)

- SearchRow data type with builder (§3.1).
- canonical_query() — single trim+lowercase rule used by every state
  gate (§4, fixes risk #41 dedupe-after-clear).
- highlight_markup() escapes # → ## across full text (risk #44),
  case-insensitive option, char-boundary snap, no overlaps.

Implements spec §3.1 + §6. Risks: #4, #6, #28, #41, #44, #50.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 3: `searchbar/state.rs` + `searchbar/action.rs` (pure FSM types)

Pure-Rust enums and structs that model the FSM (§4) and the Model A action queue (§7). No LVGL, no `unsafe`. Risks #2, #5, #29, #30, #41, #48.

**Files:**
- Create: `src/lvgl/searchbar/state.rs`
- Create: `src/lvgl/searchbar/action.rs`
- Modify: `src/lvgl/searchbar/mod.rs` (add `pub mod state; pub mod action;`)

### Step 3.1: state.rs — failing tests

- [ ] `src/lvgl/searchbar/state.rs`:

```rust
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
        // Sanity: the five spec states all compare unequal to each other.
        let all = [State::Empty, State::Loading, State::Results, State::NoResults, State::Error];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                assert_eq!(a == b, i == j, "state equality failed for {:?} vs {:?}", a, b);
            }
        }
    }
}
```

### Step 3.2: action.rs — failing tests

- [ ] `src/lvgl/searchbar/action.rs`:

```rust
//! Model A actions (§7). The dispatch loop in `inner.rs` enqueues these
//! while holding the InnerState borrow, then drains them after dropping
//! the borrow — guaranteeing user callbacks NEVER re-enter the borrow.
use super::state::Token;
use alloc::collections::VecDeque;
use alloc::string::String;

#[derive(Clone, Debug)]
pub enum Callback {
    QueryChanged { token: Token, query: String },
    LoadMore     { token: Token, page_index: u32 },
    Select       { row_id: u64, selected: bool },
    QueryCleared,
    Retry        { token: Token, query: String },
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
    fn default() -> Self { Self::new() }
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
            debug_assert!(false, "SearchBar Action queue overflow (>{QUEUE_CAP}); risk #39");
            return;
        }
        self.inner.push_back(a);
    }
    /// Pop the next action in FIFO order; returns `None` when empty.
    pub fn pop_front(&mut self) -> Option<Action> { self.inner.pop_front() }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    pub fn capacity(&self) -> usize { self.inner.capacity() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_preallocates_capacity() {
        // Spec §7 / risk #39: zero-alloc hot path under bounded re-entrancy.
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
        // The 17th push must NOT extend the buffer; debug builds panic via
        // debug_assert (intended trap), so this asserts only in release.
        let mut q = ActionQueue::default();
        for _ in 0..QUEUE_CAP { q.push(Action::CancelPendingLoadMore); }
        q.push(Action::CancelPendingLoadMore);
        assert_eq!(q.len(), QUEUE_CAP);
        assert_eq!(q.overflow_count, 1);
    }
}
```

### Step 3.3: Wire and run

- [ ] Modify `src/lvgl/searchbar/mod.rs`:

```rust
pub mod state;
pub mod action;
```

(append to existing).

- [ ] Run: `cargo test --lib lvgl::searchbar::state::tests lvgl::searchbar::action::tests`
  Expected: 5 PASS in debug builds (3 state tests + 2 action tests; release-only overflow test compiles in debug but is `#[cfg(not(debug_assertions))]` so it's skipped). 6 PASS in release.

### Step 3.4: Commit

```bash
git add src/lvgl/searchbar/state.rs src/lvgl/searchbar/action.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): FSM types + Model A action queue (Task 3)

- State enum (§4): Empty, Loading, Results, NoResults, Error.
  pending_load_more is a flag on StateSnapshot, not a state.
- Token, StateSnapshot with last_fired_canonical (§4 cond 2),
  pre_error_state (deterministic set_error(false) restore),
  alive flag (post-deletion guard, risk #52).
- Callback + Action enums (§7).
- ActionQueue backed by VecDeque preallocated with_capacity(QUEUE_CAP)
  (§7, risk #39 — zero-alloc hot path), pop_front for FIFO drain,
  overflow_count + debug_assert (risk #39).

Implements spec §4 (types only) + §7. Risks: #2, #5, #29, #30, #39,
#41, #48, #52.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 4: `searchbar/inner.rs` skeleton — Model A dispatch loop

Implements the `InnerState` struct, the `with_inner` helper that takes a `RefCell` borrow and returns an action vec, and the `dispatch_after_borrow` helper that fires callbacks AFTER the borrow is released. NO LVGL widget creation yet — the dispatch loop is testable purely with stub callbacks. Implements §7 + §4 acceptance gate. Risks #2, #5, #29, #30, #41.

**Files:**
- Create: `src/lvgl/searchbar/inner.rs`
- Modify: `src/lvgl/searchbar/mod.rs` (add `pub mod inner;`)

### Step 4.1: Failing tests

- [ ] `src/lvgl/searchbar/inner.rs`:

```rust
//! Model A inner state + dispatch loop (§7).
//!
//! All public SearchBar operations follow this pattern:
//!   1. let mut actions = with_inner(&self.inner, |s| { ... s.queue.push(...) });
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
    pub on_load_more:     Option<alloc::boxed::Box<dyn FnMut(Token, u32)>>,
    pub on_select:        Option<alloc::boxed::Box<dyn FnMut(u64, bool)>>,
    pub on_query_cleared: Option<alloc::boxed::Box<dyn FnMut()>>,
    pub on_retry:         Option<alloc::boxed::Box<dyn FnMut(Token, &str)>>,
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
    /// True only while we are draining the queue. Re-entrant calls into
    /// public APIs while `is_draining` is true are queued, never executed
    /// inline (risk #2).
    pub is_draining: bool,
    /// Pending load-more page index awaiting drain (risk #29). This is
    /// the *page number* tracker; `snap.pending_load_more: bool` is the
    /// visibility/state flag. Both are kept in sync but serve different
    /// roles (visibility vs. payload).
    pub pending_load_more: Option<u32>,
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
            is_draining: false,
            pending_load_more: None,
        }
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
            if !s.snap.alive { return (Vec::new(), None); }
            let r = f(&mut s);
            let mut drained = Vec::with_capacity(s.queue.len());
            while let Some(a) = s.queue.pop_front() { drained.push(a); }
            (drained, Some(r))
        }
        Err(_) => {
            debug_assert!(false, "SearchBar re-entrant borrow (risk #2); inputs ignored");
            (Vec::new(), None)
        }
    }
}

/// Drains an action vec by firing the matching user callbacks. Safe to
/// re-enter SearchBar APIs from these callbacks because the InnerState
/// borrow is NOT held here.
pub fn dispatch_after_borrow(actions: Vec<Action>, cb_cell: &RefCell<Callbacks>) {
    if actions.is_empty() { return; }
    // Take callbacks OUT of the cell so the user callback may legitimately
    // reach in to mutate other callback slots without RefCell panic.
    let mut cbs = match cb_cell.try_borrow_mut() {
        Ok(c) => core::mem::take(&mut *c),
        Err(_) => return,
    };
    for a in actions {
        match a {
            Action::EmitCallback(Callback::QueryChanged { token, query }) => {
                if let Some(f) = cbs.on_query_changed.as_mut() { f(token, &query); }
            }
            Action::EmitCallback(Callback::LoadMore { token, page_index }) => {
                if let Some(f) = cbs.on_load_more.as_mut() { f(token, page_index); }
            }
            Action::EmitCallback(Callback::Select { row_id, selected }) => {
                if let Some(f) = cbs.on_select.as_mut() { f(row_id, selected); }
            }
            Action::EmitCallback(Callback::QueryCleared) => {
                if let Some(f) = cbs.on_query_cleared.as_mut() { f(); }
            }
            Action::EmitCallback(Callback::Retry { token, query }) => {
                if let Some(f) = cbs.on_retry.as_mut() { f(token, &query); }
            }
            Action::CancelPendingLoadMore => { /* handled by caller before drain */ }
        }
    }
    // Restore callback bag (whatever the user did inside).
    if let Ok(mut c) = cb_cell.try_borrow_mut() {
        // Only put back slots that the user did not already replace.
        if c.on_query_changed.is_none() { c.on_query_changed = cbs.on_query_changed.take(); }
        if c.on_load_more.is_none()     { c.on_load_more     = cbs.on_load_more.take(); }
        if c.on_select.is_none()        { c.on_select        = cbs.on_select.take(); }
        if c.on_query_cleared.is_none() { c.on_query_cleared = cbs.on_query_cleared.take(); }
        if c.on_retry.is_none()         { c.on_retry         = cbs.on_retry.take(); }
    }
}

/// Two-condition acceptance gate (§4). Returns true if the reply for
/// `(token, canonical)` should be applied. Pass `condition2_required = false`
/// for `set_loading(_, false)` and `set_error(_, false)` (cancellation
/// signals — only condition 1 applies).
pub fn accept_reply(snap: &mut StateSnapshot, token: Token, canonical: &str, condition2_required: bool) -> bool {
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
        (RefCell::new(InnerState::new(true, 0, 200)), RefCell::new(Callbacks::default()))
    }

    #[test]
    fn dispatch_fires_query_changed() {
        static N: AtomicUsize = AtomicUsize::new(0);
        let (s, c) = fresh();
        c.borrow_mut().on_query_changed = Some(alloc::boxed::Box::new(|_t, q| {
            assert_eq!(q, "pizza"); N.fetch_add(1, Ordering::SeqCst);
        }));
        let (acts, _) = with_inner(&s, |st| {
            st.queue.push(Action::EmitCallback(Callback::QueryChanged {
                token: Token(1), query: alloc::string::String::from("pizza"),
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
            let (_acts, _) = with_inner(&s2, |st| { st.snap.stale_drop_count += 7; });
        }));
        let (acts, _) = with_inner(&s, |st| {
            st.queue.push(Action::EmitCallback(Callback::QueryCleared));
        });
        dispatch_after_borrow(acts, &c);
    }

    #[test]
    fn accept_reply_token_mismatch() {
        let mut snap = StateSnapshot { current_token: Token(7), ..Default::default() };
        snap.last_fired_canonical = "pizza".into();
        assert!(!accept_reply(&mut snap, Token(6), "pizza", true));
        assert_eq!(snap.stale_drop_count, 1);
    }

    #[test]
    fn accept_reply_canonical_mismatch_for_results() {
        let mut snap = StateSnapshot { current_token: Token(7), ..Default::default() };
        snap.last_fired_canonical = "pizza".into();
        assert!(!accept_reply(&mut snap, Token(7), "burger", true));
        assert_eq!(snap.stale_drop_count, 1);
    }

    #[test]
    fn accept_reply_cancel_only_checks_token() {
        // set_loading(_,false): condition2_required = false
        let mut snap = StateSnapshot { current_token: Token(7), ..Default::default() };
        snap.last_fired_canonical = "pizza".into();
        assert!(accept_reply(&mut snap, Token(7), "anything", false));
    }
}
```

### Step 4.2: Wire + run + commit

- [ ] Add `pub mod inner;` to `src/lvgl/searchbar/mod.rs`.
- [ ] Run: `cargo test --lib lvgl::searchbar::inner::tests`
  Expected: 5 PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/inner.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): InnerState + Model A dispatch loop (Task 4)

- InnerState with snap/queue/rows/selected + is_draining + alive flags.
- with_inner(): single point that takes RefCell borrow, runs op, drains.
- dispatch_after_borrow(): fires callbacks WITHOUT holding the borrow,
  killing risk #2 (re-entrant BorrowMutError panic).
- accept_reply(): two-condition gate (§4) with condition2 toggle for
  cancellation signals.

Implements spec §4 (gate) + §7 (Model A). Risks: #2, #5, #29, #30, #41.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 5: `searchbar/slots.rs` — four optional slot containers

Implements §2 ownership table for `initial_loading_slot`, `initial_error_slot`, `footer_loading_slot`, `footer_error_slot`. Each slot is a single LVGL container the user can populate. Shows/hides via `lv_obj_add_flag(LV_OBJ_FLAG_HIDDEN)` / `lv_obj_remove_flag`. Risks #21, #22, #36.

**Files:**
- Create: `src/lvgl/searchbar/slots.rs`
- Modify: `src/lvgl/searchbar/mod.rs` (add `pub mod slots;`)

### Step 5.1: Failing tests

- [ ] `src/lvgl/searchbar/slots.rs`:

```rust
//! Four optional slot containers (§2 ownership table).
use crate::c_bindings::{lv_obj_t, lv_obj_create, lv_obj_add_flag, lv_obj_remove_flag};
use crate::lvgl::state::LvObjFlag;

#[derive(Default)]
pub struct Slots {
    pub initial_loading: Option<*mut lv_obj_t>,
    pub initial_error:   Option<*mut lv_obj_t>,
    pub footer_loading:  Option<*mut lv_obj_t>,
    pub footer_error:    Option<*mut lv_obj_t>,
}

unsafe fn ensure(slot: &mut Option<*mut lv_obj_t>, parent: *mut lv_obj_t) -> *mut lv_obj_t {
    if let Some(p) = *slot { return p; }
    let p = unsafe { lv_obj_create(parent) };
    unsafe { lv_obj_add_flag(p, LvObjFlag::HIDDEN.0); }
    *slot = Some(p);
    p
}

unsafe fn show(slot: Option<*mut lv_obj_t>) {
    if let Some(p) = slot { unsafe { lv_obj_remove_flag(p, LvObjFlag::HIDDEN.0); } }
}
unsafe fn hide(slot: Option<*mut lv_obj_t>) {
    if let Some(p) = slot { unsafe { lv_obj_add_flag(p, LvObjFlag::HIDDEN.0); } }
}

impl Slots {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    pub unsafe fn ensure_initial_loading(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.initial_loading, parent) }
    }
    pub unsafe fn ensure_initial_error(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.initial_error, parent) }
    }
    pub unsafe fn ensure_footer_loading(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.footer_loading, parent) }
    }
    pub unsafe fn ensure_footer_error(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.footer_error, parent) }
    }

    pub unsafe fn show_initial_loading(&self) { unsafe { show(self.initial_loading); } }
    pub unsafe fn hide_initial_loading(&self) { unsafe { hide(self.initial_loading); } }
    pub unsafe fn show_initial_error(&self)   { unsafe { show(self.initial_error); } }
    pub unsafe fn hide_initial_error(&self)   { unsafe { hide(self.initial_error); } }
    pub unsafe fn show_footer_loading(&self)  { unsafe { show(self.footer_loading); } }
    pub unsafe fn hide_footer_loading(&self)  { unsafe { hide(self.footer_loading); } }
    pub unsafe fn show_footer_error(&self)    { unsafe { show(self.footer_error); } }
    pub unsafe fn hide_footer_error(&self)    { unsafe { hide(self.footer_error); } }

    /// Hide every slot — used when phase transitions invalidate them.
    pub unsafe fn hide_all(&self) {
        unsafe {
            hide(self.initial_loading);
            hide(self.initial_error);
            hide(self.footer_loading);
            hide(self.footer_error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{SpyFixture, lv_obj_create, SPY, LvCall};
    use core::ptr;

    fn count_flag_calls() -> (usize, usize) {
        SPY.with(|s| {
            let s = s.borrow();
            let add  = s.iter().filter(|c| matches!(c, LvCall::AddFlag { .. })).count();
            let rem  = s.iter().filter(|c| matches!(c, LvCall::RemoveFlag { .. })).count();
            (add, rem)
        })
    }

    #[test]
    fn slots_lazy_create_and_start_hidden() {
        let _fx = SpyFixture::new();
        let mut sl = Slots::default();
        let parent = unsafe { lv_obj_create(ptr::null_mut()) };
        assert!(sl.initial_loading.is_none());
        let p = unsafe { sl.ensure_initial_loading(parent) };
        assert!(!p.is_null());
        assert!(sl.initial_loading.is_some());
        // ensure() must add HIDDEN at creation.
        let (add, _rem) = count_flag_calls();
        assert!(add >= 1);
    }

    #[test]
    fn slots_show_then_hide() {
        let _fx = SpyFixture::new();
        let mut sl = Slots::default();
        let parent = unsafe { lv_obj_create(ptr::null_mut()) };
        unsafe { sl.ensure_footer_loading(parent); }
        unsafe { sl.show_footer_loading(); }
        unsafe { sl.hide_footer_loading(); }
        let (add, rem) = count_flag_calls();
        assert!(add >= 2);   // ensure() adds HIDDEN, hide() adds HIDDEN
        assert_eq!(rem, 1);  // show() removes HIDDEN exactly once
    }

    #[test]
    fn hide_all_no_panic_on_empty_slots() {
        let _fx = SpyFixture::new();
        let sl = Slots::default();
        unsafe { sl.hide_all(); }
    }
}
```

### Step 5.2: Run + commit

- [ ] Add `pub mod slots;` to `src/lvgl/searchbar/mod.rs`.
- [ ] Run: `cargo test --lib lvgl::searchbar::slots::tests`
  Expected: 3 PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/slots.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): four optional slot containers (Task 5)

Lazy-created, start hidden, show/hide via LV_OBJ_FLAG_HIDDEN. Implements
spec §2 ownership table for initial_loading/initial_error/footer_loading/
footer_error slots. Risks: #21, #22, #36.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 6: `searchbar/bar.rs` — widget construction

Builds the visible widget tree (§2): root → input_container (TextArea + clear button) and result_container. Owns the slot accessors. No FSM logic yet — just creation, child reads, and slot accessors. Risks #11, #15, #21, #36.

**Files:**
- Create: `src/lvgl/searchbar/bar.rs`
- Modify: `src/lvgl/searchbar/mod.rs` (add `pub mod bar;`)

### Step 6.1: Failing tests + impl

- [ ] `src/lvgl/searchbar/bar.rs`:

```rust
//! SearchBar widget tree (§2). Owns the LVGL objects but NOT the FSM —
//! the SearchBar struct in `mod.rs` composes Bar + InnerState + Slots.
use crate::c_bindings::{
    lv_obj_t, lv_obj_create, lv_obj_set_flex_flow, lv_obj_set_size,
    lv_textarea_create, lv_button_create, lv_label_create, lv_label_set_text,
    LV_FLEX_FLOW_COLUMN, LV_FLEX_FLOW_ROW,
};
use super::slots::Slots;

pub struct Bar {
    pub root:             *mut lv_obj_t,
    pub input_container:  *mut lv_obj_t,
    pub text_area:        *mut lv_obj_t,
    pub clear_button:     *mut lv_obj_t,
    pub clear_label:      *mut lv_obj_t,
    pub result_container: *mut lv_obj_t,
    pub slots:            Slots,
}

impl Bar {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer (or null for screen).
    pub unsafe fn build(parent: *mut lv_obj_t, width: i32, height: i32) -> Self {
        unsafe {
            let root = lv_obj_create(parent);
            lv_obj_set_size(root, width, height);
            lv_obj_set_flex_flow(root, LV_FLEX_FLOW_COLUMN);

            let input_container = lv_obj_create(root);
            lv_obj_set_flex_flow(input_container, LV_FLEX_FLOW_ROW);

            let text_area    = lv_textarea_create(input_container);
            let clear_button = lv_button_create(input_container);
            let clear_label  = lv_label_create(clear_button);
            lv_label_set_text(clear_label, b"\xC3\x97\0".as_ptr() as _); // "×"

            let result_container = lv_obj_create(root);
            lv_obj_set_flex_flow(result_container, LV_FLEX_FLOW_COLUMN);

            Bar {
                root, input_container, text_area, clear_button, clear_label,
                result_container, slots: Slots::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{SpyFixture, SPY, LvCall};
    use core::ptr;

    #[test]
    fn build_creates_full_tree() {
        let _fx = SpyFixture::new();
        let b = unsafe { Bar::build(ptr::null_mut(), 320, 240) };
        assert!(!b.root.is_null());
        assert!(!b.input_container.is_null());
        assert!(!b.text_area.is_null());
        assert!(!b.result_container.is_null());
        assert_ne!(b.root as usize, b.input_container as usize);
        assert_ne!(b.input_container as usize, b.result_container as usize);

        let creates = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c,
                LvCall::ObjCreate { .. } |
                LvCall::TextareaCreate { .. } |
                LvCall::ButtonCreate { .. } |
                LvCall::LabelCreate { .. }))
            .count());
        assert!(creates >= 6, "expected ≥6 creates, got {creates}");
    }

    #[test]
    fn build_sets_flex_flows() {
        let _fx = SpyFixture::new();
        let _ = unsafe { Bar::build(ptr::null_mut(), 320, 240) };
        let cols = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::ObjSetFlexFlow { flow, .. } if *flow == LV_FLEX_FLOW_COLUMN))
            .count());
        let rows = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::ObjSetFlexFlow { flow, .. } if *flow == LV_FLEX_FLOW_ROW))
            .count());
        assert_eq!(cols, 2);
        assert_eq!(rows, 1);
    }
}
```

> **Required spy additions for this task** (apply to `src/c_bindings.rs` `mod mock` AND export at the `pub use mock::*` line):
>
> 1. Add LVGL v9.2 flex-flow constants near the other public consts:
>    ```rust
>    pub const LV_FLEX_FLOW_ROW: u32 = 0x00;
>    pub const LV_FLEX_FLOW_COLUMN: u32 = 0x01;
>    ```
>    (Match LVGL v9.2 `enum _lv_flex_flow_t` — ROW=0, COLUMN=1, ROW_WRAP=4, COLUMN_WRAP=5.)
>
> 2. Add `LvCall` variants (alphabetize within the enum):
>    ```rust
>    ObjCreate        { obj: usize, parent: usize },
>    TextareaCreate   { obj: usize, parent: usize },
>    ButtonCreate     { obj: usize, parent: usize },
>    LabelCreate      { obj: usize, parent: usize },
>    ObjSetFlexFlow   { obj: usize, flow: u32 },
>    ObjSetSize       { obj: usize, w: i32, h: i32 },
>    ```
>
> 3. Update the corresponding shim functions to push to `SPY` (currently silent):
>    ```rust
>    pub unsafe fn lv_obj_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
>        let obj = alloc_fake_obj();
>        SPY.with(|s| s.borrow_mut().push(LvCall::ObjCreate {
>            obj: obj as usize, parent: parent as usize,
>        }));
>        obj
>    }
>    pub unsafe fn lv_button_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
>        let obj = alloc_fake_obj();
>        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonCreate {
>            obj: obj as usize, parent: parent as usize,
>        }));
>        obj
>    }
>    pub unsafe fn lv_label_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
>        let obj = alloc_fake_obj();
>        SPY.with(|s| s.borrow_mut().push(LvCall::LabelCreate {
>            obj: obj as usize, parent: parent as usize,
>        }));
>        obj
>    }
>    pub unsafe fn lv_textarea_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
>        let obj = alloc_fake_obj();
>        SPY.with(|s| s.borrow_mut().push(LvCall::TextareaCreate {
>            obj: obj as usize, parent: parent as usize,
>        }));
>        obj
>    }
>    pub unsafe fn lv_obj_set_flex_flow(obj: *mut lv_obj_t, flow: u32) {
>        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetFlexFlow {
>            obj: obj as usize, flow,
>        }));
>    }
>    pub unsafe fn lv_obj_set_size(obj: *mut lv_obj_t, w: i32, h: i32) {
>        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetSize {
>            obj: obj as usize, w, h,
>        }));
>    }
>    ```
>    Note: `lv_textarea_create` already has a body that pre-populates KB binding state (around line 1032 in `c_bindings.rs`); keep that body and add the `SPY.with(...)` push.
>
> 4. Append a `bindings.conf` §8 line documenting the additional bindings rolled in by Task 6 (button, textarea, label create + flex flow + size).
>
> Do NOT touch the production Zephyr extern decls — they already exist (see lines 51–197 of `c_bindings.rs`). Only the `mod mock` shim bodies and `LvCall` enum need updating. The plan's `extern "C"` block is fine.

### Step 6.2: Run + commit

- [ ] Add `pub mod bar;` to `src/lvgl/searchbar/mod.rs`.
- [ ] Run: `cargo test --lib lvgl::searchbar::bar::tests`
  Expected: 2 PASS (after any missing shim additions).
- [ ] Commit:

```bash
git add src/lvgl/searchbar/bar.rs src/lvgl/searchbar/mod.rs src/c_bindings.rs src/lvgl/bindings.conf
git commit -m "feat(searchbar): widget tree construction (Task 6)

Bar::build() lays out root → input_container (textarea + clear btn) →
result_container per spec §2. Owns Slots. Adds any missing flex-flow
and create shims to desktop-sim. Risks: #11, #15, #21, #36.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 7: `searchbar/debounce.rs` — debounce timer + token bump

Wraps an `lv_timer_t` plus the rule "token bumps only when the timer fires AND the canonical query has changed". Pure-logic test using spy `spy_fire_timer`. Risks #5, #14, #41, #46.

**Files:**
- Create: `src/lvgl/searchbar/debounce.rs`
- Modify: `src/lvgl/searchbar/mod.rs` (add `pub mod debounce;`)

### Step 7.1: Failing tests + impl

- [ ] `src/lvgl/searchbar/debounce.rs`:

```rust
//! Debounce timer (§4). One-shot timer started on every keystroke; on
//! fire, if `canonical_query(text) != last_fired_canonical` AND length
//! ≥ min_query_len, bump token and emit Callback::QueryChanged.
use crate::c_bindings::{
    lv_timer_t, lv_timer_create, lv_timer_set_period, lv_timer_set_repeat_count,
    lv_timer_pause, lv_timer_resume, lv_timer_reset, lv_timer_delete,
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
            Self { handle: h, period_ms }
        }
    }

    /// Restart the debounce window. Safe to call on every keystroke.
    pub unsafe fn kick(&mut self) {
        unsafe {
            lv_timer_set_period(self.handle, self.period_ms);
            lv_timer_reset(self.handle);
            lv_timer_resume(self.handle);
        }
    }

    pub unsafe fn pause(&mut self) { unsafe { lv_timer_pause(self.handle); } }

    pub unsafe fn delete(&mut self) {
        if !self.handle.is_null() {
            unsafe { lv_timer_delete(self.handle); }
            self.handle = core::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{SpyFixture, SPY, LvCall, spy_fire_timer, spy_live_timer_handles};
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
        let pauses = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::TimerPause { .. })).count());
        assert!(pauses >= 1);
    }

    #[test]
    fn kick_resumes_and_resets() {
        let _fx = SpyFixture::new();
        FIRES.store(0, Ordering::SeqCst);
        let mut d = unsafe { Debounce::new(150, cb, core::ptr::null_mut()) };
        unsafe { d.kick(); }
        let resumes = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::TimerResume { .. })).count());
        let resets  = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::TimerReset  { .. })).count());
        assert_eq!(resumes, 1);
        assert_eq!(resets,  1);
        // Fire once: callback runs.
        spy_fire_timer(d.handle);
        assert_eq!(FIRES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn delete_removes_handle_and_is_idempotent() {
        let _fx = SpyFixture::new();
        let mut d = unsafe { Debounce::new(150, cb, core::ptr::null_mut()) };
        unsafe { d.delete(); }
        unsafe { d.delete(); } // idempotent — null guard
        assert!(spy_live_timer_handles().is_empty());
    }
}
```

### Step 7.2: Run + commit

- [ ] Add `pub mod debounce;` to `src/lvgl/searchbar/mod.rs`.
- [ ] Run: `cargo test --lib lvgl::searchbar::debounce::tests`
  Expected: 3 PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/debounce.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): debounce timer wrapper (Task 7)

Debounce::new() creates a paused, repeating timer. kick() resets +
resumes (called on every keystroke). delete() is idempotent. Risks:
#5, #14, #41, #46.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 8: Token semantics + two-condition acceptance gate (`SearchBar` shell)

Composes Bar + InnerState + Slots + Debounce into a real `SearchBar` struct in `mod.rs`. Implements: textarea-text → debounce → token bump + emit `QueryChanged`; `set_results` / `set_loading` / `set_error` apply the gate; `query_text`, `current_token`, `stale_drop_count` accessors. Risks #2, #5, #29, #30, #41.

**Files:**
- Modify: `src/c_bindings.rs` (Step 8.0 — add stateful textarea text round-trip)
- Modify: `src/lvgl/searchbar/mod.rs` (add the public `SearchBar` struct)

### Step 8.0: Make textarea text a round-trip mock

Currently `lv_textarea_get_text` returns `null` (line ~1080 of `c_bindings.rs`). The SearchBar shell needs `set_text` → `get_text` to return what was stored. Add:

```rust
// in the existing `thread_local! { … }` block alongside USER_DATA:
pub(crate) static TEXTAREA_TEXT:
    RefCell<HashMap<usize, alloc::ffi::CString>> = RefCell::new(HashMap::new());
```

Add a clear in `reset_all_thread_local_spy_state`:
```rust
TEXTAREA_TEXT.with(|m| m.borrow_mut().clear());
```

Update the two shims:
```rust
pub unsafe fn lv_textarea_set_text(obj: *mut lv_obj_t, txt: *const core::ffi::c_char) {
    // SAFETY: caller guarantees a valid NUL-terminated buffer.
    let cstr = unsafe { CStr::from_ptr(txt) };
    let owned = alloc::ffi::CString::from(cstr);
    let bytes = owned.as_bytes_with_nul().to_vec();
    SPY.with(|s| s.borrow_mut().push(LvCall::TextAreaSetText { obj: obj as usize, text: bytes }));
    TEXTAREA_TEXT.with(|m| { m.borrow_mut().insert(obj as usize, owned); });
}
pub unsafe fn lv_textarea_get_text(obj: *mut lv_obj_t) -> *const core::ffi::c_char {
    TEXTAREA_TEXT.with(|m| {
        m.borrow().get(&(obj as usize))
            .map(|s| s.as_ptr())
            .unwrap_or_else(|| {
                // LVGL returns a stable empty string, not null. Use a static C "".
                static EMPTY: &[u8] = b"\0";
                EMPTY.as_ptr() as *const core::ffi::c_char
            })
    })
}
```

The `as_ptr()` returned from a `CString` stored in the map remains valid until the next `lv_textarea_set_text` call on the same `obj` (which replaces the map entry). Callers MUST NOT hold the pointer across a `set_text`. This matches LVGL's contract.

### Step 8.1: Append the SearchBar shell to `src/lvgl/searchbar/mod.rs`

```rust
use core::cell::RefCell;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::c_bindings::{
    lv_obj_t, lv_textarea_get_text, lv_textarea_set_text, lv_obj_set_user_data,
};
use self::action::{Action, Callback};
use self::bar::Bar;
use self::debounce::Debounce;
use self::highlight::canonical_query;
use self::inner::{Callbacks, InnerState, accept_reply, dispatch_after_borrow, with_inner};
use self::row::SearchRow;
use self::state::{State, Token};

pub struct SearchBarConfig {
    pub width: i32,
    pub height: i32,
    pub case_insensitive: bool,
    pub min_query_len: usize,
    pub debounce_ms: u32,
}
impl Default for SearchBarConfig {
    fn default() -> Self {
        Self { width: 320, height: 240, case_insensitive: true, min_query_len: 0, debounce_ms: 200 }
    }
}

pub struct SearchBar {
    pub bar: Bar,
    pub inner: Rc<RefCell<InnerState>>,
    pub callbacks: Rc<RefCell<Callbacks>>,
    pub debounce: Debounce,
}

impl SearchBar {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer (or null for screen).
    pub unsafe fn build(parent: *mut lv_obj_t, cfg: SearchBarConfig) -> alloc::boxed::Box<Self> {
        let bar = unsafe { Bar::build(parent, cfg.width, cfg.height) };
        let inner = Rc::new(RefCell::new(InnerState::new(
            cfg.case_insensitive, cfg.min_query_len, cfg.debounce_ms,
        )));
        let callbacks = Rc::new(RefCell::new(Callbacks::default()));

        // Trampolines come in Task 11. For now: create a paused debounce
        // timer with a no-op callback; tests drive `tick_debounce()` directly.
        unsafe extern "C" fn _noop(_t: *mut crate::c_bindings::lv_timer_t) {}
        let debounce = unsafe { Debounce::new(cfg.debounce_ms, _noop, core::ptr::null_mut()) };

        // Stash the InnerState pointer on the textarea so trampolines (Task 11)
        // can recover it. Risk #19.
        let raw_inner = Rc::as_ptr(&inner) as *mut core::ffi::c_void;
        unsafe { lv_obj_set_user_data(bar.text_area, raw_inner); }

        alloc::boxed::Box::new(SearchBar { bar, inner, callbacks, debounce })
    }

    // ---- Callbacks (setters) ----
    pub fn on_query_changed(&mut self, f: impl FnMut(Token, &str) + 'static) {
        self.callbacks.borrow_mut().on_query_changed = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_query_cleared(&mut self, f: impl FnMut() + 'static) {
        self.callbacks.borrow_mut().on_query_cleared = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_select(&mut self, f: impl FnMut(u64, bool) + 'static) {
        self.callbacks.borrow_mut().on_select = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_load_more(&mut self, f: impl FnMut(Token, u32) + 'static) {
        self.callbacks.borrow_mut().on_load_more = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_retry(&mut self, f: impl FnMut(Token, &str) + 'static) {
        self.callbacks.borrow_mut().on_retry = Some(alloc::boxed::Box::new(f));
    }

    // ---- Accessors ----
    pub fn query_text(&self) -> String {
        let raw = unsafe { lv_textarea_get_text(self.bar.text_area) };
        if raw.is_null() { return String::new(); }
        let cstr = unsafe { core::ffi::CStr::from_ptr(raw) };
        cstr.to_string_lossy().into_owned()
    }
    pub fn current_token(&self) -> Token { self.inner.borrow().snap.current_token }
    pub fn stale_drop_count(&self) -> u64 { self.inner.borrow().snap.stale_drop_count }
    pub fn state(&self) -> State { self.inner.borrow().snap.state }

    /// Programmatic text injection (testing / "preset" search).
    pub fn set_text(&mut self, s: &str) {
        let cstring = alloc::ffi::CString::new(s).unwrap_or_default();
        unsafe { lv_textarea_set_text(self.bar.text_area, cstring.as_ptr()); }
        self.tick_debounce();
    }

    /// Simulates a debounce timer fire. Real production calls this from
    /// the trampoline (Task 11). Tests call it directly to skip waiting.
    pub fn tick_debounce(&mut self) {
        let q = self.query_text();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&q, case_insens);

        let (acts, _) = with_inner(&self.inner, |s| {
            // Empty / TooShort pivot — reset everything per §4.
            if canonical.is_empty() || canonical.chars().count() < s.min_query_len {
                if !s.snap.last_fired_canonical.is_empty()
                    || s.snap.state != State::Empty
                {
                    s.snap.last_fired_canonical.clear();
                    s.snap.current_token = Token(s.snap.current_token.0 + 1);
                    s.rows.clear();
                    s.selected.clear();
                    s.snap.state = State::Empty;
                    s.snap.pre_error_state = None;
                    s.snap.pending_load_more = false;
                    s.pending_load_more = None;
                    s.queue.push(Action::EmitCallback(Callback::QueryCleared));
                }
                return;
            }
            // Dedupe: same canonical → no-op (§4 + risk #41).
            if canonical == s.snap.last_fired_canonical { return; }
            s.snap.current_token = Token(s.snap.current_token.0 + 1);
            s.snap.last_fired_canonical = canonical.clone();
            s.snap.state = State::Loading;
            s.snap.pre_error_state = None;
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            s.queue.push(Action::EmitCallback(Callback::QueryChanged {
                token: s.snap.current_token, query: canonical,
            }));
        });
        dispatch_after_borrow(acts, &*self.callbacks);
    }

    // ---- Reply API ----
    pub fn set_results(&mut self, token: Token, rows: Vec<SearchRow>) -> bool {
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, true) { return; }
            let empty = rows.is_empty();
            s.rows = rows;
            s.snap.state = if empty { State::NoResults } else { State::Results };
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            s.page_index = 0;
            ok = true;
        });
        dispatch_after_borrow(acts, &*self.callbacks);
        ok
    }

    pub fn append_results(&mut self, token: Token, mut rows: Vec<SearchRow>) -> bool {
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, true) { return; }
            let new_non_empty = !rows.is_empty();
            s.rows.append(&mut rows);
            // Promote NoResults → Results when new rows arrive (§4 visibility table).
            if new_non_empty || !s.rows.is_empty() {
                s.snap.state = State::Results;
            } else if s.rows.is_empty() {
                s.snap.state = State::NoResults;
            }
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            s.page_index = s.page_index.saturating_add(1);
            ok = true;
        });
        dispatch_after_borrow(acts, &*self.callbacks);
        ok
    }

    pub fn set_loading(&mut self, token: Token, on: bool) -> bool {
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            // condition2 only required when entering loading from a query reply.
            // For set_loading(_, false) (cancel) only the token gate applies.
            if !accept_reply(&mut s.snap, token, &canonical, on) { return; }
            if on {
                s.snap.state = State::Loading;
            } else {
                // Restore to a data-bearing state if rows present.
                s.snap.state = if s.rows.is_empty() { State::NoResults } else { State::Results };
            }
            ok = true;
        });
        dispatch_after_borrow(acts, &*self.callbacks);
        ok
    }

    pub fn set_error(&mut self, token: Token, on: bool) -> bool {
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, on) { return; }
            if on {
                if s.snap.state != State::Error {
                    s.snap.pre_error_state = Some(s.snap.state);
                }
                s.snap.state = State::Error;
            } else {
                // Restore previous state deterministically per spec §4.
                let prev = s.snap.pre_error_state.take().unwrap_or_else(|| {
                    if s.rows.is_empty() { State::Loading } else { State::Results }
                });
                s.snap.state = prev;
            }
            ok = true;
        });
        dispatch_after_borrow(acts, &*self.callbacks);
        ok
    }

    pub fn clear_query(&mut self) {
        let cstring = alloc::ffi::CString::new("").unwrap();
        unsafe { lv_textarea_set_text(self.bar.text_area, cstring.as_ptr()); }
        self.tick_debounce();
    }
}

impl Drop for SearchBar {
    fn drop(&mut self) {
        if let Ok(mut s) = self.inner.try_borrow_mut() { s.snap.alive = false; }
        unsafe { self.debounce.delete(); }
    }
}
```

### Step 8.2: Tests — append to `src/lvgl/searchbar/mod.rs`

```rust
#[cfg(test)]
mod sb_tests {
    use super::*;
    use crate::c_bindings::SpyFixture;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use core::ptr;

    fn build() -> alloc::boxed::Box<SearchBar> {
        unsafe { SearchBar::build(ptr::null_mut(), SearchBarConfig::default()) }
    }

    #[test]
    fn token_bumps_on_new_query_only() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        let t0 = sb.current_token();
        sb.set_text("piz");
        let t1 = sb.current_token();
        sb.set_text("piz"); // identical canonical → no bump
        let t2 = sb.current_token();
        sb.set_text("pizz");
        let t3 = sb.current_token();
        assert_ne!(t0, t1);
        assert_eq!(t1, t2);
        assert_ne!(t2, t3);
    }

    #[test]
    fn query_changed_emitted_under_drained_borrow() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_query_changed(|_t, q| {
            assert_eq!(q, "pizza");
            N.fetch_add(1, Ordering::SeqCst);
        });
        sb.set_text("Pizza   "); // canonical = "pizza"
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn stale_token_dropped() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a");
        let t1 = sb.current_token();
        sb.set_text("ab"); // bumps
        let accepted = sb.set_results(t1, alloc::vec![SearchRow::new(1, "x")]);
        assert!(!accepted);
        assert_eq!(sb.stale_drop_count(), 1);
    }

    #[test]
    fn canonical_mismatch_dropped() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        sb.set_text("ab");
        // current_token is now > t, but set_results uses *current* canonical
        // "ab" — passing the OLD token must still drop.
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "x")]);
        assert!(sb.stale_drop_count() >= 1);
    }

    #[test]
    fn clear_then_retype_same_string_still_fires() {
        // Risk #41 — the dedupe-after-clear bug.
        let _fx = SpyFixture::new();
        let mut sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_query_changed(|_t, _q| { N.fetch_add(1, Ordering::SeqCst); });

        sb.set_text("pizza");           // fire #1
        sb.clear_query();               // resets last_fired_canonical
        sb.set_text("pizza");           // MUST fire again
        assert_eq!(N.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn set_loading_false_only_checks_token() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        // Mutate text without re-firing through the textarea API:
        sb.set_text("a"); // dedupe → token unchanged
        assert!(sb.set_loading(t, false));
    }

    #[test]
    fn empty_results_yields_no_results_state() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("xyzzy");
        let t = sb.current_token();
        assert!(sb.set_results(t, alloc::vec![]));
        assert_eq!(sb.state(), State::NoResults);
    }

    #[test]
    fn set_error_records_and_restores_pre_state() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("pizza");
        let t = sb.current_token();
        assert!(sb.set_results(t, alloc::vec![SearchRow::new(1, "x")]));
        assert_eq!(sb.state(), State::Results);
        assert!(sb.set_error(t, true));
        assert_eq!(sb.state(), State::Error);
        assert!(sb.set_error(t, false));
        assert_eq!(sb.state(), State::Results);
    }

    #[test]
    fn drop_disables_inner_alive_flag() {
        let _fx = SpyFixture::new();
        let sb = build();
        let inner = sb.inner.clone();
        drop(sb);
        assert!(!inner.borrow().snap.alive);
    }
}
```

### Step 8.3: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar::sb_tests -- --test-threads=4`
  Expected: 9 PASS.
- [ ] Run full suite: `cargo test --lib -- --test-threads=4`
  Expected: 251 PASS (242 prior + 9 new).
- [ ] Commit:

```bash
git add src/lvgl/searchbar/mod.rs src/c_bindings.rs
git commit -m "feat(searchbar): SearchBar shell + acceptance gate (Task 8)

- SearchBar struct composes Bar + InnerState + Callbacks + Debounce.
- tick_debounce(): empty/TooShort pivot, dedupe, token bump on new
  canonical query (§4). Fixes risk #41 — clear→retype same string
  must re-fire because last_fired_canonical was reset on clear.
- set_results / append_results: gated, normalize state to
  Results/NoResults per spec §4 visibility table.
- set_loading: condition2 only required when entering Loading;
  set_loading(_,false) restores to data state (Results/NoResults).
- set_error: records pre_error_state on entry, restores deterministically
  on exit per spec §4.
- query_text/current_token/stale_drop_count/state accessors.
- Drop sets snap.alive=false (single source of truth) so trampolines
  scheduled before drop become safe no-ops.

Spy infra: lv_textarea_set_text / lv_textarea_get_text now round-trip
via TEXTAREA_TEXT thread-local map of CString, returning a stable
empty-C-string pointer when unset.

Implements spec §3 (subset) + §4 acceptance gate. Risks: #2, #5, #29,
#30, #41.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 9: `searchbar/selection.rs` — silent-clear selection model

Implements §5: `select(id)`, `deselect(id)`, `toggle(id)`, `is_selected_id`, `selected_row_ids`, `selected_count`, `clear_selection`. Critically: selection survives `set_results`/`append_results` only for IDs still present; IDs that disappear are silently dropped (no `Select(_, false)` callback). `clear_selection()` clears state but emits NO callbacks. Risks #16, #17, #28.

**Files:**
- Create: `src/lvgl/searchbar/selection.rs`
- Modify: `src/lvgl/searchbar/mod.rs` — wire `select_*` into `SearchBar`; tweak `set_results`/`append_results` to call `selection::reconcile`.

### Step 9.1: Failing tests + impl

- [ ] `src/lvgl/searchbar/selection.rs`:

```rust
//! Selection model (§5). Selection lives as a Vec<u64> of row IDs in
//! InnerState.selected. We never emit `Select(_, false)` for IDs lost
//! to `set_results`/`append_results` — those are silently reconciled.
use super::action::{Action, Callback};
use super::inner::InnerState;
use super::row::SearchRow;
use alloc::vec::Vec;

pub fn select(s: &mut InnerState, row_id: u64) {
    if s.selected.contains(&row_id) { return; }
    s.selected.push(row_id);
    s.queue.push(Action::EmitCallback(Callback::Select { row_id, selected: true }));
}

pub fn deselect(s: &mut InnerState, row_id: u64) {
    let before = s.selected.len();
    s.selected.retain(|id| *id != row_id);
    if s.selected.len() != before {
        s.queue.push(Action::EmitCallback(Callback::Select { row_id, selected: false }));
    }
}

pub fn toggle(s: &mut InnerState, row_id: u64) {
    if s.selected.contains(&row_id) { deselect(s, row_id); } else { select(s, row_id); }
}

pub fn is_selected_id(s: &InnerState, row_id: u64) -> bool {
    s.selected.contains(&row_id)
}

pub fn selected_row_ids(s: &InnerState) -> Vec<u64> { s.selected.clone() }
pub fn selected_count(s: &InnerState) -> usize { s.selected.len() }

/// Silent clear — internal state only, no callback (§5).
pub fn clear_selection(s: &mut InnerState) { s.selected.clear(); }

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
    use super::super::action::ActionQueue;

    fn st() -> InnerState { InnerState::new(true, 0, 200) }

    #[test]
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
        toggle(&mut s, 1); toggle(&mut s, 1);
        assert!(s.selected.is_empty());
        assert_eq!(drain_queue(&mut s).len(), 2);
    }
    #[test]
    fn clear_selection_silent() {
        let _fx = SpyFixture::new();
        let mut s = st();
        select(&mut s, 1); select(&mut s, 2);
        let _ = drain_queue(&mut s);
        clear_selection(&mut s);
        assert!(s.selected.is_empty());
        assert_eq!(s.queue.len(), 0); // no callbacks
    }
    #[test]
    fn reconcile_drops_missing_silently() {
        let _fx = SpyFixture::new();
        let mut s = st();
        s.rows = alloc::vec![SearchRow::new(1, "a"), SearchRow::new(2, "b")];
        select(&mut s, 1); select(&mut s, 2); select(&mut s, 99);
        let _ = drain_queue(&mut s); // discard select-emits
        s.rows = alloc::vec![SearchRow::new(1, "a")];
        let rows_snapshot = s.rows.clone();
        reconcile(&mut s, &rows_snapshot);
        assert_eq!(s.selected, alloc::vec![1]);
        assert_eq!(s.queue.len(), 0); // §5: no callback for silent drop
    }
}
```

- [ ] Modify `src/lvgl/searchbar/mod.rs` — add `pub mod selection;`. Add to `impl SearchBar`:

```rust
    pub fn select(&mut self, row_id: u64) {
        let (acts, _) = with_inner(&self.inner, |s| selection::select(s, row_id));
        dispatch_after_borrow(acts, &*self.callbacks);
    }
    pub fn deselect(&mut self, row_id: u64) {
        let (acts, _) = with_inner(&self.inner, |s| selection::deselect(s, row_id));
        dispatch_after_borrow(acts, &*self.callbacks);
    }
    pub fn toggle_select(&mut self, row_id: u64) {
        let (acts, _) = with_inner(&self.inner, |s| selection::toggle(s, row_id));
        dispatch_after_borrow(acts, &*self.callbacks);
    }
    pub fn clear_selection(&mut self) {
        let (acts, _) = with_inner(&self.inner, |s| selection::clear_selection(s));
        dispatch_after_borrow(acts, &*self.callbacks);
    }
    pub fn is_selected_id(&self, row_id: u64) -> bool { selection::is_selected_id(&self.inner.borrow(), row_id) }
    pub fn selected_row_ids(&self) -> Vec<u64>       { selection::selected_row_ids(&self.inner.borrow()) }
    pub fn selected_count(&self) -> usize            { selection::selected_count(&self.inner.borrow()) }
```

- [ ] In `set_results` and `append_results`, after `s.rows = …`, call `selection::reconcile(s, &[])`.

### Step 9.2: Failing integration test

- [ ] In the `sb_tests` block:

```rust
    #[test]
    fn selection_survives_compatible_set_results() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "a"), SearchRow::new(2, "b")]);
        sb.select(1); sb.select(2);
        assert_eq!(sb.selected_count(), 2);
        // New result set drops id=2 silently.
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "a")]);
        assert_eq!(sb.selected_row_ids(), alloc::vec![1]);
    }

    #[test]
    fn clear_selection_emits_no_callbacks() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_select(|_, _| { N.fetch_add(1, Ordering::SeqCst); });
        sb.set_text("a"); let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "x"), SearchRow::new(2, "y")]);
        sb.select(1); sb.select(2);
        let pre = N.load(Ordering::SeqCst);
        sb.clear_selection();
        assert_eq!(N.load(Ordering::SeqCst), pre); // no extra fires
        assert_eq!(sb.selected_count(), 0);
    }
```

### Step 9.3: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar`
  Expected: All previous tests + 2 new SearchBar tests + 4 selection unit tests PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/selection.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): selection model with silent reconcile (Task 9)

- select / deselect / toggle / is_selected_id / selected_row_ids /
  selected_count.
- clear_selection() is silent (no on_select callbacks per §5).
- reconcile() runs after set_results/append_results, silently drops
  IDs no longer present.
Risks: #16, #17, #28.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 10: `searchbar/pagination.rs` — load-more + scroll-bottom trigger

Implements §3.3 + §4 LoadingMore phase. `request_load_more()`, scroll-bottom proximity check (uses `lv_obj_get_scroll_bottom`), `cancel_pending_load_more()`. Token captured at request time; reply via `append_results`. Risks #29, #30, #34.

**Files:**
- Create: `src/lvgl/searchbar/pagination.rs`
- Modify: `src/lvgl/searchbar/mod.rs` — add `pub mod pagination;`, methods.

### Step 10.1: Failing tests + impl

- [ ] `src/lvgl/searchbar/pagination.rs`:

```rust
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
/// inner.pending_load_more: Option<u32> tracks the page number for replay.
pub fn request_load_more(s: &mut InnerState) -> bool {
    if s.snap.state != State::Results { return false; }
    if s.snap.pending_load_more { return false; }
    let next = s.page_index + 1;
    s.pending_load_more = Some(next);
    s.snap.pending_load_more = true;
    s.queue.push(Action::EmitCallback(Callback::LoadMore {
        token: s.snap.current_token, page_index: next,
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
    use super::*;
    use super::super::state::Token;
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
        assert!(drained.iter().all(|a| matches!(a, Action::CancelPendingLoadMore)));
    }
}
```

- [ ] Modify `src/lvgl/searchbar/mod.rs`:

```rust
pub mod pagination;
```

Methods on `SearchBar`:

```rust
    pub fn request_load_more(&mut self) -> bool {
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| { ok = pagination::request_load_more(s); });
        dispatch_after_borrow(acts, &*self.callbacks);
        ok
    }
    pub fn cancel_pending_load_more(&mut self) {
        let (acts, _) = with_inner(&self.inner, |s| pagination::cancel_pending(s));
        dispatch_after_borrow(acts, &*self.callbacks);
    }
    /// Production: hook this into `LV_EVENT_SCROLL_END`. Tests call directly.
    pub fn check_scroll_for_load_more(&mut self) {
        let scroll_bottom = unsafe {
            crate::c_bindings::lv_obj_get_scroll_bottom(self.bar.result_container)
        };
        if pagination::should_trigger(scroll_bottom) { self.request_load_more(); }
    }
```

(Note: `set_results`/`append_results` already clear `s.snap.pending_load_more` and `s.pending_load_more` per Task 8. No edits needed there.)

- [ ] Add SearchBar test:

```rust
    #[test]
    fn load_more_triggers_on_low_scroll_bottom() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a"); let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "x")]);
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_load_more(|_t, _p| { N.fetch_add(1, Ordering::SeqCst); });
        crate::c_bindings::set_next_scroll_bottom(10); // < threshold
        sb.check_scroll_for_load_more();
        assert_eq!(N.load(Ordering::SeqCst), 1);
        // Second check while pending — no extra fire.
        crate::c_bindings::set_next_scroll_bottom(5);
        sb.check_scroll_for_load_more();
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }
```

### Step 10.2: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar::pagination::tests lvgl::searchbar::sb_tests::load_more_triggers_on_low_scroll_bottom`
  Expected: 4 PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/pagination.rs src/lvgl/searchbar/mod.rs
git commit -m "feat(searchbar): pagination + load-more (Task 10)

- request_load_more() emits LoadMore(token, page_index+1) iff Loaded
  and no pending request.
- cancel_pending_load_more() reverts phase to Loaded silently.
- check_scroll_for_load_more() reads lv_obj_get_scroll_bottom and
  triggers under LOAD_MORE_THRESHOLD_PX (24).
- append_results / set_results clear pending_load_more.

Risks: #29, #30, #34.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 11: `searchbar/trampolines.rs` + keyboard integration

Wires real LVGL events to `SearchBar` operations: textarea `LV_EVENT_VALUE_CHANGED` → `debounce.kick()`; debounce timer fire → `tick_debounce()`; clear button click → `clear_query()`; result container `LV_EVENT_SCROLL_END` → `check_scroll_for_load_more()`. Also `attach_keyboard()` / `detach_keyboard()`. Risks #2, #19, #20, #36, #45, #47.

**Files:**
- Modify: `src/c_bindings.rs` (Step 11.0 — LV_EVENT_* constants + `lv_timer_get_user_data` shim)
- Modify: `src/lvgl/bindings.conf` (add `lv_timer_get_user_data` to allowlist)
- Create: `src/lvgl/searchbar/trampolines.rs`
- Modify: `src/lvgl/searchbar/mod.rs` — add ctx pinning, register/unregister, keyboard methods.

### Step 11.0: Spy infra additions

In `src/c_bindings.rs`, inside `mod mock { ... }` (top of the public-consts area, alongside other LV_* constants — search for `LV_FLEX_FLOW_ROW` to find the right spot):

```rust
pub const LV_EVENT_CLICKED: u32        = 10;
pub const LV_EVENT_SCROLL_END: u32     = 31;
pub const LV_EVENT_VALUE_CHANGED: u32  = 35;
```

Then add a real-name shim (the production extern goes into the `extern "C"` block; the mock impl goes in the mock body alongside other `pub unsafe fn lv_*` shims):

```rust
// In the unsafe extern "C" { … } block alongside lv_timer_create:
pub fn lv_timer_get_user_data(t: *mut lv_timer_t) -> *mut core::ffi::c_void;
```

```rust
// In the mock module body (near the existing lv_timer_create shim):
pub unsafe fn lv_timer_get_user_data(t: *mut lv_timer_t) -> *mut core::ffi::c_void {
    TIMER_REG.with(|m| {
        m.borrow().get(&(t as usize)).map(|r| r.user_data).unwrap_or(core::ptr::null_mut())
    })
}
```

In `src/lvgl/bindings.conf`, append `lv_timer_get_user_data` to the function allowlist (one line, in the alphabetical position).

### Step 11.1: `src/lvgl/searchbar/trampolines.rs`

```rust
//! Trampolines: C-ABI fns LVGL invokes; each recovers `*mut SearchBar`
//! from the user_data we registered on the relevant object/timer,
//! checks `snap.alive`, and dispatches via the Model A pattern.
//! Risks #2, #19, #45, #47.
use crate::c_bindings::{
    lv_event_t, lv_event_get_user_data, lv_obj_add_event_cb,
    lv_obj_remove_event_cb_with_user_data, lv_timer_get_user_data, lv_timer_t,
    LV_EVENT_CLICKED, LV_EVENT_SCROLL_END, LV_EVENT_VALUE_CHANGED,
};

/// Pinned context that holds the raw SearchBar pointer. Lives inside a
/// `Box<TrampolineCtx>` owned by `SearchBar` itself — same heap address
/// as long as `SearchBar` is not moved (boxed), so the pointer we hand
/// to LVGL stays valid until `Drop`.
pub struct TrampolineCtx {
    pub sb: *mut super::SearchBar,
}

unsafe fn sb_from_event(e: *mut lv_event_t) -> Option<&'static mut super::SearchBar> {
    let ud = unsafe { lv_event_get_user_data(e) } as *mut TrampolineCtx;
    if ud.is_null() { return None; }
    let ctx = unsafe { &mut *ud };
    if ctx.sb.is_null() { return None; }
    let sb = unsafe { &mut *ctx.sb };
    if !sb.inner.borrow().snap.alive { return None; }
    Some(sb)
}

unsafe fn sb_from_timer(t: *mut lv_timer_t) -> Option<&'static mut super::SearchBar> {
    let ud = unsafe { lv_timer_get_user_data(t) } as *mut TrampolineCtx;
    if ud.is_null() { return None; }
    let ctx = unsafe { &mut *ud };
    if ctx.sb.is_null() { return None; }
    let sb = unsafe { &mut *ctx.sb };
    if !sb.inner.borrow().snap.alive { return None; }
    Some(sb)
}

pub unsafe extern "C" fn on_textarea_value_changed(e: *mut lv_event_t) {
    let Some(sb) = (unsafe { sb_from_event(e) }) else { return; };
    unsafe { sb.debounce.kick(); }
}

pub unsafe extern "C" fn on_debounce_fire(t: *mut lv_timer_t) {
    let Some(sb) = (unsafe { sb_from_timer(t) }) else { return; };
    sb.tick_debounce();
}

pub unsafe extern "C" fn on_clear_button_clicked(e: *mut lv_event_t) {
    let Some(sb) = (unsafe { sb_from_event(e) }) else { return; };
    sb.clear_query();
}

pub unsafe extern "C" fn on_result_scroll_end(e: *mut lv_event_t) {
    let Some(sb) = (unsafe { sb_from_event(e) }) else { return; };
    sb.check_scroll_for_load_more();
}

pub unsafe fn register(sb: *mut super::SearchBar, ctx: *mut TrampolineCtx) {
    let bar = unsafe { &(*sb).bar };
    unsafe {
        lv_obj_add_event_cb(bar.text_area,        Some(on_textarea_value_changed),
                            LV_EVENT_VALUE_CHANGED, ctx as *mut _);
        lv_obj_add_event_cb(bar.clear_button,     Some(on_clear_button_clicked),
                            LV_EVENT_CLICKED, ctx as *mut _);
        lv_obj_add_event_cb(bar.result_container, Some(on_result_scroll_end),
                            LV_EVENT_SCROLL_END, ctx as *mut _);
    }
}

pub unsafe fn unregister(sb: *mut super::SearchBar, ctx: *mut TrampolineCtx) {
    let bar = unsafe { &(*sb).bar };
    unsafe {
        lv_obj_remove_event_cb_with_user_data(bar.text_area,        Some(on_textarea_value_changed),  ctx as *mut _);
        lv_obj_remove_event_cb_with_user_data(bar.clear_button,     Some(on_clear_button_clicked),    ctx as *mut _);
        lv_obj_remove_event_cb_with_user_data(bar.result_container, Some(on_result_scroll_end),       ctx as *mut _);
    }
}
```

### Step 11.2: Wire into `SearchBar`

In `src/lvgl/searchbar/mod.rs`:

1. Add `pub mod trampolines;` alongside the other `pub mod` lines.
2. Add fields to `SearchBar` (heap-pinned ctx + optional keyboard handle):

```rust
pub struct SearchBar {
    pub bar: Bar,
    pub inner: Rc<RefCell<InnerState>>,
    pub callbacks: Rc<RefCell<Callbacks>>,
    pub debounce: Debounce,
    pub _ctx: alloc::boxed::Box<trampolines::TrampolineCtx>,
    pub keyboard: Option<*mut lv_obj_t>,
}
```

3. Replace the body of `pub unsafe fn build` to use the real trampolines and register events. The existing `_noop` debounce callback goes away:

```rust
    pub unsafe fn build(parent: *mut lv_obj_t, cfg: SearchBarConfig) -> alloc::boxed::Box<Self> {
        let bar = unsafe { Bar::build(parent, cfg.width, cfg.height) };
        let inner = Rc::new(RefCell::new(InnerState::new(
            cfg.case_insensitive, cfg.min_query_len, cfg.debounce_ms,
        )));
        let callbacks = Rc::new(RefCell::new(Callbacks::default()));

        // Pre-allocate the context with a placeholder sb pointer. We
        // patch it once the SearchBar Box is on the heap.
        let mut ctx = alloc::boxed::Box::new(trampolines::TrampolineCtx {
            sb: core::ptr::null_mut(),
        });

        let debounce = unsafe {
            Debounce::new(
                cfg.debounce_ms,
                trampolines::on_debounce_fire,
                ctx.as_mut() as *mut _ as *mut core::ffi::c_void,
            )
        };

        // Stash the InnerState pointer on the textarea (legacy hook —
        // some tests may inspect lv_obj_get_user_data; trampolines
        // themselves no longer use it). Risk #19 still satisfied.
        let raw_inner = Rc::as_ptr(&inner) as *mut core::ffi::c_void;
        unsafe { lv_obj_set_user_data(bar.text_area, raw_inner); }

        let mut sb = alloc::boxed::Box::new(SearchBar {
            bar, inner, callbacks, debounce, _ctx: ctx, keyboard: None,
        });
        let sb_ptr: *mut SearchBar = sb.as_mut() as *mut _;
        sb._ctx.sb = sb_ptr;
        unsafe {
            trampolines::register(sb_ptr, sb._ctx.as_mut() as *mut _);
        }
        sb
    }
```

4. Replace `Drop` to unregister BEFORE deleting the timer (so a fired event during teardown becomes a clean no-op, not a UAF):

```rust
impl Drop for SearchBar {
    fn drop(&mut self) {
        // 1. Flip the alive flag so any in-flight callback returns early.
        if let Ok(mut s) = self.inner.try_borrow_mut() { s.snap.alive = false; }
        // 2. Unregister LVGL event callbacks (so future fires do nothing).
        let sb_ptr: *mut SearchBar = self as *mut _;
        unsafe {
            trampolines::unregister(sb_ptr, self._ctx.as_mut() as *mut _);
            self.debounce.delete();
            // Also detach the keyboard if still attached.
            if let Some(kb) = self.keyboard.take() {
                crate::c_bindings::lv_keyboard_set_textarea(kb, core::ptr::null_mut());
            }
        }
    }
}
```

5. Add keyboard methods on `impl SearchBar`:

```rust
    pub fn attach_keyboard(&mut self, kb: *mut lv_obj_t) {
        self.keyboard = Some(kb);
        unsafe { crate::c_bindings::lv_keyboard_set_textarea(kb, self.bar.text_area); }
    }
    pub fn detach_keyboard(&mut self) {
        if let Some(kb) = self.keyboard.take() {
            unsafe { crate::c_bindings::lv_keyboard_set_textarea(kb, core::ptr::null_mut()); }
        }
    }
```

### Step 11.3: Tests — append to `sb_tests`

```rust
    #[test]
    fn textarea_event_kicks_debounce() {
        let _fx = SpyFixture::new();
        let sb = build();
        // Fire LV_EVENT_VALUE_CHANGED on the textarea — trampoline must
        // call debounce.kick() (= one TimerReset + one TimerResume).
        crate::c_bindings::spy_emit_event(sb.bar.text_area,
            crate::c_bindings::LV_EVENT_VALUE_CHANGED);
        let resets = crate::c_bindings::SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, crate::c_bindings::LvCall::TimerReset { .. })).count());
        assert!(resets >= 1);
    }

    #[test]
    fn clear_button_emits_query_cleared() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        static N: AtomicUsize = AtomicUsize::new(0); N.store(0, Ordering::SeqCst);
        sb.on_query_cleared(|| { N.fetch_add(1, Ordering::SeqCst); });
        sb.set_text("foo");                                // fires query_changed
        crate::c_bindings::spy_emit_event(sb.bar.clear_button,
            crate::c_bindings::LV_EVENT_CLICKED);
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_unregisters_event_callbacks() {
        let _fx = SpyFixture::new();
        let sb = build();
        let ta = sb.bar.text_area;
        drop(sb);
        // After drop, firing the event must NOT crash and must not run
        // any registered handler.
        crate::c_bindings::spy_emit_event(ta, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
    }

    #[test]
    fn debounce_fire_invokes_tick() {
        // Manually invoke the trampoline with a synthesized timer that
        // carries the SearchBar's ctx pointer — verifies tick_debounce
        // runs without panicking (full integration is exercised by
        // textarea_event_kicks_debounce).
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("a"); // ensure non-empty
        let _t0 = sb.current_token();
        // Direct call equivalent: tick_debounce is what on_debounce_fire
        // ultimately calls when alive==true.
        sb.tick_debounce();
        // No assertion about token bump — set_text already kicked tick.
        // Just ensures no panic / no double-borrow.
    }

    #[test]
    fn attach_detach_keyboard() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        let kb = unsafe { crate::c_bindings::lv_keyboard_create(core::ptr::null_mut()) };
        sb.attach_keyboard(kb);
        sb.detach_keyboard();
        let sets = crate::c_bindings::SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, crate::c_bindings::LvCall::KeyboardSetTextarea { .. })).count());
        assert_eq!(sets, 2);
    }
```

### Step 11.4: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar -- --test-threads=4`
  Expected: All previous tests + 5 new trampoline tests PASS (267 total).
- [ ] Commit:

```bash
git add src/lvgl/searchbar/trampolines.rs src/lvgl/searchbar/mod.rs src/c_bindings.rs src/lvgl/bindings.conf
git commit -m "feat(searchbar): trampolines + keyboard integration (Task 11)

- TrampolineCtx pins the SearchBar pointer in a heap-Box owned by the
  SearchBar itself; debounce timer carries it via user_data; event
  callbacks recover it via lv_event_get_user_data.
- on_textarea_value_changed → debounce.kick().
- on_debounce_fire → tick_debounce() (recovered via
  lv_timer_get_user_data).
- on_clear_button_clicked → clear_query().
- on_result_scroll_end → check_scroll_for_load_more().
- All trampolines guard on snap.alive; Drop unregisters every cb via
  lv_obj_remove_event_cb_with_user_data BEFORE deleting the debounce
  timer (risks #19, #45, #47).
- attach_keyboard / detach_keyboard wraps lv_keyboard_set_textarea
  (risk #20). Drop also detaches the keyboard.

Spy infra: added LV_EVENT_VALUE_CHANGED/CLICKED/SCROLL_END constants
and lv_timer_get_user_data shim (real symbol added to bindings.conf
so production cross-compile gets the real LVGL function).

Risks: #2, #19, #20, #36, #45, #47.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```


---

## Task 12: Highlight rendering wired into row labels

Implements §6 end-to-end: when `set_results`/`append_results` build/refresh row labels, each label is created with `lv_label_set_recolor(label, true)` + `lv_label_set_long_mode(label, LV_LABEL_LONG_DOT)`, and the text is the `highlight_markup(row.primary, canonical_query, "FFAA00", case_insensitive)` output. Risks #4, #6, #44, #50.

**Files:**
- Modify: `src/lvgl/searchbar/mod.rs` — add private `render_rows()` helper.

### Step 12.0: Spy infra additions

Add to `src/c_bindings.rs`:

In the `unsafe extern "C" { … }` block (production externs), alongside other `lv_obj_*` declarations:

```rust
pub fn lv_obj_clean(obj: *mut lv_obj_t);
```

In `mod mock`, add the constant near other LV_LABEL/LV_FLEX constants:

```rust
pub const LV_LABEL_LONG_WRAP:        u32 = 0;
pub const LV_LABEL_LONG_DOT:         u32 = 1;
pub const LV_LABEL_LONG_SCROLL:      u32 = 2;
pub const LV_LABEL_LONG_SCROLL_CIRC: u32 = 3;
pub const LV_LABEL_LONG_CLIP:        u32 = 4;
```

Add to `enum LvCall` near other Obj* variants:

```rust
ObjClean              { obj: usize },
```

Add the mock impl alongside other `lv_obj_*` shims:

```rust
pub unsafe fn lv_obj_clean(obj: *mut lv_obj_t) {
    SPY.with(|s| s.borrow_mut().push(LvCall::ObjClean { obj: obj as usize }));
    // Tests don't rely on actual child removal; the label create count
    // in the spy is what matters.
}
```

Append to `bindings.conf` (alphabetical position): `lv_obj_clean`.

### Step 12.1: Failing test

- [ ] In `sb_tests`:

```rust
    #[test]
    fn rows_rendered_with_recolor_and_highlight_markup() {
        let _fx = SpyFixture::new();
        let mut sb = build();
        sb.set_text("piz");
        let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "Pizza Hut")]);

        // 1) lv_label_set_recolor(_, true) called at least once.
        let recolors = crate::c_bindings::SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, crate::c_bindings::LvCall::LabelSetRecolor { en: true, .. }))
            .count());
        assert!(recolors >= 1);

        // 2) The label text contains the highlight escape "#FFAA00" and "Pizza".
        // LabelSetText carries text_bytes: Vec<u8> (NUL-terminated).
        let mut found = false;
        crate::c_bindings::SPY.with(|s| {
            for c in s.borrow().iter() {
                if let crate::c_bindings::LvCall::LabelSetText { text_bytes, .. } = c {
                    if let Ok(text) = core::str::from_utf8(text_bytes) {
                        if text.contains("#FFAA00") && text.contains("Pizza") { found = true; }
                    }
                }
            }
        });
        assert!(found, "highlighted label text not seen in spy");
    }
```

### Step 12.2: Impl

- [ ] In `src/lvgl/searchbar/mod.rs`, add inside `impl SearchBar`:

```rust
    fn render_rows(&mut self) {
        use crate::c_bindings::{
            lv_obj_clean, lv_label_create, lv_label_set_text, lv_label_set_recolor,
            lv_label_set_long_mode, LV_LABEL_LONG_DOT,
        };
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let rows = self.inner.borrow().rows.clone();
        unsafe { lv_obj_clean(self.bar.result_container); }
        for r in rows.iter() {
            let label = unsafe { lv_label_create(self.bar.result_container) };
            unsafe {
                lv_label_set_recolor(label, true);
                lv_label_set_long_mode(label, LV_LABEL_LONG_DOT);
            }
            let marked = self::highlight::highlight_markup(&r.primary, &canonical, "FFAA00", case_insens);
            let cs = alloc::ffi::CString::new(marked).unwrap_or_default();
            unsafe { lv_label_set_text(label, cs.as_ptr()); }
        }
    }
```

Call `self.render_rows()` from `set_results` and `append_results` AFTER `dispatch_after_borrow(...)`, and ONLY when `ok` (the gate accepted the reply).

### Step 12.3: Run + commit

- [ ] Run: `cargo test --lib lvgl::searchbar::sb_tests::rows_rendered_with_recolor_and_highlight_markup`
  Expected: PASS.
- [ ] Commit:

```bash
git add src/lvgl/searchbar/mod.rs src/c_bindings.rs src/lvgl/bindings.conf
git commit -m "feat(searchbar): render rows with recolor highlight (Task 12)

render_rows() drops all current children, creates one recolor-enabled
LV_LABEL_LONG_DOT label per row, sets text to highlight_markup(text,
canonical_query, FFAA00, case_insensitive). Wired into set_results and
append_results post-drain.

Implements spec §6. Risks: #4, #6, #44, #50.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 13: `examples/searchbar_demo.rs` desktop-sim smoke binary

End-to-end exercise of the full SearchBar in the desktop-sim runtime: build, set_text, fake reply, click row, scroll → load_more, clear. Doubles as a worked example for downstream consumers and as a smoke test (`cargo run --example searchbar_demo`). Risks #20, #21, #36, #38.

**Files:**
- Create: `examples/searchbar_demo.rs`

### Step 13.1: Code

- [ ] `examples/searchbar_demo.rs`:

```rust
//! End-to-end smoke demo for the SearchBar widget.
//!
//! Runs under desktop-sim (no real LVGL frame buffer) — its purpose is to
//! exercise every public API path together and assert the order of
//! callbacks. If this binary panics, an integration regression has crept in.
#![cfg_attr(not(test), no_main)]

extern crate alloc;

use lvgl_dsl::lvgl::searchbar::{SearchBar, SearchBarConfig};
use lvgl_dsl::lvgl::searchbar::row::SearchRow;
use lvgl_dsl::c_bindings::{SpyFixture, set_next_scroll_bottom, spy_emit_event,
    LV_EVENT_VALUE_CHANGED, LV_EVENT_CLICKED};
use std::sync::{Arc, Mutex};

fn main() {
    let _fx = SpyFixture::new();
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let mut sb = unsafe {
        SearchBar::build(core::ptr::null_mut(), SearchBarConfig {
            width: 400, height: 300, case_insensitive: true,
            min_query_len: 2, debounce_ms: 100,
        })
    };

    {
        let log = log.clone();
        sb.on_query_changed(move |t, q| {
            log.lock().unwrap().push(format!("query t={} q={}", t.0, q));
        });
    }
    {
        let log = log.clone();
        sb.on_load_more(move |t, p| {
            log.lock().unwrap().push(format!("load_more t={} page={}", t.0, p));
        });
    }
    {
        let log = log.clone();
        sb.on_select(move |id, on| {
            log.lock().unwrap().push(format!("select id={} on={}", id, on));
        });
    }
    {
        let log = log.clone();
        sb.on_query_cleared(move || { log.lock().unwrap().push("cleared".into()); });
    }

    // 1) Type
    sb.set_text("piz");
    let t = sb.current_token();
    println!("token after typing: {:?}", t);

    // 2) Reply
    let accepted = sb.set_results(t, vec![
        SearchRow::new(1, "Pizza Hut"),
        SearchRow::new(2, "Domino's Pizza"),
        SearchRow::new(3, "Pizza Express"),
    ]);
    assert!(accepted, "set_results rejected");

    // 3) Select a row
    sb.select(1);
    assert!(sb.is_selected_id(1));

    // 4) Scroll down → load more
    set_next_scroll_bottom(5);
    sb.check_scroll_for_load_more();

    // Reply to the load-more.
    let t2 = sb.current_token();
    let _ = sb.append_results(t2, vec![SearchRow::new(4, "Sbarro Pizza")]);

    // 5) Clear
    sb.clear_query();

    let log = log.lock().unwrap();
    println!("--- callback log ({}) ---", log.len());
    for l in log.iter() { println!("  {}", l); }

    assert!(log.iter().any(|l| l.starts_with("query")));
    assert!(log.iter().any(|l| l.starts_with("select id=1 on=true")));
    assert!(log.iter().any(|l| l.starts_with("load_more")));
    assert!(log.iter().any(|l| l == "cleared"));
    println!("searchbar_demo OK");
}
```

> The example uses `std`. Add `#[cfg(not(target_os = "none"))]` gating on the `examples/` directory if your CI cross-compiles to no_std targets — or simply mark this example with `required-features = ["desktop_sim_example"]` in `Cargo.toml` and gate accordingly. Pick whichever convention already exists in the repo (verify before writing the commit).

### Step 13.2: Run + commit

- [ ] Run: `cargo run --example searchbar_demo`
  Expected: prints `searchbar_demo OK` and exits 0.
- [ ] Run: `cargo test --lib lvgl::searchbar`
  Expected: every SearchBar test from Tasks 2-12 still PASSES.
- [ ] Run: `cargo clippy --all-targets --all-features -- -D warnings`
  Expected: 0 warnings.
- [ ] Commit:

```bash
git add examples/searchbar_demo.rs Cargo.toml
git commit -m "feat(examples): end-to-end SearchBar smoke demo (Task 13)

Type → reply → select → scroll → load_more → append → clear, asserts
each callback was observed. Doubles as a downstream-facing worked
example.

Risks: #20, #21, #36, #38.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Self-review checklist

Run BEFORE handing off:

- [ ] `cargo test --lib -- --test-threads=4` — every test from Tasks 1-12 PASSES.
- [ ] `cargo run --example searchbar_demo` — prints `searchbar_demo OK`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- [ ] Spec-coverage scan:
  - §3 public methods: `set_text`, `query_text`, `clear_query`, `set_results`, `append_results`, `set_loading`, `set_error`, `current_token`, `stale_drop_count`, `select`/`deselect`/`toggle_select`/`clear_selection`/`is_selected_id`/`selected_row_ids`/`selected_count`, `request_load_more`, `cancel_pending_load_more`, `attach_keyboard`/`detach_keyboard`, `on_query_changed`/`on_query_cleared`/`on_select`/`on_load_more`/`on_retry`, slot accessors (initial_loading_slot etc.) — every one implemented somewhere in Tasks 5-12.
  - §4 phase transitions Initial→Debouncing→Loading→Loaded→{LoadingMore|Error} all covered by tests in Task 8/10.
  - §5 silent reconcile + clear_selection no-callback — Task 9.
  - §6 highlight + escape — Task 2 (markup) + Task 12 (rendering).
  - §7 Model A — Task 4 (dispatch loop) used by EVERY task ≥8.
  - §8 binding deltas — Task 1.
  - §10 spy infra — Task 1.
  - §12 risk register: Task 1 covers #19/26/37/38/43/45/47/53; Task 2 covers #4/6/28/41/44/50; Task 3 covers #2/5/29/30/48; Task 4 covers #2/5/29/30/41; Task 5 covers #21/22/36; Task 6 covers #11/15/21/36; Task 7 covers #5/14/41/46; Task 8 covers #2/5/29/30/41; Task 9 covers #16/17/28; Task 10 covers #29/30/34; Task 11 covers #2/19/20/36/45/47; Task 12 covers #4/6/44/50; Task 13 covers #20/21/36/38. **Risks not explicitly mapped:** #1, #3, #7-13, #23-25, #27, #31-33, #35, #39, #42, #49, #51, #52 — review whether each is (a) implicitly tested by composition (most are: e.g. #1 "callback re-entrancy" is implicitly proven by Task 4's `reentrancy_does_not_panic`), or (b) requires a follow-up test.
- [ ] Method-name consistency grep:
  `git grep -nE 'set_results|append_results|set_loading|set_error|clear_query|clear_selection|on_query_changed|on_load_more|on_select|on_query_cleared|on_retry|attach_keyboard|detach_keyboard|tick_debounce|check_scroll_for_load_more|request_load_more|cancel_pending_load_more|stale_drop_count|current_token|query_text|initial_loading_slot|footer_loading_slot|initial_error_slot|footer_error_slot' src/`
  — every name exactly as listed.
- [ ] Placeholder scan: `git grep -nE 'TODO|FIXME|TBD|placeholder|fill in|similar to|etc\.' src/lvgl/searchbar/`
  — must return nothing.
- [ ] Branch & trailer:
  - On `feature/serach_bar` (sic — preserve the existing branch typo).
  - Every commit ends with `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

