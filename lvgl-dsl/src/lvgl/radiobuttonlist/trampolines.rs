use crate::c_bindings;

#[cfg(any(test, no_zephyr))]
fn run_trampoline<F: FnOnce()>(f: F) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
}

#[cfg(not(any(test, no_zephyr)))]
fn run_trampoline<F: FnOnce()>(f: F) {
    f();
}

pub(crate) struct RowCtx {
    pub inner: alloc::rc::Rc<core::cell::RefCell<super::RadioButtonListInner>>,
    pub index: usize,
}

unsafe extern "C" fn on_row_clicked(e: *mut c_bindings::lv_event_t) {
    let user_data = unsafe { c_bindings::lv_event_get_user_data(e) } as *mut RowCtx;
    if user_data.is_null() {
        return;
    }
    let (inner, index) = {
        let ctx = unsafe { &*user_data };
        (ctx.inner.clone(), ctx.index)
    };
    run_trampoline(|| super::RadioButtonList::handle_row_clicked(&inner, index));
}

pub(crate) unsafe fn register_row(row: *mut c_bindings::lv_obj_t, ctx: *mut RowCtx) {
    unsafe {
        c_bindings::lv_obj_add_event_cb(
            row,
            Some(on_row_clicked),
            c_bindings::LV_EVENT_CLICKED,
            ctx as *mut core::ffi::c_void,
        );
    }
}

pub(crate) unsafe fn unregister_row(row: *mut c_bindings::lv_obj_t, ctx: *mut RowCtx) {
    unsafe {
        c_bindings::lv_obj_remove_event_cb_with_user_data(
            row,
            Some(on_row_clicked),
            ctx as *mut core::ffi::c_void,
        );
    }
}
