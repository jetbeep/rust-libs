//! LVGL wrappers for desktop — mirrors the Zephyr jetbeep_core::lvgl API.
//!
//! These call real LVGL functions via FFI. The C side (main.c) initializes
//! LVGL + SDL display before app_main() is called, so the display is ready.

use std::ffi::c_char;
use std::ffi::c_void;

// Opaque LVGL types — we only hold pointers
#[repr(C)]
pub struct lv_obj_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_font_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_display_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_indev_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_event_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_event_dsc_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct lv_group_t {
    _opaque: [u8; 0],
}

/// LVGL color — BGR byte order
#[repr(C)]
#[derive(Copy, Clone)]
pub struct lv_color_t {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
}

#[allow(non_camel_case_types)]
pub type lv_event_cb_t = unsafe extern "C" fn(*mut lv_event_t);

mod ffi {
    use super::*;

    unsafe extern "C" {
        // Screen / object basics
        pub fn lv_screen_active() -> *mut lv_obj_t;
        pub fn lv_obj_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_obj_align(obj: *mut lv_obj_t, align: u32, x_ofs: i32, y_ofs: i32);
        pub fn lv_obj_delete(obj: *mut lv_obj_t);

        // Size / position
        pub fn lv_obj_set_size(obj: *mut lv_obj_t, w: i32, h: i32);
        pub fn lv_obj_set_width(obj: *mut lv_obj_t, w: i32);
        pub fn lv_obj_set_height(obj: *mut lv_obj_t, h: i32);
        pub fn lv_obj_set_pos(obj: *mut lv_obj_t, x: i32, y: i32);

        // Flex layout
        pub fn lv_obj_set_flex_flow(obj: *mut lv_obj_t, flow: u32);
        pub fn lv_obj_set_flex_align(obj: *mut lv_obj_t, main_place: u32, cross_place: u32, track_cross_place: u32);
        pub fn lv_obj_set_flex_grow(obj: *mut lv_obj_t, grow: u8);

        // Flags & state
        pub fn lv_obj_add_flag(obj: *mut lv_obj_t, f: u32);
        pub fn lv_obj_remove_flag(obj: *mut lv_obj_t, f: u32);
        pub fn lv_obj_add_state(obj: *mut lv_obj_t, state: u32);
        pub fn lv_obj_remove_state(obj: *mut lv_obj_t, state: u32);

        // Styles — background
        pub fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, value: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, value: u8, selector: u32);

        // Styles — border
        pub fn lv_obj_set_style_border_color(obj: *mut lv_obj_t, value: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_border_width(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Styles — text
        pub fn lv_obj_set_style_text_font(obj: *mut lv_obj_t, font: *const lv_font_t, selector: u32);
        pub fn lv_obj_set_style_text_color(obj: *mut lv_obj_t, value: lv_color_t, selector: u32);

        // Styles — padding
        pub fn lv_obj_set_style_pad_top(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_bottom(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_left(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_right(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_row(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_column(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Styles — radius
        pub fn lv_obj_set_style_radius(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Widgets — button / label
        pub fn lv_button_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_label_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_label_set_text(obj: *mut lv_obj_t, text: *const c_char);

        // Widgets — textarea
        pub fn lv_textarea_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_textarea_set_text(obj: *mut lv_obj_t, txt: *const c_char);
        pub fn lv_textarea_add_text(obj: *mut lv_obj_t, txt: *const c_char);
        pub fn lv_textarea_get_text(obj: *const lv_obj_t) -> *const c_char;
        pub fn lv_textarea_set_placeholder_text(obj: *mut lv_obj_t, txt: *const c_char);
        pub fn lv_textarea_set_one_line(obj: *mut lv_obj_t, en: bool);

        // Events
        pub fn lv_obj_add_event_cb(obj: *mut lv_obj_t, event_cb: lv_event_cb_t, filter: u32, user_data: *mut c_void) -> *mut lv_event_dsc_t;
        pub fn lv_event_get_target_obj(e: *mut lv_event_t) -> *mut lv_obj_t;
        pub fn lv_event_get_code(e: *mut lv_event_t) -> u32;
        pub fn lv_event_get_user_data(e: *mut lv_event_t) -> *mut c_void;

        // Colors
        pub fn lv_color_hex(c: u32) -> lv_color_t;

        // Display management
        pub fn lv_sdl_window_create(hor_res: i32, ver_res: i32) -> *mut lv_display_t;
        pub fn lv_sdl_window_set_title(disp: *mut lv_display_t, title: *const c_char);
        pub fn lv_display_set_resolution(disp: *mut lv_display_t, hor_res: i32, ver_res: i32);
        pub fn lv_display_set_default(disp: *mut lv_display_t);
        pub fn lv_display_get_default() -> *mut lv_display_t;

        // Input devices
        pub fn lv_sdl_mouse_create() -> *mut lv_indev_t;
        pub fn lv_sdl_keyboard_create() -> *mut lv_indev_t;
        pub fn lv_indev_get_next(indev: *mut lv_indev_t) -> *mut lv_indev_t;
        pub fn lv_indev_get_type(indev: *const lv_indev_t) -> u32;
        pub fn lv_indev_get_display(indev: *const lv_indev_t) -> *mut lv_display_t;
        pub fn lv_indev_set_display(indev: *mut lv_indev_t, disp: *mut lv_display_t);
        pub fn lv_indev_set_group(indev: *mut lv_indev_t, group: *mut lv_group_t);

        // Groups
        pub fn lv_group_create() -> *mut lv_group_t;
        pub fn lv_group_add_obj(group: *mut lv_group_t, obj: *mut lv_obj_t);
        pub fn lv_group_set_default(group: *mut lv_group_t);
        pub fn lv_group_focus_obj(obj: *mut lv_obj_t);

        // Fonts
        pub static lv_font_montserrat_14: lv_font_t;
        pub static lv_font_montserrat_30: lv_font_t;

        // Dropdown
        pub fn lv_dropdown_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_dropdown_set_options(obj: *mut lv_obj_t, options: *const c_char);
        pub fn lv_dropdown_set_selected(obj: *mut lv_obj_t, sel_opt: u32);
        pub fn lv_dropdown_get_selected(obj: *const lv_obj_t) -> u32;
        pub fn lv_dropdown_get_selected_str(obj: *const lv_obj_t, buf: *mut c_char, buf_size: u32);

        // Msgbox (LVGL v9 API)
        pub fn lv_msgbox_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_msgbox_add_title(mbox: *mut lv_obj_t, title: *const c_char) -> *mut lv_obj_t;
        pub fn lv_msgbox_add_text(mbox: *mut lv_obj_t, text: *const c_char) -> *mut lv_obj_t;
        pub fn lv_msgbox_add_close_button(mbox: *mut lv_obj_t) -> *mut lv_obj_t;

        // Generic helpers
        pub fn lv_obj_clean(obj: *mut lv_obj_t);

        // Scrolling
        pub fn lv_obj_set_scroll_dir(obj: *mut lv_obj_t, dir: u32);
        pub fn lv_obj_scroll_to_view(obj: *mut lv_obj_t, anim_en: u32);
    }
}

// ── Safe wrappers ──────────────────────────────────────────────────────

pub struct LvObj {
    pub(crate) obj: *mut lv_obj_t,
}

impl LvObj {
    pub fn as_raw(&self) -> *mut lv_obj_t {
        self.obj
    }

    pub unsafe fn from_raw(obj: *mut lv_obj_t) -> Self {
        Self { obj }
    }
}

pub struct LvFont {
    font: *const lv_font_t,
}

pub struct LvDisplay {
    pub(crate) disp: *mut lv_display_t,
}

pub struct LvIndev {
    indev: *mut lv_indev_t,
}

// Constants
pub const LV_OBJ_FLAG_SCROLLABLE: u32 = 1 << 4;
pub const LV_OBJ_FLAG_CLICKABLE: u32 = 1 << 1;
pub const LV_OBJ_FLAG_CLICK_FOCUSABLE: u32 = 1 << 2;

pub const LV_STATE_DISABLED: u32 = 1 << 9;

pub const LV_EVENT_CLICKED: u32 = 10; // from lv_event.h enum
pub const LV_EVENT_VALUE_CHANGED: u32 = 35;

pub const LV_FLEX_FLOW_ROW: u32 = 0x00;
pub const LV_FLEX_FLOW_ROW_WRAP: u32 = 0x04;
pub const LV_FLEX_FLOW_COLUMN: u32 = 0x01;

pub const LV_FLEX_ALIGN_START: u32 = 0;
pub const LV_FLEX_ALIGN_CENTER: u32 = 2;

pub const LV_OPA_COVER: u8 = 255;

pub const LV_ANIM_OFF: u32 = 0;
pub const LV_ANIM_ON: u32 = 1;

pub const LV_INDEV_TYPE_KEYPAD: u32 = 2;

pub const LV_DIR_LEFT: u32 = 1 << 0;
pub const LV_DIR_RIGHT: u32 = 1 << 1;
pub const LV_DIR_TOP: u32 = 1 << 2;
pub const LV_DIR_BOTTOM: u32 = 1 << 3;
pub const LV_DIR_HOR: u32 = LV_DIR_LEFT | LV_DIR_RIGHT;
#[allow(dead_code)]
pub const LV_DIR_VER: u32 = LV_DIR_TOP | LV_DIR_BOTTOM;
#[allow(dead_code)]
pub const LV_DIR_ALL: u32 = 0x0F;

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum LvAlign {
    Default = 0,
    TopLeft,
    TopMid,
    TopRight,
    BottomLeft,
    BottomMid,
    BottomRight,
    LeftMid,
    RightMid,
    Center,
    OutTopLeft,
    OutTopMid,
    OutTopRight,
    OutBottomLeft,
    OutBottomMid,
    OutBottomRight,
    OutLeftTop,
    OutLeftMid,
    OutLeftBottom,
    OutRightTop,
    OutRightMid,
    OutRightBottom,
}

// ── Color helpers ──

pub fn lv_color_hex_fn(c: u32) -> lv_color_t {
    unsafe { ffi::lv_color_hex(c) }
}

// ── Display management ──

pub fn lv_sdl_window_create(hor_res: i32, ver_res: i32) -> LvDisplay {
    let disp = unsafe { ffi::lv_sdl_window_create(hor_res, ver_res) };
    if disp.is_null() {
        panic!("Failed to create SDL window");
    }
    LvDisplay { disp }
}

pub fn lv_sdl_window_set_title(disp: &LvDisplay, title: &str) {
    let c_str = to_null_terminated(title);
    unsafe { ffi::lv_sdl_window_set_title(disp.disp, c_str.as_ptr() as *const c_char) }
}

pub fn lv_sdl_window_set_resolution(disp: &LvDisplay, hor_res: i32, ver_res: i32) {
    unsafe { ffi::lv_display_set_resolution(disp.disp, hor_res, ver_res) }
}

// ── SDL monitor query ──
//
// Used by callers that want to clamp their window to a fraction of the
// physical monitor. We bind directly to SDL2 rather than going through LVGL
// because the SDL driver doesn't expose monitor dimensions.

#[repr(C)]
struct SdlDisplayMode {
    format: u32,
    w: i32,
    h: i32,
    refresh_rate: i32,
    driverdata: *mut c_void,
}

unsafe extern "C" {
    fn SDL_GetDesktopDisplayMode(display_index: i32, mode: *mut SdlDisplayMode) -> i32;
    fn SDL_GetClipboardText() -> *mut c_char;
    fn SDL_SetClipboardText(text: *const c_char) -> i32;
    fn SDL_free(mem: *mut c_void);
}

/// Read the OS clipboard as a UTF-8 string (empty if unavailable).
pub fn sdl_get_clipboard_text() -> String {
    let ptr = unsafe { SDL_GetClipboardText() };
    if ptr.is_null() {
        return String::new();
    }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { SDL_free(ptr as *mut c_void) };
    s
}

/// Write a string to the OS clipboard.
pub fn sdl_set_clipboard_text(text: &str) {
    if let Ok(c_str) = std::ffi::CString::new(text) {
        unsafe { SDL_SetClipboardText(c_str.as_ptr()) };
    }
}

/// Return the desktop resolution of display 0, or `None` if SDL can't
/// report it (e.g. before SDL_Init or on a headless host).
pub fn sdl_monitor_size() -> Option<(i32, i32)> {
    let mut mode = SdlDisplayMode {
        format: 0,
        w: 0,
        h: 0,
        refresh_rate: 0,
        driverdata: std::ptr::null_mut(),
    };
    let rc = unsafe { SDL_GetDesktopDisplayMode(0, &mut mode) };
    if rc == 0 && mode.w > 0 && mode.h > 0 {
        Some((mode.w, mode.h))
    } else {
        None
    }
}

pub fn lv_display_set_default(disp: &LvDisplay) {
    unsafe { ffi::lv_display_set_default(disp.disp) }
}

pub fn lv_display_get_default() -> LvDisplay {
    LvDisplay { disp: unsafe { ffi::lv_display_get_default() } }
}

// ── Input devices ──

pub fn lv_sdl_mouse_create() -> LvIndev {
    LvIndev { indev: unsafe { ffi::lv_sdl_mouse_create() } }
}

pub fn lv_sdl_keyboard_create() -> LvIndev {
    LvIndev { indev: unsafe { ffi::lv_sdl_keyboard_create() } }
}

pub fn lv_indev_set_display(indev: &LvIndev, disp: &LvDisplay) {
    unsafe { ffi::lv_indev_set_display(indev.indev, disp.disp) }
}

pub fn lv_indev_set_group(indev: &LvIndev, group: &LvGroup) {
    unsafe { ffi::lv_indev_set_group(indev.indev, group.group) }
}

pub fn lv_bind_group_to_display_keypads(display: &LvDisplay, group: &LvGroup) {
    let mut indev = unsafe { ffi::lv_indev_get_next(std::ptr::null_mut()) };
    while !indev.is_null() {
        let indev_type = unsafe { ffi::lv_indev_get_type(indev) };
        let indev_display = unsafe { ffi::lv_indev_get_display(indev) };
        if indev_type == LV_INDEV_TYPE_KEYPAD && indev_display == display.disp {
            unsafe { ffi::lv_indev_set_group(indev, group.group) };
        }
        indev = unsafe { ffi::lv_indev_get_next(indev) };
    }
}

// ── Groups ──

pub struct LvGroup {
    group: *mut lv_group_t,
}

impl LvGroup {
    /// Borrow the raw group pointer (e.g. to stash it and rebuild a handle
    /// later via [`LvGroup::from_raw`]).
    pub fn raw(&self) -> *mut lv_group_t {
        self.group
    }

    /// Wrap an existing raw group pointer. The caller must ensure the group
    /// outlives the returned handle; `LvGroup` does not own/free the group.
    pub fn from_raw(group: *mut lv_group_t) -> LvGroup {
        LvGroup { group }
    }
}

pub fn lv_group_create() -> LvGroup {
    let group = unsafe { ffi::lv_group_create() };
    if group.is_null() {
        panic!("Failed to create group");
    }
    LvGroup { group }
}

pub fn lv_group_add_obj(group: &LvGroup, obj: &LvObj) {
    unsafe { ffi::lv_group_add_obj(group.group, obj.obj) }
}

pub fn lv_group_set_default(group: &LvGroup) {
    unsafe { ffi::lv_group_set_default(group.group) }
}

/// Focus `obj` within its group so keyboard input is routed to it.
pub fn lv_group_focus_obj(obj: &LvObj) {
    unsafe { ffi::lv_group_focus_obj(obj.obj) }
}

// ── Object basics ──

pub fn lv_screen_active() -> LvObj {
    let obj = unsafe { ffi::lv_screen_active() };
    if obj.is_null() {
        panic!("Failed to get active screen");
    }
    LvObj { obj }
}

pub fn lv_obj_create(parent: &LvObj) -> LvObj {
    let obj = unsafe { ffi::lv_obj_create(parent.obj) };
    if obj.is_null() {
        panic!("Failed to create object");
    }
    LvObj { obj }
}

pub fn lv_obj_align(obj: &LvObj, align: LvAlign, x_ofs: i32, y_ofs: i32) {
    unsafe { ffi::lv_obj_align(obj.obj, align as u32, x_ofs, y_ofs) }
}

pub fn lv_obj_delete(obj: LvObj) {
    unsafe { ffi::lv_obj_delete(obj.obj) }
}

// ── Size / position ──

pub fn lv_obj_set_size(obj: &LvObj, w: i32, h: i32) {
    unsafe { ffi::lv_obj_set_size(obj.obj, w, h) }
}

pub fn lv_obj_set_width(obj: &LvObj, w: i32) {
    unsafe { ffi::lv_obj_set_width(obj.obj, w) }
}

pub fn lv_obj_set_height(obj: &LvObj, h: i32) {
    unsafe { ffi::lv_obj_set_height(obj.obj, h) }
}

pub fn lv_obj_set_pos(obj: &LvObj, x: i32, y: i32) {
    unsafe { ffi::lv_obj_set_pos(obj.obj, x, y) }
}

// ── Flex layout ──

pub fn lv_obj_set_flex_flow(obj: &LvObj, flow: u32) {
    unsafe { ffi::lv_obj_set_flex_flow(obj.obj, flow) }
}

pub fn lv_obj_set_flex_align(obj: &LvObj, main_place: u32, cross_place: u32, track_cross_place: u32) {
    unsafe { ffi::lv_obj_set_flex_align(obj.obj, main_place, cross_place, track_cross_place) }
}

pub fn lv_obj_set_flex_grow(obj: &LvObj, grow: u8) {
    unsafe { ffi::lv_obj_set_flex_grow(obj.obj, grow) }
}

// ── Flags / state ──

pub fn lv_obj_add_flag(obj: &LvObj, f: u32) {
    unsafe { ffi::lv_obj_add_flag(obj.obj, f) }
}

pub fn lv_obj_remove_flag(obj: &LvObj, f: u32) {
    unsafe { ffi::lv_obj_remove_flag(obj.obj, f) }
}

pub fn lv_obj_add_state(obj: &LvObj, state: u32) {
    unsafe { ffi::lv_obj_add_state(obj.obj, state) }
}

pub fn lv_obj_remove_state(obj: &LvObj, state: u32) {
    unsafe { ffi::lv_obj_remove_state(obj.obj, state) }
}

// ── Styles ──

pub fn lv_obj_set_style_bg_color(obj: &LvObj, color: lv_color_t, selector: u32) {
    unsafe { ffi::lv_obj_set_style_bg_color(obj.obj, color, selector) }
}

pub fn lv_obj_set_style_bg_opa(obj: &LvObj, opa: u8, selector: u32) {
    unsafe { ffi::lv_obj_set_style_bg_opa(obj.obj, opa, selector) }
}

pub fn lv_obj_set_style_border_color(obj: &LvObj, color: lv_color_t, selector: u32) {
    unsafe { ffi::lv_obj_set_style_border_color(obj.obj, color, selector) }
}

pub fn lv_obj_set_style_border_width(obj: &LvObj, width: i32, selector: u32) {
    unsafe { ffi::lv_obj_set_style_border_width(obj.obj, width, selector) }
}

pub fn lv_obj_set_style_text_font(obj: &LvObj, font: &LvFont, selector: u32) {
    unsafe { ffi::lv_obj_set_style_text_font(obj.obj, font.font, selector) }
}

pub fn lv_obj_set_style_text_color(obj: &LvObj, color: lv_color_t, selector: u32) {
    unsafe { ffi::lv_obj_set_style_text_color(obj.obj, color, selector) }
}

pub fn lv_obj_set_style_pad_all(obj: &LvObj, value: i32, selector: u32) {
    unsafe {
        ffi::lv_obj_set_style_pad_top(obj.obj, value, selector);
        ffi::lv_obj_set_style_pad_bottom(obj.obj, value, selector);
        ffi::lv_obj_set_style_pad_left(obj.obj, value, selector);
        ffi::lv_obj_set_style_pad_right(obj.obj, value, selector);
    }
}

pub fn lv_obj_set_style_pad_row(obj: &LvObj, value: i32, selector: u32) {
    unsafe { ffi::lv_obj_set_style_pad_row(obj.obj, value, selector) }
}

pub fn lv_obj_set_style_pad_column(obj: &LvObj, value: i32, selector: u32) {
    unsafe { ffi::lv_obj_set_style_pad_column(obj.obj, value, selector) }
}

pub fn lv_obj_set_style_radius(obj: &LvObj, value: i32, selector: u32) {
    unsafe { ffi::lv_obj_set_style_radius(obj.obj, value, selector) }
}

// ── Widgets ──

pub fn lv_button_create(parent: &LvObj) -> LvObj {
    let obj = unsafe { ffi::lv_button_create(parent.obj) };
    if obj.is_null() {
        panic!("Failed to create button");
    }
    LvObj { obj }
}

pub fn lv_label_create(parent: &LvObj) -> LvObj {
    let obj = unsafe { ffi::lv_label_create(parent.obj) };
    if obj.is_null() {
        panic!("Failed to create label");
    }
    LvObj { obj }
}

pub fn lv_textarea_create(parent: &LvObj) -> LvObj {
    let obj = unsafe { ffi::lv_textarea_create(parent.obj) };
    if obj.is_null() {
        panic!("Failed to create textarea");
    }
    LvObj { obj }
}

pub fn lv_textarea_set_text(obj: &LvObj, text: &str) {
    let c_str = to_null_terminated(text);
    unsafe { ffi::lv_textarea_set_text(obj.obj, c_str.as_ptr() as *const c_char) }
}

pub fn lv_textarea_add_text(obj: &LvObj, text: &str) {
    let c_str = to_null_terminated(text);
    unsafe { ffi::lv_textarea_add_text(obj.obj, c_str.as_ptr() as *const c_char) }
}

pub fn lv_textarea_get_text(obj: &LvObj) -> String {
    let ptr = unsafe { ffi::lv_textarea_get_text(obj.obj) };
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        std::ffi::CStr::from_ptr(ptr)
            .to_string_lossy()
            .into_owned()
    }
}

pub fn lv_textarea_set_placeholder_text(obj: &LvObj, text: &str) {
    let c_str = to_null_terminated(text);
    unsafe { ffi::lv_textarea_set_placeholder_text(obj.obj, c_str.as_ptr() as *const c_char) }
}

pub fn lv_textarea_set_one_line(obj: &LvObj, en: bool) {
    unsafe { ffi::lv_textarea_set_one_line(obj.obj, en) }
}

// ── Events ──

pub fn lv_obj_add_event_cb(obj: &LvObj, cb: lv_event_cb_t, filter: u32, user_data: *mut c_void) {
    unsafe { ffi::lv_obj_add_event_cb(obj.obj, cb, filter, user_data) };
}

pub fn lv_event_get_target_obj(e: *mut lv_event_t) -> LvObj {
    LvObj { obj: unsafe { ffi::lv_event_get_target_obj(e) } }
}

pub fn lv_event_get_code(e: *mut lv_event_t) -> u32 {
    unsafe { ffi::lv_event_get_code(e) }
}

pub fn lv_event_get_user_data(e: *mut lv_event_t) -> *mut c_void {
    unsafe { ffi::lv_event_get_user_data(e) }
}

fn to_null_terminated(text: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(text.len() + 1);
    buf.extend_from_slice(text.as_bytes());
    buf.push(0);
    buf
}

pub fn lv_label_set_text(obj: &LvObj, text: &str) {
    let c_string = to_null_terminated(text);
    unsafe {
        ffi::lv_label_set_text(obj.obj, c_string.as_ptr() as *const c_char);
    }
}

pub fn lv_font_montserrat_14() -> LvFont {
    LvFont {
        font: unsafe { &ffi::lv_font_montserrat_14 as *const lv_font_t },
    }
}

pub fn lv_font_montserrat_30() -> LvFont {
    LvFont {
        font: unsafe { &ffi::lv_font_montserrat_30 as *const lv_font_t },
    }
}

// ── Dropdown ──

pub fn lv_dropdown_create_obj(parent: &LvObj) -> LvObj {
    let obj = unsafe { ffi::lv_dropdown_create(parent.obj) };
    if obj.is_null() {
        panic!("Failed to create dropdown");
    }
    LvObj { obj }
}

pub fn lv_dropdown_set_options_str(dd: &LvObj, options: &str) {
    let c = std::ffi::CString::new(options).expect("dropdown options contains NUL");
    unsafe { ffi::lv_dropdown_set_options(dd.obj, c.as_ptr()) };
}

pub fn lv_dropdown_set_selected_idx(dd: &LvObj, idx: u32) {
    unsafe { ffi::lv_dropdown_set_selected(dd.obj, idx) };
}

pub fn lv_dropdown_get_selected_idx(dd: &LvObj) -> u32 {
    unsafe { ffi::lv_dropdown_get_selected(dd.obj) }
}

pub fn lv_dropdown_get_selected_text(dd: &LvObj) -> String {
    let mut buf = [0u8; 64];
    unsafe {
        ffi::lv_dropdown_get_selected_str(dd.obj, buf.as_mut_ptr() as *mut c_char, buf.len() as u32);
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

// ── Msgbox ──

pub fn lv_msgbox_show(parent: &LvObj, title: &str, body: &str) {
    let mbox = unsafe { ffi::lv_msgbox_create(parent.obj) };
    let t = std::ffi::CString::new(title).expect("title NUL");
    let b = std::ffi::CString::new(body).expect("body NUL");
    unsafe {
        ffi::lv_msgbox_add_title(mbox, t.as_ptr());
        ffi::lv_msgbox_add_text(mbox, b.as_ptr());
        ffi::lv_msgbox_add_close_button(mbox);
    }
}

// ── Generic helpers ──

pub fn lv_obj_clean(obj: &LvObj) {
    unsafe { ffi::lv_obj_clean(obj.obj) }
}

pub fn lv_obj_set_scroll_dir(obj: &LvObj, dir: u32) {
    unsafe { ffi::lv_obj_set_scroll_dir(obj.obj, dir) }
}

pub fn lv_obj_scroll_to_view(obj: &LvObj, anim_en: u32) {
    unsafe { ffi::lv_obj_scroll_to_view(obj.obj, anim_en) }
}
