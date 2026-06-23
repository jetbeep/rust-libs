use crate::c_bindings;

pub(crate) struct RowCtx {
    pub list: *mut super::RadioButtonList,
    pub index: usize,
}

unsafe extern "C" fn on_row_clicked(e: *mut c_bindings::lv_event_t) {
    let user_data = unsafe { c_bindings::lv_event_get_user_data(e) } as *mut RowCtx;
    if user_data.is_null() { return; }
    let ctx = unsafe { &mut *user_data };
    if ctx.list.is_null() { return; }
    let list = unsafe { &mut *ctx.list };
    list.handle_row_clicked(ctx.index);
}

pub(crate) unsafe fn register_row(
    row: *mut c_bindings::lv_obj_t,
    ctx: *mut RowCtx,
) {
    unsafe {
        c_bindings::lv_obj_add_event_cb(
            row,
            Some(on_row_clicked),
            c_bindings::LV_EVENT_CLICKED,
            ctx as *mut core::ffi::c_void,
        );
    }
}

pub(crate) unsafe fn unregister_row(
    row: *mut c_bindings::lv_obj_t,
    ctx: *mut RowCtx,
) {
    unsafe {
        c_bindings::lv_obj_remove_event_cb_with_user_data(
            row,
            Some(on_row_clicked),
            ctx as *mut core::ffi::c_void,
        );
    }
}
