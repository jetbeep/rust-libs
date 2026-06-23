//! Trampolines: C-ABI fns LVGL invokes; each recovers `*mut SearchBar`
//! from the user_data we registered on the relevant object/timer,
//! checks `snap.alive`, and dispatches via the Model A pattern.
//! Risks #2, #19, #45, #47.
use crate::c_bindings::{
    LV_EVENT_CLICKED, LV_EVENT_SCROLL_END, LV_EVENT_VALUE_CHANGED, lv_event_get_user_data,
    lv_event_t, lv_obj_add_event_cb, lv_obj_remove_event_cb_with_user_data, lv_timer_get_user_data,
    lv_timer_t,
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
    if ud.is_null() {
        return None;
    }
    let ctx = unsafe { &mut *ud };
    if ctx.sb.is_null() {
        return None;
    }
    let sb = unsafe { &mut *ctx.sb };
    if !sb.inner.borrow().snap.alive {
        return None;
    }
    Some(sb)
}

unsafe fn sb_from_timer(t: *mut lv_timer_t) -> Option<&'static mut super::SearchBar> {
    let ud = unsafe { lv_timer_get_user_data(t) } as *mut TrampolineCtx;
    if ud.is_null() {
        return None;
    }
    let ctx = unsafe { &mut *ud };
    if ctx.sb.is_null() {
        return None;
    }
    let sb = unsafe { &mut *ctx.sb };
    if !sb.inner.borrow().snap.alive {
        return None;
    }
    Some(sb)
}

pub unsafe extern "C" fn on_textarea_value_changed(e: *mut lv_event_t) {
    let Some(sb) = (unsafe { sb_from_event(e) }) else {
        return;
    };
    unsafe {
        sb.debounce.kick();
    }
}

pub unsafe extern "C" fn on_debounce_fire(t: *mut lv_timer_t) {
    let Some(sb) = (unsafe { sb_from_timer(t) }) else {
        return;
    };
    sb.tick_debounce();
}

pub unsafe extern "C" fn on_clear_button_clicked(e: *mut lv_event_t) {
    let Some(sb) = (unsafe { sb_from_event(e) }) else {
        return;
    };
    sb.clear_query();
}

pub unsafe extern "C" fn on_result_scroll_end(e: *mut lv_event_t) {
    let Some(_sb) = (unsafe { sb_from_event(e) }) else {
        return;
    };
    // SCROLL_END is fired by LVGL from inside the refresh/animation walk
    // (e.g. during the screen-load animation that scrolls result_container
    // into its initial position). Calling `check_scroll_for_load_more`
    // synchronously would mutate widgets — `sync_slot_visibility` toggles
    // HIDDEN flags which call `lv_obj_invalidate` — and trip the LVGL
    // assert "Invalidate area is not allowed during rendering."
    //
    // Defer to the next `lv_timer_handler` tick via `lv_async_call`. The
    // user_data is the same TrampolineCtx we registered for the event,
    // so the deferred callback re-validates `snap.alive` before doing work.
    let ud = unsafe { lv_event_get_user_data(e) };
    unsafe {
        crate::c_bindings::lv_async_call(Some(on_result_scroll_end_async), ud);
    }
}

unsafe extern "C" fn on_result_scroll_end_async(ud: *mut core::ffi::c_void) {
    if ud.is_null() {
        return;
    }
    let ctx = unsafe { &*(ud as *mut TrampolineCtx) };
    if ctx.sb.is_null() {
        return;
    }
    let sb = unsafe { &mut *ctx.sb };
    if !sb.inner.borrow().snap.alive {
        return;
    }
    sb.check_scroll_for_load_more();
}

pub unsafe fn register(sb: *mut super::SearchBar, ctx: *mut TrampolineCtx) {
    let bar = unsafe { &(*sb).bar };
    unsafe {
        lv_obj_add_event_cb(
            bar.text_area,
            Some(on_textarea_value_changed),
            LV_EVENT_VALUE_CHANGED,
            ctx as *mut _,
        );
        lv_obj_add_event_cb(
            bar.clear_button,
            Some(on_clear_button_clicked),
            LV_EVENT_CLICKED,
            ctx as *mut _,
        );
        lv_obj_add_event_cb(
            bar.result_container,
            Some(on_result_scroll_end),
            LV_EVENT_SCROLL_END,
            ctx as *mut _,
        );
    }
}

pub unsafe fn unregister(sb: *mut super::SearchBar, ctx: *mut TrampolineCtx) {
    let bar = unsafe { &(*sb).bar };
    unsafe {
        lv_obj_remove_event_cb_with_user_data(
            bar.text_area,
            Some(on_textarea_value_changed),
            ctx as *mut _,
        );
        lv_obj_remove_event_cb_with_user_data(
            bar.clear_button,
            Some(on_clear_button_clicked),
            ctx as *mut _,
        );
        lv_obj_remove_event_cb_with_user_data(
            bar.result_container,
            Some(on_result_scroll_end),
            ctx as *mut _,
        );
    }
}
