//! SearchBar widget — see docs/superpowers/specs/2026-04-24-lvgl-searchbar-design.md.
pub mod action;
pub mod bar;
pub mod debounce;
pub mod highlight;
pub mod inner;
pub mod pagination;
pub mod row;
pub mod selection;
pub mod slots;
pub mod state;
pub mod trampolines;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::marker::PhantomPinned;
use core::pin::Pin;

use self::action::{Action, Callback};
use self::bar::Bar;
use self::debounce::Debounce;
use self::highlight::canonical_query;
use self::inner::{Callbacks, InnerState, accept_reply, dispatch_after_borrow, with_inner};
use self::row::SearchRow;
use self::state::{State, Token};
use crate::c_bindings::{
    lv_obj_delete, lv_obj_is_valid, lv_obj_set_user_data, lv_obj_t, lv_textarea_get_text,
    lv_textarea_set_text,
};

pub struct SearchBarConfig {
    pub width: i32,
    pub height: i32,
    pub case_insensitive: bool,
    pub min_query_len: usize,
    pub debounce_ms: u32,
}
impl Default for SearchBarConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 240,
            case_insensitive: true,
            min_query_len: 0,
            debounce_ms: 200,
        }
    }
}

pub struct SearchBar {
    pub bar: Bar,
    pub inner: Rc<RefCell<InnerState>>,
    pub callbacks: Rc<RefCell<Callbacks>>,
    pub debounce: RefCell<Debounce>,
    pub _ctx: alloc::boxed::Box<trampolines::TrampolineCtx>,
    pub keyboard: RefCell<Option<*mut lv_obj_t>>,
    /// Optional caller-provided row renderer. When `Some`, `render_rows`
    /// dispatches to this closure instead of producing the default
    /// highlighted-label row. The closure receives:
    ///   `(parent=result_container, &SearchRow, canonical_query, selected)`.
    /// Stored on `SearchBar` (not `Callbacks`) because the borrow shape
    /// differs: the inner-state borrow is released before invocation.
    #[allow(clippy::type_complexity)]
    pub row_renderer:
        Rc<RefCell<Option<alloc::boxed::Box<dyn FnMut(*mut lv_obj_t, &SearchRow, &str, bool)>>>>,
    _pin: PhantomPinned,
    /// Test-only snapshot of the most recent visibility decision computed
    /// by `sync_slot_visibility`. Lets headless tests inspect intent
    /// without needing real LVGL slot pointers.
    #[cfg(test)]
    pub last_sync: core::cell::Cell<SlotVis>,
}

/// Test-only struct mirroring the desired visibility of every lazy slot.
#[cfg(test)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SlotVis {
    pub initial_empty: bool,
    pub initial_loading: bool,
    pub footer_loading: bool,
    pub initial_error: bool,
    pub footer_error: bool,
}

#[cfg(test)]
impl SlotVis {
    pub fn all_hidden() -> Self {
        Self::default()
    }
}

impl SearchBar {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer (or null for screen).
    pub unsafe fn build(
        parent: *mut lv_obj_t,
        cfg: SearchBarConfig,
    ) -> Pin<alloc::boxed::Box<Self>> {
        let bar = unsafe { Bar::build(parent, cfg.width, cfg.height) };
        let inner = Rc::new(RefCell::new(InnerState::new(
            cfg.case_insensitive,
            cfg.min_query_len,
            cfg.debounce_ms,
        )));
        let callbacks = Rc::new(RefCell::new(Callbacks::default()));

        let mut ctx = alloc::boxed::Box::new(trampolines::TrampolineCtx {
            sb: core::ptr::null(),
        });

        let debounce = unsafe {
            Debounce::new(
                cfg.debounce_ms,
                trampolines::on_debounce_fire,
                ctx.as_mut() as *mut _ as *mut core::ffi::c_void,
            )
        };

        let raw_inner = Rc::as_ptr(&inner) as *mut core::ffi::c_void;
        unsafe {
            lv_obj_set_user_data(bar.text_area, raw_inner);
        }

        let mut sb = alloc::boxed::Box::pin(SearchBar {
            bar,
            inner,
            callbacks,
            debounce: RefCell::new(debounce),
            _ctx: ctx,
            keyboard: RefCell::new(None),
            row_renderer: Rc::new(RefCell::new(None)),
            _pin: PhantomPinned,
            #[cfg(test)]
            last_sync: core::cell::Cell::new(SlotVis::all_hidden()),
        });
        let sb_mut = unsafe { Pin::as_mut(&mut sb).get_unchecked_mut() };
        let sb_ptr: *const SearchBar = sb_mut as *const _;
        sb_mut._ctx.sb = sb_ptr;
        unsafe {
            trampolines::register(sb_ptr, sb_mut._ctx.as_mut() as *mut _);
        }
        sb
    }

    // ---- Callbacks (setters) ----
    pub fn on_query_changed(&self, f: impl FnMut(Token, &str) + 'static) {
        self.callbacks.borrow_mut().on_query_changed = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_query_cleared(&self, f: impl FnMut() + 'static) {
        self.callbacks.borrow_mut().on_query_cleared = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_select(&self, f: impl FnMut(u64, bool) + 'static) {
        self.callbacks.borrow_mut().on_select = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_load_more(&self, f: impl FnMut(Token, u32) + 'static) {
        self.callbacks.borrow_mut().on_load_more = Some(alloc::boxed::Box::new(f));
    }
    pub fn on_retry(&self, f: impl FnMut(Token, &str) + 'static) {
        self.callbacks.borrow_mut().on_retry = Some(alloc::boxed::Box::new(f));
    }

    // ---- Accessors ----
    pub fn query_text(&self) -> String {
        let raw = unsafe { lv_textarea_get_text(self.bar.text_area) };
        if raw.is_null() {
            return String::new();
        }
        let cstr = unsafe { core::ffi::CStr::from_ptr(raw) };
        cstr.to_string_lossy().into_owned()
    }
    pub fn current_token(&self) -> Token {
        self.inner.borrow().snap.current_token
    }
    pub fn stale_drop_count(&self) -> u64 {
        self.inner.borrow().snap.stale_drop_count
    }
    pub fn state(&self) -> State {
        self.inner.borrow().snap.state
    }

    /// Programmatic text injection (testing / "preset" search).
    pub fn set_text(&self, s: &str) {
        let cstring = alloc::ffi::CString::new(s).unwrap_or_default();
        unsafe {
            lv_textarea_set_text(self.bar.text_area, cstring.as_ptr());
        }
        // Note: sync_slot_visibility is invoked transitively via tick_debounce.
        self.tick_debounce();
    }

    /// Simulates a debounce timer fire. Real production calls this from
    /// the trampoline (Task 11). Tests call it directly to skip waiting.
    pub fn tick_debounce(&self) {
        let callbacks = self.callbacks.clone();
        let q = self.query_text();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&q, case_insens);

        let (acts, _) = with_inner(&self.inner, |s| {
            // Empty / TooShort pivot — reset everything per §4.
            if canonical.is_empty() || canonical.chars().count() < s.min_query_len {
                if !s.snap.last_fired_canonical.is_empty() || s.snap.state != State::Empty {
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
            if canonical == s.snap.last_fired_canonical {
                return;
            }
            s.snap.current_token = Token(s.snap.current_token.0 + 1);
            s.snap.last_fired_canonical = canonical.clone();
            s.snap.state = State::Loading;
            s.snap.pre_error_state = None;
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            s.queue.push(Action::EmitCallback(Callback::QueryChanged {
                token: s.snap.current_token,
                query: canonical,
            }));
        });
        self.sync_slot_visibility();
        dispatch_after_borrow(acts, &callbacks);
    }

    // ---- Reply API ----
    pub fn set_results(&self, token: Token, rows: Vec<SearchRow>) -> bool {
        let callbacks = self.callbacks.clone();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, true) {
                return;
            }
            let empty = rows.is_empty();
            s.rows = rows;
            selection::reconcile(s, &[]);
            s.snap.state = if empty {
                State::NoResults
            } else {
                State::Results
            };
            s.snap.pending_load_more = false;
            s.pending_load_more = None;
            s.page_index = 0;
            ok = true;
        });
        if ok {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
        ok
    }

    pub fn append_results(&self, token: Token, mut rows: Vec<SearchRow>) -> bool {
        let callbacks = self.callbacks.clone();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, true) {
                return;
            }
            let new_non_empty = !rows.is_empty();
            s.rows.append(&mut rows);
            selection::reconcile(s, &[]);
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
        if ok {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
        ok
    }

    pub fn set_loading(&self, token: Token, on: bool) -> bool {
        let callbacks = self.callbacks.clone();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            // condition2 only required when entering loading from a query reply.
            // For set_loading(_, false) (cancel) only the token gate applies.
            if !accept_reply(&mut s.snap, token, &canonical, on) {
                return;
            }
            if on {
                s.snap.state = State::Loading;
            } else {
                // Restore to a data-bearing state if rows present.
                s.snap.state = if s.rows.is_empty() {
                    State::NoResults
                } else {
                    State::Results
                };
            }
            ok = true;
        });
        self.sync_slot_visibility();
        dispatch_after_borrow(acts, &callbacks);
        ok
    }

    pub fn set_error(&self, token: Token, on: bool) -> bool {
        let callbacks = self.callbacks.clone();
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            if !accept_reply(&mut s.snap, token, &canonical, on) {
                return;
            }
            if on {
                if s.snap.state != State::Error {
                    s.snap.pre_error_state = Some(s.snap.state);
                }
                s.snap.state = State::Error;
            } else {
                // Restore previous state deterministically per spec §4.
                let prev = s.snap.pre_error_state.take().unwrap_or_else(|| {
                    if s.rows.is_empty() {
                        State::Loading
                    } else {
                        State::Results
                    }
                });
                s.snap.state = prev;
            }
            ok = true;
        });
        self.sync_slot_visibility();
        dispatch_after_borrow(acts, &callbacks);
        ok
    }

    pub fn select(&self, row_id: u64) {
        let callbacks = self.callbacks.clone();
        let (acts, dirty) = with_inner(&self.inner, |s| {
            selection::select(s, row_id);
            core::mem::replace(&mut s.selection_dirty, false)
        });
        if dirty.unwrap_or(false) {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
    }
    pub fn deselect(&self, row_id: u64) {
        let callbacks = self.callbacks.clone();
        let (acts, dirty) = with_inner(&self.inner, |s| {
            selection::deselect(s, row_id);
            core::mem::replace(&mut s.selection_dirty, false)
        });
        if dirty.unwrap_or(false) {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
    }
    pub fn toggle_select(&self, row_id: u64) {
        let callbacks = self.callbacks.clone();
        let (acts, dirty) = with_inner(&self.inner, |s| {
            selection::toggle(s, row_id);
            core::mem::replace(&mut s.selection_dirty, false)
        });
        if dirty.unwrap_or(false) {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
    }
    pub fn clear_selection(&self) {
        let callbacks = self.callbacks.clone();
        let (acts, dirty) = with_inner(&self.inner, |s| {
            selection::clear_selection(s);
            core::mem::replace(&mut s.selection_dirty, false)
        });
        if dirty.unwrap_or(false) {
            self.sync_slot_visibility();
            self.render_rows();
        } else {
            self.sync_slot_visibility();
        }
        dispatch_after_borrow(acts, &callbacks);
    }
    pub fn is_selected_id(&self, row_id: u64) -> bool {
        selection::is_selected_id(&self.inner.borrow(), row_id)
    }
    pub fn selected_row_ids(&self) -> Vec<u64> {
        selection::selected_row_ids(&self.inner.borrow())
    }
    pub fn selected_count(&self) -> usize {
        selection::selected_count(&self.inner.borrow())
    }

    /// Sets `pending_load_more=true`. The page is responsible for calling
    /// `set_loading(token, true)` afterwards to drive the FSM into Loading;
    /// visibility will then resolve to `footer_loading`.
    pub fn request_load_more(&self) -> bool {
        let callbacks = self.callbacks.clone();
        let mut ok = false;
        let (acts, _) = with_inner(&self.inner, |s| {
            ok = pagination::request_load_more(s);
        });
        self.sync_slot_visibility();
        dispatch_after_borrow(acts, &callbacks);
        ok
    }
    pub fn cancel_pending_load_more(&self) {
        let callbacks = self.callbacks.clone();
        let (acts, _) = with_inner(&self.inner, |s| pagination::cancel_pending(s));
        self.sync_slot_visibility();
        dispatch_after_borrow(acts, &callbacks);
    }
    /// Production: hook this into `LV_EVENT_SCROLL_END`. Tests call directly.
    pub fn check_scroll_for_load_more(&self) {
        let scroll_bottom =
            unsafe { crate::c_bindings::lv_obj_get_scroll_bottom(self.bar.result_container) };
        if pagination::should_trigger(scroll_bottom) {
            self.request_load_more();
        }
    }

    pub fn clear_query(&self) {
        let cstring = alloc::ffi::CString::new("").unwrap();
        unsafe {
            lv_textarea_set_text(self.bar.text_area, cstring.as_ptr());
        }
        // Note: sync_slot_visibility is invoked transitively via tick_debounce.
        self.tick_debounce();
    }

    pub fn attach_keyboard(&self, kb: *mut lv_obj_t) {
        *self.keyboard.borrow_mut() = Some(kb);
        unsafe {
            crate::c_bindings::lv_keyboard_set_textarea(kb, self.bar.text_area);
        }
    }
    pub fn detach_keyboard(&self) {
        if let Some(kb) = self.keyboard.borrow_mut().take() {
            unsafe {
                crate::c_bindings::lv_keyboard_set_textarea(kb, core::ptr::null_mut());
            }
        }
    }

    /// Install builder for the initial-empty slot's contents.
    /// `builder` is called with the slot's parent pointer; append children to it.
    /// Calling this replaces any existing children in the slot, then triggers
    /// `sync_slot_visibility` so the slot becomes visible if state is Empty.
    ///
    /// On a headless `test_stub` (null `root`), the builder is still
    /// invoked with a null parent so callers can no-op safely without LVGL.
    ///
    /// The slot is parented to the SearchBar root (NOT `result_container`):
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// the slot and leave a dangling pointer in `Slots`.
    pub fn set_initial_empty_hint<F>(&self, mut builder: F)
    where
        F: FnMut(*mut lv_obj_t),
    {
        unsafe {
            let parent = if self.bar.root.is_null() {
                core::ptr::null_mut()
            } else {
                let slot = self
                    .bar
                    .slots
                    .borrow_mut()
                    .ensure_initial_empty(self.bar.root);
                if !slot.is_null() {
                    crate::c_bindings::lv_obj_clean(slot);
                }
                slot
            };
            let liveness = self.inner.clone();
            builder(parent);
            if !liveness.borrow().snap.alive {
                return;
            }
        }
        self.sync_slot_visibility();
    }

    /// Install (or replace) a custom row renderer and re-render immediately.
    /// The closure is invoked once per row on every `render_rows` call,
    /// after the inner-state borrow has been released.
    pub fn set_row_renderer<F>(&self, f: F)
    where
        F: FnMut(*mut lv_obj_t, &SearchRow, &str, bool) + 'static,
    {
        *self.row_renderer.borrow_mut() = Some(alloc::boxed::Box::new(f));
        self.render_rows();
    }

    /// Map the current FSM state + rows-empty onto the lazy slots from
    /// spec §4. Called outside the `with_inner` borrow and before any
    /// pending user-callback dispatch, so callbacks may drop/re-enter us
    /// without leaving later widget visibility work that touches `self`.
    fn sync_slot_visibility(&self) {
        // Per state.rs §4 visibility table, the initial-vs-footer split is
        // driven by the `pending_load_more` flag, NOT by row count. This
        // matters for refinement-typing: editing a query that already has
        // results enters Loading with rows present but pending_load_more=false,
        // which must show the standard initial-loading affordance — not a
        // footer spinner.
        let (state, pending_load_more) = {
            let s = self.inner.borrow();
            (s.snap.state, s.snap.pending_load_more)
        };

        #[cfg(test)]
        let desired = SlotVis {
            initial_empty: matches!(state, State::Empty),
            initial_loading: matches!(state, State::Loading) && !pending_load_more,
            footer_loading: matches!(state, State::Loading) && pending_load_more,
            initial_error: matches!(state, State::Error) && !pending_load_more,
            footer_error: matches!(state, State::Error) && pending_load_more,
        };
        #[cfg(test)]
        self.last_sync.set(desired);

        let want_initial_empty = matches!(state, State::Empty);
        let want_initial_loading = matches!(state, State::Loading) && !pending_load_more;
        let want_footer_loading = matches!(state, State::Loading) && pending_load_more;
        let want_initial_error = matches!(state, State::Error) && !pending_load_more;
        let want_footer_error = matches!(state, State::Error) && pending_load_more;

        unsafe {
            let slots = self.bar.slots.borrow();
            slots.hide_all();
            if want_initial_empty && slots.initial_empty.is_some() {
                slots.show_initial_empty();
            }
            if want_initial_loading && slots.initial_loading.is_some() {
                slots.show_initial_loading();
            }
            if want_footer_loading && slots.footer_loading.is_some() {
                slots.show_footer_loading();
            }
            if want_initial_error && slots.initial_error.is_some() {
                slots.show_initial_error();
            }
            if want_footer_error && slots.footer_error.is_some() {
                slots.show_footer_error();
            }
        }
    }

    fn render_rows(&self) {
        use crate::c_bindings::{
            LV_LABEL_LONG_MODE_DOTS, lv_label_create, lv_label_set_long_mode, lv_label_set_recolor,
            lv_label_set_text, lv_obj_clean,
        };
        let case_insens = self.inner.borrow().case_insensitive;
        let canonical = canonical_query(&self.query_text(), case_insens);

        // Custom renderer path: snapshot rows + selection without holding
        // the inner borrow across the closure call (risk #2 re-entrancy).
        // The custom renderer is responsible for handling a null parent
        // pointer itself, which lets headless tests use `test_stub`.
        let renderer_cell = self.row_renderer.clone();
        let liveness = self.inner.clone();
        let mut renderer = renderer_cell.borrow_mut().take();
        if let Some(render_fn) = renderer.as_mut() {
            let (rows, selected): (alloc::vec::Vec<SearchRow>, alloc::vec::Vec<u64>) = {
                let s = self.inner.borrow();
                (s.rows.clone(), s.selected.clone())
            };
            let parent = self.bar.result_container;
            if !parent.is_null() {
                unsafe {
                    lv_obj_clean(parent);
                }
            }
            for row in &rows {
                let is_sel = selected.contains(&row.id);
                render_fn(parent, row, &canonical, is_sel);
                if !liveness.borrow().snap.alive {
                    return;
                }
            }
            if renderer_cell.borrow().is_none() {
                *renderer_cell.borrow_mut() = renderer;
            }
            return;
        }

        // Default renderer path requires a real result_container; skip
        // entirely when null so headless callers (test_stub) don't issue
        // LVGL calls against a sentinel pointer.
        if self.bar.result_container.is_null() {
            return;
        }

        let rows = self.inner.borrow().rows.clone();
        unsafe {
            lv_obj_clean(self.bar.result_container);
        }
        for r in rows.iter() {
            let label = unsafe { lv_label_create(self.bar.result_container) };
            unsafe {
                lv_label_set_recolor(label, true);
                lv_label_set_long_mode(label, LV_LABEL_LONG_MODE_DOTS);
            }
            let marked =
                self::highlight::highlight_markup(&r.primary, &canonical, "FFAA00", case_insens);
            let cs = alloc::ffi::CString::new(marked).unwrap_or_default();
            unsafe {
                lv_label_set_text(label, cs.as_ptr());
            }
        }
    }

    #[cfg(test)]
    pub fn with_inner_mut_for_test<F: FnOnce(&mut InnerState)>(&self, f: F) {
        let mut s = self.inner.borrow_mut();
        f(&mut s);
    }

    /// Construct an instance with all LVGL pointers set to null and no
    /// row renderer installed. For unit tests that exercise pure-Rust
    /// paths (e.g. custom row-renderer dispatch) without an LVGL display.
    #[cfg(test)]
    pub fn test_stub() -> Self {
        use core::ptr;
        let bar = Bar {
            root: ptr::null_mut(),
            input_container: ptr::null_mut(),
            text_area: ptr::null_mut(),
            clear_button: ptr::null_mut(),
            clear_label: ptr::null_mut(),
            result_container: ptr::null_mut(),
            slots: RefCell::new(self::slots::Slots::default()),
        };
        let inner = Rc::new(RefCell::new(InnerState::test_default()));
        let callbacks = Rc::new(RefCell::new(Callbacks::default()));
        let debounce = Debounce {
            handle: ptr::null_mut(),
            period_ms: 200,
        };
        let ctx = alloc::boxed::Box::new(trampolines::TrampolineCtx { sb: ptr::null() });
        SearchBar {
            bar,
            inner,
            callbacks,
            debounce: RefCell::new(debounce),
            _ctx: ctx,
            keyboard: RefCell::new(None),
            row_renderer: Rc::new(RefCell::new(None)),
            _pin: PhantomPinned,
            last_sync: core::cell::Cell::new(SlotVis::all_hidden()),
        }
    }

    #[cfg(test)]
    pub fn debug_slot_visibility(&self) -> SlotVis {
        self.last_sync.get()
    }
}

/// Attaches an `LV_EVENT_CLICKED` handler that invokes `f` whenever `card`
/// is tapped. The closure is boxed into LVGL `user_data` and reclaimed by an
/// `LV_EVENT_DELETE` handler when LVGL deletes the card.
///
/// Also flips `LV_OBJ_FLAG_CLICKABLE` on so plain `lv_obj_create`-style
/// containers receive click events (buttons get this flag for free).
///
/// # Safety
/// `card` must be a valid, non-null `lv_obj_t *` owned by LVGL for the
/// duration of any potential click dispatch.
pub unsafe fn install_card_click<F>(card: *mut lv_obj_t, f: F)
where
    F: FnMut() + 'static,
{
    use crate::c_bindings::{
        LV_EVENT_CLICKED, LV_EVENT_DELETE, lv_event_get_user_data, lv_obj_add_event_cb,
        lv_obj_add_flag,
    };
    type RowClick = alloc::boxed::Box<dyn FnMut()>;
    unsafe extern "C" fn trampoline(e: *mut crate::c_bindings::lv_event_t) {
        unsafe {
            let ud = lv_event_get_user_data(e);
            if ud.is_null() {
                return;
            }
            let cb = &mut *(ud as *mut RowClick);
            cb();
        }
    }
    unsafe extern "C" fn cleanup(e: *mut crate::c_bindings::lv_event_t) {
        unsafe {
            let ud = lv_event_get_user_data(e);
            if ud.is_null() {
                return;
            }
            drop(alloc::boxed::Box::from_raw(ud as *mut RowClick));
        }
    }
    // LV_OBJ_FLAG_CLICKABLE == (1 << 1) in LVGL v9 (lv_obj.h).
    const LV_OBJ_FLAG_CLICKABLE: u32 = 1 << 1;
    unsafe {
        lv_obj_add_flag(card, LV_OBJ_FLAG_CLICKABLE);
        let boxed: alloc::boxed::Box<RowClick> = alloc::boxed::Box::new(alloc::boxed::Box::new(f));
        let raw = alloc::boxed::Box::into_raw(boxed) as *mut core::ffi::c_void;
        lv_obj_add_event_cb(card, Some(trampoline), LV_EVENT_CLICKED, raw);
        lv_obj_add_event_cb(card, Some(cleanup), LV_EVENT_DELETE, raw);
    }
}

impl Drop for SearchBar {
    fn drop(&mut self) {
        if let Ok(mut s) = self.inner.try_borrow_mut() {
            s.snap.alive = false;
        }
        let sb_ptr: *const SearchBar = self as *const _;
        let ctx_ptr = self._ctx.as_mut() as *mut _;
        unsafe {
            trampolines::cancel_pending_result_scroll_end(ctx_ptr);
            let root_alive = !self.bar.root.is_null() && lv_obj_is_valid(self.bar.root);
            if root_alive {
                trampolines::unregister(sb_ptr, ctx_ptr);
            }
            self.debounce.borrow_mut().delete();
            if let Some(kb) = self.keyboard.borrow_mut().take() {
                if !kb.is_null() && lv_obj_is_valid(kb) {
                    crate::c_bindings::lv_keyboard_set_textarea(kb, core::ptr::null_mut());
                }
            }
            if root_alive {
                lv_obj_set_user_data(self.bar.text_area, core::ptr::null_mut());
                lv_obj_delete(self.bar.root);
            }
        }
    }
}

#[cfg(test)]
mod sb_tests {
    use super::*;
    use crate::c_bindings::{LvCall, SpyFixture, spy_drain};
    use core::ptr;
    use core::sync::atomic::{AtomicUsize, Ordering};

    fn build() -> core::pin::Pin<alloc::boxed::Box<SearchBar>> {
        unsafe { SearchBar::build(ptr::null_mut(), SearchBarConfig::default()) }
    }

    #[test]
    fn token_bumps_on_new_query_only() {
        let _fx = SpyFixture::new();
        let sb = build();
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
        let sb = build();
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
    fn query_callback_may_drop_owner_during_trampoline_dispatch() {
        use alloc::rc::Rc;
        use core::cell::RefCell;
        use core::pin::Pin;

        let _fx = SpyFixture::new();
        let holder: Rc<RefCell<Option<Pin<alloc::boxed::Box<SearchBar>>>>> =
            Rc::new(RefCell::new(None));
        let sb = build();
        let holder_for_cb = holder.clone();
        sb.on_query_changed(move |_t, _q| {
            holder_for_cb.borrow_mut().take();
        });
        holder.borrow_mut().replace(sb);
        let sb_ptr = {
            let borrowed = holder.borrow();
            borrowed.as_ref().unwrap().as_ref().get_ref() as *const SearchBar
        };

        unsafe { (&*sb_ptr).set_text("pizza") };

        assert!(holder.borrow().is_none());
    }

    #[test]
    fn stale_token_dropped() {
        let _fx = SpyFixture::new();
        let sb = build();
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
        let sb = build();
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
        let sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_query_changed(|_t, _q| {
            N.fetch_add(1, Ordering::SeqCst);
        });

        sb.set_text("pizza"); // fire #1
        sb.clear_query(); // resets last_fired_canonical
        sb.set_text("pizza"); // MUST fire again
        assert_eq!(N.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn set_loading_false_only_checks_token() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        // Mutate text without re-firing through the textarea API:
        sb.set_text("a"); // dedupe → token unchanged
        assert!(sb.set_loading(t, false));
    }

    #[test]
    fn empty_results_yields_no_results_state() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("xyzzy");
        let t = sb.current_token();
        assert!(sb.set_results(t, alloc::vec![]));
        assert_eq!(sb.state(), State::NoResults);
    }

    #[test]
    fn set_error_records_and_restores_pre_state() {
        let _fx = SpyFixture::new();
        let sb = build();
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

    #[test]
    fn selection_survives_compatible_set_results() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        let _ = sb.set_results(
            t,
            alloc::vec![SearchRow::new(1, "a"), SearchRow::new(2, "b")],
        );
        sb.select(1);
        sb.select(2);
        assert_eq!(sb.selected_count(), 2);
        // New result set drops id=2 silently.
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "a")]);
        assert_eq!(sb.selected_row_ids(), alloc::vec![1]);
    }

    #[test]
    fn load_more_triggers_on_low_scroll_bottom() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "x")]);
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_load_more(|_t, _p| {
            N.fetch_add(1, Ordering::SeqCst);
        });
        crate::c_bindings::set_next_scroll_bottom(10); // < threshold
        sb.check_scroll_for_load_more();
        assert_eq!(N.load(Ordering::SeqCst), 1);
        // Second check while pending — no extra fire.
        crate::c_bindings::set_next_scroll_bottom(5);
        sb.check_scroll_for_load_more();
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn scroll_end_async_trampoline_swallows_panicking_load_more_callback() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("a");
        let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "x")]);
        sb.on_load_more(|_t, _p| panic!("load more callback boom"));
        crate::c_bindings::set_next_scroll_bottom(10);

        crate::c_bindings::spy_emit_event(
            sb.bar.result_container,
            crate::c_bindings::LV_EVENT_SCROLL_END,
        );

        assert_eq!(sb.state(), State::Results);
    }

    #[test]
    fn clear_selection_emits_no_callbacks() {
        let _fx = SpyFixture::new();
        let sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_select(|_, _| {
            N.fetch_add(1, Ordering::SeqCst);
        });
        sb.set_text("a");
        let t = sb.current_token();
        let _ = sb.set_results(
            t,
            alloc::vec![SearchRow::new(1, "x"), SearchRow::new(2, "y")],
        );
        sb.select(1);
        sb.select(2);
        let pre = N.load(Ordering::SeqCst);
        sb.clear_selection();
        assert_eq!(N.load(Ordering::SeqCst), pre); // no extra fires
        assert_eq!(sb.selected_count(), 0);
    }

    #[test]
    fn textarea_event_kicks_debounce() {
        let _fx = SpyFixture::new();
        let sb = build();
        crate::c_bindings::spy_emit_event(
            sb.bar.text_area,
            crate::c_bindings::LV_EVENT_VALUE_CHANGED,
        );
        let resets = crate::c_bindings::SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| matches!(c, crate::c_bindings::LvCall::TimerReset { .. }))
                .count()
        });
        assert!(resets >= 1);
    }

    #[test]
    fn textarea_trampoline_returns_when_inner_is_already_borrowed() {
        let _fx = SpyFixture::new();
        let sb = build();
        let _borrow = sb.inner.borrow_mut();

        crate::c_bindings::spy_emit_event(
            sb.bar.text_area,
            crate::c_bindings::LV_EVENT_VALUE_CHANGED,
        );
    }

    #[test]
    fn textarea_trampoline_returns_when_debounce_is_already_borrowed() {
        let _fx = SpyFixture::new();
        let sb = build();
        let _borrow = sb.debounce.borrow_mut();

        crate::c_bindings::spy_emit_event(
            sb.bar.text_area,
            crate::c_bindings::LV_EVENT_VALUE_CHANGED,
        );
    }

    #[test]
    fn clear_button_emits_query_cleared() {
        let _fx = SpyFixture::new();
        let sb = build();
        static N: AtomicUsize = AtomicUsize::new(0);
        N.store(0, Ordering::SeqCst);
        sb.on_query_cleared(|| {
            N.fetch_add(1, Ordering::SeqCst);
        });
        sb.set_text("foo");
        crate::c_bindings::spy_emit_event(sb.bar.clear_button, crate::c_bindings::LV_EVENT_CLICKED);
        assert_eq!(N.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_unregisters_event_callbacks() {
        let _fx = SpyFixture::new();
        let sb = build();
        let ta = sb.bar.text_area;
        drop(sb);
        crate::c_bindings::spy_emit_event(ta, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
    }

    #[test]
    fn drop_cancels_async_clears_textarea_user_data_then_deletes_root() {
        let _fx = SpyFixture::new();
        let sb = build();
        let root = sb.bar.root as usize;
        let text_area = sb.bar.text_area as usize;
        let ctx = sb._ctx.as_ref() as *const trampolines::TrampolineCtx as usize;

        crate::c_bindings::spy_emit_event(
            sb.bar.result_container,
            crate::c_bindings::LV_EVENT_SCROLL_END,
        );
        let _ = spy_drain();

        drop(sb);
        let calls = spy_drain();

        let cancel = calls
            .iter()
            .position(|c| matches!(c, LvCall::AsyncCallCancel { user_data } if *user_data == ctx))
            .expect(
                "Drop must cancel pending SearchBar scroll-end async callback before freeing ctx",
            );
        let clear_user_data = calls
            .iter()
            .position(|c| matches!(c, LvCall::ObjSetUserData { obj, data } if *obj == text_area && *data == 0))
            .expect("Drop must clear textarea user_data before releasing InnerState Rc");
        let delete_root = calls
            .iter()
            .position(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == root))
            .expect("Drop must delete the SearchBar root LVGL subtree");

        assert!(
            cancel < clear_user_data && clear_user_data < delete_root,
            "expected async cancel before textarea user_data clear before root delete; calls: {calls:?}"
        );
    }

    #[test]
    fn drop_skips_child_lvgl_cleanup_when_root_was_already_deleted() {
        let _fx = SpyFixture::new();
        let sb = build();
        let root = sb.bar.root as usize;
        let text_area = sb.bar.text_area as usize;
        let ctx = sb._ctx.as_ref() as *const trampolines::TrampolineCtx as usize;

        unsafe {
            crate::c_bindings::lv_obj_delete(sb.bar.root);
        }
        let _ = spy_drain();

        drop(sb);
        let calls = spy_drain();

        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::AsyncCallCancel { user_data } if *user_data == ctx)),
            "Drop must still cancel pending async callbacks without touching LVGL child objects"
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::RemoveEventCbWithUserData { .. })),
            "Drop must not unregister callbacks through child pointers after root deletion; calls: {calls:?}"
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetUserData { obj, .. } if *obj == text_area)),
            "Drop must not clear user_data through a deleted textarea pointer; calls: {calls:?}"
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == root)),
            "Drop must not delete an already-deleted root; calls: {calls:?}"
        );
    }

    #[test]
    fn debounce_fire_invokes_tick() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("a");
        let _t0 = sb.current_token();
        sb.tick_debounce();
    }

    #[test]
    fn attach_detach_keyboard() {
        let _fx = SpyFixture::new();
        let sb = build();
        let kb = unsafe { crate::c_bindings::lv_keyboard_create(core::ptr::null_mut()) };
        sb.attach_keyboard(kb);
        sb.detach_keyboard();
        let sets = crate::c_bindings::SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| matches!(c, crate::c_bindings::LvCall::KeyboardSetTextarea { .. }))
                .count()
        });
        assert_eq!(sets, 2);
    }

    #[test]
    fn rows_rendered_with_recolor_and_highlight_markup() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("pizza");
        let t = sb.current_token();
        let _ = sb.set_results(t, alloc::vec![SearchRow::new(1, "Pizza Hut")]);

        // 1) lv_label_set_recolor(_, true) called at least once.
        let recolors = crate::c_bindings::SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| {
                    matches!(
                        c,
                        crate::c_bindings::LvCall::LabelSetRecolor { en: true, .. }
                    )
                })
                .count()
        });
        assert!(recolors >= 1);

        // 2) The label text contains the highlight escape "#FFAA00" and "Pizza".
        // LabelSetText carries text_bytes: Vec<u8> (NUL-terminated).
        let mut found = false;
        crate::c_bindings::SPY.with(|s| {
            for c in s.borrow().iter() {
                if let crate::c_bindings::LvCall::LabelSetText { text_bytes, .. } = c {
                    if let Ok(text) = core::str::from_utf8(text_bytes) {
                        if text.contains("#FFAA00") && text.contains("Pizza") {
                            found = true;
                        }
                    }
                }
            }
        });
        assert!(found, "highlighted label text not seen in spy");
    }

    #[test]
    fn render_rows_dispatches_to_custom_renderer() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(alloc::vec::Vec::<u64>::new()));
        let calls_c = calls.clone();
        let bar = SearchBar::test_stub();
        bar.set_row_renderer(move |_parent, row, _q, _sel| {
            calls_c.borrow_mut().push(row.id);
        });
        bar.with_inner_mut_for_test(|s| {
            s.rows = alloc::vec![SearchRow::new(1, "A"), SearchRow::new(2, "B")];
        });
        // set_row_renderer already triggered one render (with empty rows);
        // re-render now that rows are seeded.
        // (Direct private-method access is OK — same module.)
        bar.render_rows();
        assert_eq!(*calls.borrow(), alloc::vec![1u64, 2u64]);
    }

    #[test]
    fn no_renderer_means_no_dispatch() {
        let bar = SearchBar::test_stub();
        // Should not panic even with null result_container and no rows.
        bar.render_rows();
    }

    #[test]
    fn set_results_dispatches_to_custom_renderer() {
        let _fx = SpyFixture::new();
        let calls = std::rc::Rc::new(std::cell::RefCell::new(alloc::vec::Vec::<u64>::new()));
        let calls_c = calls.clone();
        let sb = build();
        sb.set_row_renderer(move |_parent, row, _q, _sel| {
            calls_c.borrow_mut().push(row.id);
        });
        // Drive through the production path: set query text to mint a token,
        // then install rows via the public set_results API.
        sb.set_text("ab");
        let t = sb.current_token();
        let accepted = sb.set_results(
            t,
            alloc::vec![SearchRow::new(1, "A"), SearchRow::new(2, "B"),],
        );
        assert!(accepted, "set_results should accept a current-token reply");
        // Renderer fired during set_row_renderer (empty rows) plus once per
        // row on the set_results-driven render. Filter to the rows we set.
        assert_eq!(*calls.borrow(), alloc::vec![1u64, 2u64]);
    }

    #[test]
    fn custom_renderer_may_drop_owner_during_set_results() {
        use alloc::rc::Rc;
        use core::cell::RefCell;
        use core::pin::Pin;

        let _fx = SpyFixture::new();
        let holder: Rc<RefCell<Option<Pin<alloc::boxed::Box<SearchBar>>>>> =
            Rc::new(RefCell::new(None));
        let sb = build();
        sb.set_text("pizza");
        let token = sb.current_token();
        let holder_for_renderer = holder.clone();
        sb.set_row_renderer(move |_parent, _row, _q, _sel| {
            holder_for_renderer.borrow_mut().take();
        });
        holder.borrow_mut().replace(sb);
        let sb_ptr = {
            let borrowed = holder.borrow();
            borrowed.as_ref().unwrap().as_ref().get_ref() as *const SearchBar
        };

        unsafe { (&*sb_ptr).set_results(token, alloc::vec![SearchRow::new(1, "Pizza")]) };

        assert!(holder.borrow().is_none());
    }

    struct DropProbe {
        drops: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    #[test]
    fn install_card_click_reclaims_closure_when_card_is_deleted() {
        let _fx = SpyFixture::new();
        let drops = std::rc::Rc::new(std::cell::Cell::new(0usize));
        let card = unsafe { crate::c_bindings::lv_obj_create(core::ptr::null_mut()) };
        let probe = DropProbe {
            drops: drops.clone(),
        };

        unsafe {
            install_card_click(card, move || {
                let _keep_probe_alive = &probe;
            });
        }

        assert_eq!(drops.get(), 0);
        crate::c_bindings::spy_emit_event(card, crate::c_bindings::LV_EVENT_DELETE);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn custom_card_click_closures_drop_on_rerender_and_searchbar_drop() {
        let _fx = SpyFixture::new();
        let drops = std::rc::Rc::new(std::cell::Cell::new(0usize));
        let drops_for_renderer = drops.clone();
        let mut sb = build();

        sb.set_row_renderer(move |parent, row, _q, _sel| {
            let card = unsafe { crate::c_bindings::lv_obj_create(parent) };
            let probe = DropProbe {
                drops: drops_for_renderer.clone(),
            };
            let row_id = row.id;
            unsafe {
                install_card_click(card, move || {
                    let _keep_probe_alive = &probe;
                    let _keep_row_id_alive = row_id;
                });
            }
        });

        sb.set_text("ab");
        let t = sb.current_token();
        assert!(sb.set_results(
            t,
            alloc::vec![SearchRow::new(1, "A"), SearchRow::new(2, "B")]
        ));
        assert_eq!(drops.get(), 0);

        assert!(sb.set_results(t, alloc::vec![SearchRow::new(3, "C")]));
        assert_eq!(
            drops.get(),
            2,
            "rerender must delete old cards and reclaim their click closures"
        );

        drop(sb);
        assert_eq!(
            drops.get(),
            3,
            "dropping SearchBar must delete remaining cards and reclaim closures"
        );
    }

    // ---- Task 8: selection mutators trigger re-render on dirty ----
    #[test]
    fn select_re_renders_via_custom_renderer() {
        let _fx = SpyFixture::new();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0u64));
        let sb = build();
        sb.set_text("ab");
        let t = sb.current_token();
        assert!(sb.set_results(t, alloc::vec![SearchRow::new(1, "A")]));
        let c = calls.clone();
        sb.set_row_renderer(move |_, _, _, _| {
            c.set(c.get() + 1);
        });
        let before = calls.get();
        sb.select(1);
        assert!(calls.get() > before, "select() should re-render");
    }

    #[test]
    fn select_noop_does_not_re_render() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("ab");
        let t = sb.current_token();
        assert!(sb.set_results(t, alloc::vec![SearchRow::new(1, "A")]));
        sb.select(1);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0u64));
        let c = calls.clone();
        sb.set_row_renderer(move |_, _, _, _| {
            c.set(c.get() + 1);
        });
        let before = calls.get();
        sb.select(1); // already selected → selection_dirty stays false → no re-render
        assert_eq!(
            calls.get(),
            before,
            "re-selecting same id must NOT re-render"
        );
    }

    #[test]
    fn deselect_toggle_clear_re_render() {
        let _fx = SpyFixture::new();
        let sb = build();
        sb.set_text("ab");
        let t = sb.current_token();
        assert!(sb.set_results(
            t,
            alloc::vec![SearchRow::new(1, "A"), SearchRow::new(2, "B"),]
        ));
        sb.select(1);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0u64));
        let c = calls.clone();
        sb.set_row_renderer(move |_, _, _, _| {
            c.set(c.get() + 1);
        });
        let n0 = calls.get();
        sb.deselect(1);
        assert!(calls.get() > n0, "deselect must re-render");
        let n1 = calls.get();
        sb.toggle_select(2);
        assert!(calls.get() > n1, "toggle_select must re-render");
        let n2 = calls.get();
        sb.clear_selection();
        assert!(calls.get() > n2, "clear_selection must re-render");
    }

    // ---- Task 6: sync_slot_visibility ----
    #[test]
    fn sync_visibility_empty_state() {
        let bar = SearchBar::test_stub();
        bar.sync_slot_visibility();
        let v = bar.debug_slot_visibility();
        assert_eq!(
            v,
            SlotVis {
                initial_empty: true,
                ..SlotVis::all_hidden()
            }
        );
    }

    #[test]
    fn sync_visibility_loading_with_rows() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.rows = alloc::vec![SearchRow::new(1, "A")];
            s.snap.state = State::Loading;
            s.snap.pending_load_more = true;
        });
        bar.sync_slot_visibility();
        let v = bar.debug_slot_visibility();
        assert!(v.footer_loading && !v.initial_loading);
        assert!(!v.initial_empty && !v.initial_error && !v.footer_error);
    }

    #[test]
    fn sync_visibility_loading_no_rows() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.snap.state = State::Loading;
        });
        bar.sync_slot_visibility();
        let v = bar.debug_slot_visibility();
        assert!(v.initial_loading && !v.footer_loading);
        assert!(!v.initial_empty);
    }

    #[test]
    fn sync_visibility_error_initial() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.snap.state = State::Error;
        });
        bar.sync_slot_visibility();
        let v = bar.debug_slot_visibility();
        assert!(v.initial_error && !v.footer_error);
    }

    #[test]
    fn sync_visibility_error_with_rows() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.rows = alloc::vec![SearchRow::new(1, "A")];
            s.snap.state = State::Error;
            s.snap.pending_load_more = true;
        });
        bar.sync_slot_visibility();
        let v = bar.debug_slot_visibility();
        assert!(v.footer_error && !v.initial_error);
    }

    #[test]
    fn sync_visibility_results_hides_all() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.rows = alloc::vec![SearchRow::new(1, "A")];
            s.snap.state = State::Results;
        });
        bar.sync_slot_visibility();
        assert_eq!(bar.debug_slot_visibility(), SlotVis::all_hidden());
    }

    #[test]
    fn sync_visibility_no_results_hides_all() {
        let bar = SearchBar::test_stub();
        bar.with_inner_mut_for_test(|s| {
            s.snap.state = State::NoResults;
        });
        bar.sync_slot_visibility();
        assert_eq!(bar.debug_slot_visibility(), SlotVis::all_hidden());
    }

    // ---- Task 6 fix: wiring — every public mutator calls sync_slot_visibility ----

    #[test]
    fn tick_debounce_invokes_sync() {
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        // last_sync starts all_hidden; default state is Empty — sync must
        // flip initial_empty to true.
        bar.tick_debounce();
        assert!(
            bar.debug_slot_visibility().initial_empty,
            "tick_debounce must call sync_slot_visibility"
        );
    }

    #[test]
    fn set_loading_invokes_sync() {
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        let token = bar.current_token();
        bar.set_loading(token, true);
        assert!(
            bar.debug_slot_visibility().initial_loading,
            "set_loading must call sync_slot_visibility"
        );
    }

    #[test]
    fn set_error_invokes_sync() {
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        let token = bar.current_token();
        bar.set_error(token, true);
        assert!(
            bar.debug_slot_visibility().initial_error,
            "set_error must call sync_slot_visibility"
        );
    }

    #[test]
    fn set_results_invokes_sync() {
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        let token = bar.current_token();
        bar.set_results(token, alloc::vec![SearchRow::new(1, "A")]);
        // Results state -> all hidden.
        assert_eq!(
            bar.debug_slot_visibility(),
            SlotVis::all_hidden(),
            "set_results must call sync_slot_visibility"
        );
    }

    // ---- Task 7: set_initial_empty_hint ----
    #[test]
    fn set_initial_empty_hint_invokes_builder_and_syncs() {
        use alloc::rc::Rc;
        use core::cell::Cell;
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        let invoked = Rc::new(Cell::new(0u32));
        let received_parent: Rc<Cell<*mut lv_obj_t>> = Rc::new(Cell::new(ptr::null_mut()));
        {
            let invoked = invoked.clone();
            let received_parent = received_parent.clone();
            bar.set_initial_empty_hint(move |parent| {
                invoked.set(invoked.get() + 1);
                received_parent.set(parent);
            });
        }
        assert_eq!(invoked.get(), 1, "builder must run exactly once");
        // On test_stub, root is null so the parent passed to builder is null.
        assert!(
            received_parent.get().is_null(),
            "test_stub has null root; builder must receive null parent"
        );
        // Stub default state is Empty -> sync sets initial_empty intent true.
        assert!(
            bar.debug_slot_visibility().initial_empty,
            "set_initial_empty_hint must trigger sync_slot_visibility"
        );
    }

    #[test]
    fn set_initial_empty_hint_can_be_called_twice() {
        use alloc::rc::Rc;
        use core::cell::Cell;
        let _fx = SpyFixture::new();
        let bar = SearchBar::test_stub();
        let count = Rc::new(Cell::new(0u32));
        for _ in 0..2 {
            let count = count.clone();
            bar.set_initial_empty_hint(move |_p| {
                count.set(count.get() + 1);
            });
        }
        assert_eq!(count.get(), 2);
    }

    /// Regression guardrail: `set_initial_empty_hint` MUST parent the slot to
    /// the SearchBar root, NOT `result_container`. Otherwise `render_rows`'s
    /// `lv_obj_clean(result_container)` would free the slot and leave a
    /// dangling pointer in `Slots::initial_empty`, which the next
    /// `show_initial_empty()` (via `sync_slot_visibility`) would deref.
    #[test]
    fn set_initial_empty_hint_does_not_parent_to_result_container() {
        let src = include_str!("mod.rs");
        let needle = "pub fn set_initial_empty_hint";
        let start = src.find(needle).expect("set_initial_empty_hint not found");
        let body_end = src[start..]
            .find("\n    /// Install (or replace) a custom row renderer")
            .map(|o| start + o)
            .unwrap_or(src.len());
        let body = &src[start..body_end];
        assert!(
            body.contains("ensure_initial_empty("),
            "set_initial_empty_hint must call ensure_initial_empty"
        );
        assert!(
            body.contains("lv_obj_clean("),
            "set_initial_empty_hint must clear prior slot children via lv_obj_clean"
        );
        // The whole point of this guardrail: the slot's parent must NOT be
        // `result_container` (which is wiped by `render_rows`).
        assert!(
            !body.contains("result_container"),
            "set_initial_empty_hint must NOT reference result_container — \
                 the slot would be freed by render_rows's lv_obj_clean"
        );
    }
}
