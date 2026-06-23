//! Four optional slot containers (§2 ownership table).
use crate::c_bindings::{
    lv_obj_add_flag, lv_obj_create, lv_obj_remove_flag, lv_obj_set_pos, lv_obj_set_size, lv_obj_t,
    lv_pct,
};
use crate::lvgl::searchbar::bar::INPUT_ROW_HEIGHT;
use crate::lvgl::state::LvObjFlag;

/// # Ownership & lifetime (spec §2)
/// Holds raw pointers to LVGL child objects created under the SearchBar's
/// parent. `Slots` does NOT implement `Drop`: the LVGL parent owns its
/// children and frees them when the parent is deleted. The caller (the
/// SearchBar shell) MUST keep the parent alive for the lifetime of this
/// `Slots` instance and ensure `lv_obj_delete(parent)` runs exactly once
/// during teardown — which clears these child pointers transitively.
#[derive(Default)]
pub struct Slots {
    pub initial_loading: Option<*mut lv_obj_t>,
    pub initial_error: Option<*mut lv_obj_t>,
    pub initial_empty: Option<*mut lv_obj_t>,
    pub footer_loading: Option<*mut lv_obj_t>,
    pub footer_error: Option<*mut lv_obj_t>,
}

unsafe fn ensure(slot: &mut Option<*mut lv_obj_t>, parent: *mut lv_obj_t) -> *mut lv_obj_t {
    if let Some(p) = *slot {
        return p;
    }
    let p = unsafe { lv_obj_create(parent) };
    unsafe {
        // Slots overlay the result area but live as children of the bar
        // ROOT (so `lv_obj_clean(result_container)` in `render_rows` doesn't
        // free them). The root uses a column flex layout — without these
        // flags the slot would become a flex child that either gets
        // squashed to 0 px (sibling has flex_grow=1) or pushed off-screen.
        // IGNORE_LAYOUT excludes the slot from flex; FLOATING also keeps
        // it out of scroll/content size calculations on the parent.
        lv_obj_add_flag(p, LvObjFlag::IGNORE_LAYOUT.0 | LvObjFlag::FLOATING.0);
        // Position the slot to cover the result area only (below the input
        // row), so user-supplied content (e.g. an empty-state hint card)
        // doesn't overlay the text_area and intercept its clicks.
        lv_obj_set_pos(p, 0, INPUT_ROW_HEIGHT);
        lv_obj_set_size(p, lv_pct(100), lv_pct(100));
        lv_obj_add_flag(p, LvObjFlag::HIDDEN.0);
    }
    *slot = Some(p);
    p
}

/// # Safety
/// The pointer in `slot` (if `Some`) must be a valid, non-dangling LVGL
/// object pointer. Guaranteed when the pointer was produced by `ensure()`
/// with a valid parent and the parent has not been deleted (LVGL parents
/// own their children, so the pointer remains valid until parent delete).
unsafe fn show(slot: Option<*mut lv_obj_t>) {
    debug_assert!(
        slot.is_some(),
        "Slots::show called before ensure_*; FSM/visibility divergence (opus re-review F3)"
    );
    if let Some(p) = slot {
        unsafe {
            lv_obj_remove_flag(p, LvObjFlag::HIDDEN.0);
        }
    }
}
/// # Safety
/// Same contract as `show`: the pointer in `slot` (if `Some`) must be
/// a valid, non-dangling LVGL object pointer.
unsafe fn hide(slot: Option<*mut lv_obj_t>) {
    // Note: hide-when-None is a legitimate no-op (used by `hide_all` during
    // teardown / partial init); only `show` requires the slot to exist.
    if let Some(p) = slot {
        unsafe {
            lv_obj_add_flag(p, LvObjFlag::HIDDEN.0);
        }
    }
}

impl Slots {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    ///
    /// IMPORTANT: `parent` MUST NOT be the SearchBar's `result_container` —
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// the initial_loading slot and leave a dangling pointer in `Slots`.
    /// Use the SearchBar root container.
    pub unsafe fn ensure_initial_loading(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.initial_loading, parent) }
    }
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    ///
    /// IMPORTANT: `parent` MUST NOT be the SearchBar's `result_container` —
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// the initial_error slot and leave a dangling pointer in `Slots`.
    /// Use the SearchBar root container.
    pub unsafe fn ensure_initial_error(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.initial_error, parent) }
    }
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    ///
    /// IMPORTANT: `parent` MUST NOT be the SearchBar's `result_container` —
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// the initial_empty slot and leave a dangling pointer in `Slots`.
    /// Use the SearchBar root container.
    pub unsafe fn ensure_initial_empty(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.initial_empty, parent) }
    }
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    ///
    /// IMPORTANT: `parent` MUST NOT be the SearchBar's `result_container` —
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// any footer slot parented there and leave a dangling pointer in `Slots`.
    /// Use the SearchBar root or a sibling container.
    pub unsafe fn ensure_footer_loading(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.footer_loading, parent) }
    }
    /// # Safety
    /// `parent` must be a valid LVGL object pointer.
    ///
    /// IMPORTANT: `parent` MUST NOT be the SearchBar's `result_container` —
    /// `render_rows` calls `lv_obj_clean(result_container)`, which would free
    /// any footer slot parented there and leave a dangling pointer in `Slots`.
    /// Use the SearchBar root or a sibling container.
    pub unsafe fn ensure_footer_error(&mut self, parent: *mut lv_obj_t) -> *mut lv_obj_t {
        unsafe { ensure(&mut self.footer_error, parent) }
    }

    pub unsafe fn show_initial_loading(&self) {
        unsafe {
            show(self.initial_loading);
        }
    }
    pub unsafe fn hide_initial_loading(&self) {
        unsafe {
            hide(self.initial_loading);
        }
    }
    pub unsafe fn show_initial_error(&self) {
        unsafe {
            show(self.initial_error);
        }
    }
    pub unsafe fn hide_initial_error(&self) {
        unsafe {
            hide(self.initial_error);
        }
    }
    pub unsafe fn show_initial_empty(&self) {
        unsafe {
            show(self.initial_empty);
        }
    }
    pub unsafe fn hide_initial_empty(&self) {
        unsafe {
            hide(self.initial_empty);
        }
    }
    pub unsafe fn show_footer_loading(&self) {
        unsafe {
            show(self.footer_loading);
        }
    }
    pub unsafe fn hide_footer_loading(&self) {
        unsafe {
            hide(self.footer_loading);
        }
    }
    pub unsafe fn show_footer_error(&self) {
        unsafe {
            show(self.footer_error);
        }
    }
    pub unsafe fn hide_footer_error(&self) {
        unsafe {
            hide(self.footer_error);
        }
    }

    /// Hide every slot — used when phase transitions invalidate them.
    pub unsafe fn hide_all(&self) {
        unsafe {
            hide(self.initial_loading);
            hide(self.initial_error);
            hide(self.initial_empty);
            hide(self.footer_loading);
            hide(self.footer_error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, lv_obj_create, reset_obj_pool, spy_drain};
    use core::ptr;

    fn count_flag_calls() -> (usize, usize) {
        let calls = spy_drain();
        let add = calls
            .iter()
            .filter(|c| matches!(c, LvCall::AddFlag { .. }))
            .count();
        let rem = calls
            .iter()
            .filter(|c| matches!(c, LvCall::RemoveFlag { .. }))
            .count();
        (add, rem)
    }

    #[test]
    fn slots_lazy_create_and_start_hidden() {
        reset_obj_pool();
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
        reset_obj_pool();
        let mut sl = Slots::default();
        let parent = unsafe { lv_obj_create(ptr::null_mut()) };
        unsafe {
            sl.ensure_footer_loading(parent);
        }
        unsafe {
            sl.show_footer_loading();
        }
        unsafe {
            sl.hide_footer_loading();
        }
        let (add, rem) = count_flag_calls();
        assert!(add >= 2); // ensure() adds HIDDEN, hide() adds HIDDEN
        assert_eq!(rem, 1); // show() removes HIDDEN exactly once
    }

    #[test]
    fn hide_all_no_panic_on_empty_slots() {
        reset_obj_pool();
        let sl = Slots::default();
        unsafe {
            sl.hide_all();
        }
    }

    #[test]
    fn initial_empty_starts_hidden_and_lazy() {
        let s = Slots::default();
        assert!(s.initial_empty.is_none());
    }
}
