#![allow(warnings)]

// Include real bindings only when compiled with the Zephyr toolchain
// (ZEPHYR_BASE set → build.rs does NOT emit cfg(no_zephyr)).
#[cfg(not(any(test, no_zephyr)))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Bindgen on LVGL v9 emits enum-style constants like
// `lv_event_code_t_LV_EVENT_CLICKED` instead of `LV_EVENT_CLICKED`.
// Keep a stable surface for lvgl-dsl by aliasing the names we use.
#[cfg(not(any(test, no_zephyr)))]
pub const LV_FLEX_FLOW_ROW: u32 = lv_flex_flow_t_LV_FLEX_FLOW_ROW as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_FLEX_FLOW_COLUMN: u32 = lv_flex_flow_t_LV_FLEX_FLOW_COLUMN as u32;

#[cfg(not(any(test, no_zephyr)))]
pub const LV_LABEL_LONG_WRAP: u32 = lv_label_long_mode_t_LV_LABEL_LONG_MODE_WRAP as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_LABEL_LONG_DOT: u32 = lv_label_long_mode_t_LV_LABEL_LONG_MODE_DOTS as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_LABEL_LONG_SCROLL: u32 = lv_label_long_mode_t_LV_LABEL_LONG_MODE_SCROLL as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_LABEL_LONG_SCROLL_CIRC: u32 = lv_label_long_mode_t_LV_LABEL_LONG_MODE_SCROLL_CIRCULAR as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_LABEL_LONG_CLIP: u32 = lv_label_long_mode_t_LV_LABEL_LONG_MODE_CLIP as u32;

#[cfg(not(any(test, no_zephyr)))]
pub const LV_EVENT_CLICKED: u32 = lv_event_code_t_LV_EVENT_CLICKED as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_EVENT_SCROLL_END: u32 = lv_event_code_t_LV_EVENT_SCROLL_END as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_EVENT_VALUE_CHANGED: u32 = lv_event_code_t_LV_EVENT_VALUE_CHANGED as u32;
#[cfg(not(any(test, no_zephyr)))]
pub const LV_EVENT_DELETE: u32 = lv_event_code_t_LV_EVENT_DELETE as u32;

#[cfg(not(any(test, no_zephyr)))]
unsafe extern "C" {
    pub fn lv_async_call(
        cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
        user_data: *mut core::ffi::c_void,
    ) -> i32;

    pub fn lv_style_set_bg_image_src(
        style: *mut lv_style_t,
        value: *const core::ffi::c_void,
    );
    pub fn lv_style_set_bg_image_opa(style: *mut lv_style_t, value: u8);
    pub fn lv_style_set_bg_image_tiled(style: *mut lv_style_t, value: bool);

    pub fn lv_obj_set_style_margin_top(obj: *mut lv_obj_t, value: i32, selector: lv_style_selector_t);
    pub fn lv_obj_set_style_margin_left(obj: *mut lv_obj_t, value: i32, selector: lv_style_selector_t);
    pub fn lv_obj_set_style_margin_bottom(obj: *mut lv_obj_t, value: i32, selector: lv_style_selector_t);
    pub fn lv_obj_set_style_length(obj: *mut lv_obj_t, value: i32, selector: lv_style_selector_t);
}

// ============================================================
//  Desktop simulator layer — active when building inside the
//  screen-sdl CMake project (LVGL_INCLUDE_DIRS env var set by
//  CMakeLists.txt → build.rs emits cfg(desktop_sim)).
//
//  Uses real `extern "C"` declarations that link against the
//  LVGL static library already compiled by CMake.
// ============================================================
#[cfg(all(no_zephyr, desktop_sim, not(test)))]
mod desktop {
    #[repr(C)]
    pub struct lv_obj_t {
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
    pub struct lv_font_t {
        _opaque: [u8; 0],
    }

    #[repr(C)]
    pub struct lv_timer_t {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    pub struct lv_display_t {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    pub struct lv_theme_t {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    pub struct lv_indev_t {
        _opaque: [u8; 0],
    }

    /// Layout matches LVGL v9 `lv_style_t` with LV_USE_ASSERT_STYLE=0 (our lv_conf.h default).
    /// `values_and_props` is allocated/freed by lv_style_init / lv_style_reset.
    #[repr(C)]
    pub struct lv_style_t {
        pub values_and_props: *mut core::ffi::c_void,
        pub has_group: u32,
        pub prop_cnt: u8,
    }

    pub type lv_style_prop_t = u8;

    #[repr(C)]
    pub union lv_style_value_t {
        pub num: i32,
        pub ptr: *const core::ffi::c_void,
        pub color: lv_color_t,
    }

    #[repr(C)]
    pub struct lv_style_const_prop_t {
        pub prop: lv_style_prop_t,
        pub value: lv_style_value_t,
    }

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    pub struct lv_color_t {
        pub blue: u8,
        pub green: u8,
        pub red: u8,
    }

    /// Opaque animation object — 256 bytes covers all known LVGL v8/v9 targets.
    ///
    /// The real `lv_anim_t` size is determined by LVGL headers (varies with version
    /// and configuration). 256 bytes is a conservative upper-bound for desktop-sim
    /// builds only; Zephyr builds use real bindgen-generated bindings and never
    /// hard-code this size.
    #[repr(C, align(8))]
    pub struct lv_anim_t {
        _data: [u8; 256],
    }

    /// `lv_result_t_LV_RESULT_OK` value — matches bindgen output for LVGL C enum (1)
    pub const lv_result_t_LV_RESULT_OK: u32 = 1;

    unsafe extern "C" {
        pub static lv_font_montserrat_48: lv_font_t;
        pub static lv_font_montserrat_40: lv_font_t;
        pub static lv_font_montserrat_32: lv_font_t;
        pub static lv_font_montserrat_30: lv_font_t;
        pub static lv_font_montserrat_24: lv_font_t;
        pub static lv_font_montserrat_20: lv_font_t;
        pub static lv_font_montserrat_14: lv_font_t;

        // Object creation
        pub fn lv_obj_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_button_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_label_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_spinner_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_spinner_set_anim_params(obj: *mut lv_obj_t, spin_ms: u32, arc_length_deg: u32);

        // Arc widget
        pub fn lv_arc_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_arc_set_range(obj: *mut lv_obj_t, min: i32, max: i32);
        pub fn lv_arc_set_value(obj: *mut lv_obj_t, value: i32);
        pub fn lv_arc_get_value(obj: *mut lv_obj_t) -> i32;
        pub fn lv_arc_set_bg_angles(obj: *mut lv_obj_t, start: u16, end: u16);
        pub fn lv_arc_set_angles(obj: *mut lv_obj_t, start: u16, end: u16);
        pub fn lv_arc_set_rotation(obj: *mut lv_obj_t, rotation: u16);
        pub fn lv_arc_set_mode(obj: *mut lv_obj_t, mode: u32);
        pub fn lv_arc_set_change_rate(obj: *mut lv_obj_t, rate: u32);
        pub fn lv_obj_remove_style_all(obj: *mut lv_obj_t);

        // Arc part-aware style setters (selector is LV_PART_MAIN/INDICATOR/KNOB)
        pub fn lv_obj_set_style_arc_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_arc_width(obj: *mut lv_obj_t, width: i32, selector: u32);
        pub fn lv_obj_set_style_arc_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);
        pub fn lv_obj_set_style_arc_rounded(obj: *mut lv_obj_t, rounded: bool, selector: u32);

        pub fn lv_screen_active() -> *mut lv_obj_t;

        // Dropdown widget
        pub fn lv_dropdown_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_dropdown_set_options(obj: *mut lv_obj_t, options: *const core::ffi::c_char);
        pub fn lv_dropdown_set_selected(obj: *mut lv_obj_t, sel_opt: u16);
        pub fn lv_dropdown_get_selected(obj: *mut lv_obj_t) -> u16;
        pub fn lv_dropdown_open(obj: *mut lv_obj_t);
        pub fn lv_dropdown_close(obj: *mut lv_obj_t);
        pub fn lv_dropdown_set_dir(obj: *mut lv_obj_t, dir: u32);
        // NOTE: lv_dropdown_set_max_height removed in LVGL v9 — use lv_obj_set_style_max_height.
        pub fn lv_dropdown_set_symbol(obj: *mut lv_obj_t, symbol: *const core::ffi::c_void);

        // Alignment
        pub fn lv_obj_align(obj: *mut lv_obj_t, align: u32, x_ofs: i32, y_ofs: i32);

        // Events
        pub fn lv_obj_add_event_cb(
            obj: *mut lv_obj_t,
            cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
            filter: u32,
            user_data: *mut core::ffi::c_void,
        ) -> *mut lv_event_dsc_t;
        pub fn lv_event_get_user_data(e: *mut lv_event_t) -> *mut core::ffi::c_void;
        pub fn lv_event_get_code(e: *mut lv_event_t) -> u32;
        pub fn lv_event_get_target(e: *mut lv_event_t) -> *mut core::ffi::c_void;

        // State
        pub fn lv_obj_add_state(obj: *mut lv_obj_t, state: u16);
        pub fn lv_obj_remove_state(obj: *mut lv_obj_t, state: u16);
        pub fn lv_obj_has_state(obj: *mut lv_obj_t, state: u16) -> bool;

        // Flags
        pub fn lv_obj_add_flag(obj: *mut lv_obj_t, flag: u32);
        pub fn lv_obj_remove_flag(obj: *mut lv_obj_t, flag: u32);
        pub fn lv_obj_has_flag(obj: *mut lv_obj_t, flag: u32) -> bool;

        // Label
        pub fn lv_label_set_text(obj: *mut lv_obj_t, text: *const core::ffi::c_char);

        // Styles — text / font
        pub fn lv_obj_set_style_text_font(
            obj: *mut lv_obj_t,
            font: *const lv_font_t,
            selector: u32,
        );
        pub fn lv_obj_set_style_text_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_text_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);

        // Screen load
        pub fn lv_screen_load(obj: *mut lv_obj_t);
        pub fn lv_screen_load_anim(obj: *mut lv_obj_t, anim: u32, dur: u32, delay: u32, del: bool);

        // Flex
        pub fn lv_obj_set_flex_flow(obj: *mut lv_obj_t, flow: u32);
        pub fn lv_obj_set_flex_align(obj: *mut lv_obj_t, main: u32, cross: u32, track: u32);
        pub fn lv_obj_set_flex_grow(obj: *mut lv_obj_t, grow: u8);

        // Size
        pub fn lv_obj_set_width(obj: *mut lv_obj_t, w: i32);
        pub fn lv_obj_set_height(obj: *mut lv_obj_t, h: i32);
        pub fn lv_obj_set_size(obj: *mut lv_obj_t, w: i32, h: i32);
        pub fn lv_obj_set_style_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_min_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_max_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_min_height(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_max_height(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_translate_y(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_transform_rotation(obj: *mut lv_obj_t, angle: i32, selector: u32);

        // Padding
        pub fn lv_obj_set_style_pad_row(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_column(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_top(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_bottom(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_left(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_pad_right(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_margin_top(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_margin_left(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_margin_bottom(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Background
        pub fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);

        // Shape / opacity
        pub fn lv_obj_set_style_radius(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_length(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);

        // Border
        pub fn lv_obj_set_style_border_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_border_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_border_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);
        pub fn lv_obj_set_style_border_side(obj: *mut lv_obj_t, value: u32, selector: u32);

        // Outline
        pub fn lv_obj_set_style_outline_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_outline_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_outline_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);
        pub fn lv_obj_set_style_outline_pad(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Shadow
        pub fn lv_obj_set_style_shadow_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_shadow_width(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_shadow_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);
        pub fn lv_obj_set_style_shadow_offset_x(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_shadow_offset_y(obj: *mut lv_obj_t, value: i32, selector: u32);
        pub fn lv_obj_set_style_shadow_spread(obj: *mut lv_obj_t, value: i32, selector: u32);

        // Image widget
        pub fn lv_image_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_image_set_src(obj: *mut lv_obj_t, src: *const core::ffi::c_void);
        pub fn lv_image_set_offset_x(obj: *mut lv_obj_t, x: i32);
        pub fn lv_image_set_offset_y(obj: *mut lv_obj_t, y: i32);
        pub fn lv_image_set_scale(obj: *mut lv_obj_t, factor: u32);
        pub fn lv_image_set_rotation(obj: *mut lv_obj_t, angle: i32);
        pub fn lv_image_set_pivot(obj: *mut lv_obj_t, x: i32, y: i32);
        pub fn lv_image_set_inner_align(obj: *mut lv_obj_t, align: u32);

        // Keyboard widget
        pub fn lv_keyboard_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_keyboard_set_mode(obj: *mut lv_obj_t, mode: u32);
        pub fn lv_keyboard_set_textarea(obj: *mut lv_obj_t, ta: *mut lv_obj_t);
        pub fn lv_keyboard_get_textarea(obj: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_keyboard_set_map(
            obj: *mut lv_obj_t,
            mode: u32,
            map: *const *const core::ffi::c_char,
            ctrl_map: *const u32,
        );

        // ButtonMatrix accessors (keyboard inherits from buttonmatrix)
        pub fn lv_buttonmatrix_get_selected_button(obj: *mut lv_obj_t) -> u32;
        pub fn lv_buttonmatrix_get_button_text(
            obj: *const lv_obj_t,
            btn_id: u32,
        ) -> *const core::ffi::c_char;

        // ButtonMatrix widget (for accent popup)
        pub fn lv_buttonmatrix_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_buttonmatrix_set_map(obj: *mut lv_obj_t, map: *const *const core::ffi::c_char);
        pub fn lv_buttonmatrix_set_ctrl_map(obj: *mut lv_obj_t, ctrl_map: *const u32);
        pub fn lv_buttonmatrix_set_button_width(obj: *mut lv_obj_t, btn_id: u32, width: u32);
        pub fn lv_buttonmatrix_set_button_ctrl(obj: *mut lv_obj_t, btn_id: u32, ctrl: u32);
        pub fn lv_buttonmatrix_clear_button_ctrl(obj: *mut lv_obj_t, btn_id: u32, ctrl: u32);
        pub fn lv_buttonmatrix_set_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32);
        pub fn lv_buttonmatrix_clear_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32);
        pub fn lv_buttonmatrix_set_one_checked(obj: *mut lv_obj_t, en: bool);

        // Object positioning & geometry
        pub fn lv_obj_set_pos(obj: *mut lv_obj_t, x: i32, y: i32);
        pub fn lv_obj_get_x(obj: *mut lv_obj_t) -> i32;
        pub fn lv_obj_get_y(obj: *mut lv_obj_t) -> i32;
        pub fn lv_obj_get_width(obj: *mut lv_obj_t) -> i32;
        pub fn lv_obj_get_parent(obj: *mut lv_obj_t) -> *mut lv_obj_t;

        // TextArea widget
        pub fn lv_textarea_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_textarea_set_placeholder_text(obj: *mut lv_obj_t, txt: *const core::ffi::c_char);
        pub fn lv_textarea_set_max_length(obj: *mut lv_obj_t, num: u32);
        pub fn lv_textarea_set_one_line(obj: *mut lv_obj_t, en: bool);
        pub fn lv_textarea_set_password_mode(obj: *mut lv_obj_t, en: bool);
        pub fn lv_textarea_set_text(obj: *mut lv_obj_t, txt: *const core::ffi::c_char);
        pub fn lv_textarea_get_text(obj: *mut lv_obj_t) -> *const core::ffi::c_char;
        pub fn lv_textarea_delete_char(obj: *mut lv_obj_t);
        pub fn lv_textarea_add_char(obj: *mut lv_obj_t, c: u32);
        pub fn lv_textarea_add_text(obj: *mut lv_obj_t, txt: *const core::ffi::c_char);
        pub fn lv_textarea_cursor_left(obj: *mut lv_obj_t);
        pub fn lv_textarea_cursor_right(obj: *mut lv_obj_t);

        // Object event management
        pub fn lv_obj_send_event(
            obj: *mut lv_obj_t,
            event: u32,
            param: *mut core::ffi::c_void,
        ) -> u32;
        pub fn lv_obj_remove_event_cb(
            obj: *mut lv_obj_t,
            cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        ) -> bool;

        // Default keyboard event handler (for delegation)
        pub fn lv_keyboard_def_event_cb(e: *mut lv_event_t);

        // Keyboard popovers (owned by lv_keyboard in LVGL v9, not buttonmatrix)
        pub fn lv_keyboard_set_popovers(obj: *mut lv_obj_t, en: bool);

        // Image button widget
        pub fn lv_imagebutton_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_imagebutton_set_src(
            obj: *mut lv_obj_t,
            state: u32,
            src_left: *const core::ffi::c_void,
            src_mid: *const core::ffi::c_void,
            src_right: *const core::ffi::c_void,
        );

        // Style — background image
        pub fn lv_obj_set_style_bg_image_src(
            obj: *mut lv_obj_t,
            value: *const core::ffi::c_void,
            selector: u32,
        );
        pub fn lv_obj_set_style_bg_image_tiled(obj: *mut lv_obj_t, value: bool, selector: u32);
        pub fn lv_obj_set_style_bg_image_opa(obj: *mut lv_obj_t, value: u8, selector: u32);
        pub fn lv_obj_set_style_bg_image_recolor(
            obj: *mut lv_obj_t,
            value: lv_color_t,
            selector: u32,
        );
        pub fn lv_obj_set_style_bg_image_recolor_opa(obj: *mut lv_obj_t, value: u8, selector: u32);

        // Style — image
        pub fn lv_obj_set_style_image_recolor(
            obj: *mut lv_obj_t,
            value: lv_color_t,
            selector: u32,
        );
        pub fn lv_obj_set_style_image_recolor_opa(obj: *mut lv_obj_t, value: u8, selector: u32);

        // QR code
        pub fn lv_qrcode_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_qrcode_set_size(obj: *mut lv_obj_t, size: i32);
        pub fn lv_qrcode_set_dark_color(obj: *mut lv_obj_t, color: lv_color_t);
        pub fn lv_qrcode_set_light_color(obj: *mut lv_obj_t, color: lv_color_t);
        pub fn lv_qrcode_update(
            obj: *mut lv_obj_t,
            data: *const core::ffi::c_void,
            data_len: u32,
        ) -> u32;

        // Delete
        pub fn lv_obj_delete(obj: *mut lv_obj_t);

        // Theme
        pub fn lv_theme_default_init(
            disp: *mut lv_display_t,
            color_primary: lv_color_t,
            color_secondary: lv_color_t,
            dark: bool,
            font: *const lv_font_t,
        ) -> *mut lv_theme_t;

        // Style objects (lv_style_t / lv_obj_add_style)
        pub fn lv_style_init(style: *mut lv_style_t);
        pub fn lv_style_reset(style: *mut lv_style_t);
        pub fn lv_obj_add_style(obj: *mut lv_obj_t, style: *const lv_style_t, selector: u32);
        pub fn lv_style_set_bg_color(style: *mut lv_style_t, value: lv_color_t);
        pub fn lv_style_set_bg_opa(style: *mut lv_style_t, value: u8);
        pub fn lv_style_set_text_color(style: *mut lv_style_t, value: lv_color_t);
        pub fn lv_style_set_text_font(style: *mut lv_style_t, value: *const lv_font_t);
        pub fn lv_style_set_border_color(style: *mut lv_style_t, value: lv_color_t);
        pub fn lv_style_set_border_width(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_radius(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_opa(style: *mut lv_style_t, value: u8);
        pub fn lv_style_set_width(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_height(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_pad_top(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_pad_bottom(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_pad_left(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_pad_right(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_bg_image_src(
            style: *mut lv_style_t,
            value: *const core::ffi::c_void,
        );
        pub fn lv_style_set_bg_image_opa(style: *mut lv_style_t, value: u8);
        pub fn lv_style_set_bg_image_tiled(style: *mut lv_style_t, value: bool);
        pub fn lv_style_set_pad_row(style: *mut lv_style_t, value: i32);
        pub fn lv_style_set_pad_column(style: *mut lv_style_t, value: i32);

        // Colors
        pub fn lv_color_hex(val: u32) -> lv_color_t;
        pub fn lv_color_make(r: u8, g: u8, b: u8) -> lv_color_t;
        pub fn lv_color_white() -> lv_color_t;
        pub fn lv_color_black() -> lv_color_t;
        pub fn lv_palette_main(p: u32) -> lv_color_t;
        pub fn lv_palette_lighten(p: u32, level: u8) -> lv_color_t;
        pub fn lv_palette_darken(p: u32, level: u8) -> lv_color_t;

        // Percentage helper
        pub fn lv_pct(v: i32) -> i32;

        // Y-coordinate helpers (used by slide animations)
        pub fn lv_obj_set_y(obj: *mut lv_obj_t, y: i32);
        pub fn lv_obj_get_height(obj: *mut lv_obj_t) -> i32;

        // Animation API (lv_anim.h)
        pub fn lv_anim_init(a: *mut lv_anim_t);
        pub fn lv_anim_set_var(a: *mut lv_anim_t, var: *mut core::ffi::c_void);
        pub fn lv_anim_set_exec_cb(
            a: *mut lv_anim_t,
            exec_cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)>,
        );
        pub fn lv_anim_set_values(a: *mut lv_anim_t, start: i32, end: i32);
        pub fn lv_anim_set_duration(a: *mut lv_anim_t, duration: u32);
        pub fn lv_anim_set_path_cb(
            a: *mut lv_anim_t,
            path_cb: Option<unsafe extern "C" fn(*const lv_anim_t) -> i32>,
        );
        pub fn lv_anim_set_completed_cb(
            a: *mut lv_anim_t,
            completed_cb: Option<unsafe extern "C" fn(*mut lv_anim_t)>,
        );
        pub fn lv_anim_set_repeat_count(a: *mut lv_anim_t, count: u32);
        pub fn lv_anim_start(a: *const lv_anim_t) -> *mut lv_anim_t;
        pub fn lv_anim_delete(
            var: *mut core::ffi::c_void,
            exec_cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)>,
        ) -> u32;
        pub fn lv_anim_path_ease_in(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_ease_out(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_linear(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_ease_in_out(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_overshoot(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_bounce(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_path_step(a: *const lv_anim_t) -> i32;
        pub fn lv_anim_set_user_data(a: *mut lv_anim_t, user_data: *mut core::ffi::c_void);
        pub fn lv_anim_get_user_data(a: *const lv_anim_t) -> *mut core::ffi::c_void;

        // ---- SearchBar deltas (§8) ----
        pub fn lv_timer_create(
            cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
            period_ms: u32,
            user_data: *mut core::ffi::c_void,
        ) -> *mut lv_timer_t;
        pub fn lv_timer_get_user_data(t: *mut lv_timer_t) -> *mut core::ffi::c_void;
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
        /// Defer `cb(user_data)` to the next `lv_timer_handler` invocation —
        /// the standard LVGL escape hatch for code that must mutate widgets
        /// outside an event/render walk.
        pub fn lv_async_call(
            cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
            user_data: *mut core::ffi::c_void,
        ) -> i32;
        pub fn lv_obj_set_user_data(obj: *mut lv_obj_t, ud: *mut core::ffi::c_void);
        pub fn lv_obj_get_user_data(obj: *mut lv_obj_t) -> *mut core::ffi::c_void;
        pub fn lv_label_set_long_mode(label: *mut lv_obj_t, mode: u32);
        pub fn lv_label_set_recolor(label: *mut lv_obj_t, en: bool);
        pub fn lv_obj_clean(obj: *mut lv_obj_t);
        pub fn lv_obj_get_child_count(obj: *mut lv_obj_t) -> u32;
        pub fn lv_obj_get_child(obj: *mut lv_obj_t, idx: i32) -> *mut lv_obj_t;
        /// Move `obj` to a specific z-order index among its siblings. Pass
        /// `lv_obj_get_child_count(parent) - 1` to bring to the foreground.
        /// See LVGL v9 `core/lv_obj_tree.h`.
        pub fn lv_obj_move_to_index(obj: *mut lv_obj_t, index: i32);
        pub fn lv_obj_remove_event_cb_with_user_data(
            obj: *mut lv_obj_t,
            cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
            user_data: *mut core::ffi::c_void,
        );
        pub fn lv_obj_remove_local_style_prop(
            obj: *mut lv_obj_t,
            prop: u8,
            selector: u32,
        ) -> bool;
        pub fn lv_group_focus_obj(obj: *mut lv_obj_t);

        // ---- Input devices ----
        /// Returns the input device currently delivering the active event,
        /// or NULL if the call is not made from within an indev event.
        pub fn lv_indev_active() -> *mut lv_indev_t;
        /// Suppress further events from `indev` until the user releases the
        /// pointer/finger. Used to "swallow" the rest of a long-press so it
        /// can't generate further LV_EVENT_VALUE_CHANGED auto-repeats.
        pub fn lv_indev_wait_release(indev: *mut lv_indev_t);
    }

    // ---------------------------------------------------------
    // LVGL v9 enum constants used by Phase-1 SearchBar code.
    // Defined here so the desktop_sim build resolves them; the
    // mock layer (cfg(test)/cfg(no_zephyr) without desktop_sim)
    // defines its own copies further below.
    // ---------------------------------------------------------
    pub const LV_FLEX_FLOW_ROW: u32 = 0x00;
    pub const LV_FLEX_FLOW_COLUMN: u32 = 0x01;

    pub const LV_LABEL_LONG_WRAP: u32 = 0;
    pub const LV_LABEL_LONG_DOT: u32 = 1;
    pub const LV_LABEL_LONG_SCROLL: u32 = 2;
    pub const LV_LABEL_LONG_SCROLL_CIRC: u32 = 3;
    pub const LV_LABEL_LONG_CLIP: u32 = 4;

    pub const LV_EVENT_CLICKED: u32 = 10;
    pub const LV_EVENT_SCROLL_END: u32 = 31;
    pub const LV_EVENT_VALUE_CHANGED: u32 = 35;
    pub const LV_EVENT_DELETE: u32 = 41;

    pub const LV_ANIM_REPEAT_INFINITE: u32 = 0xFFFF_FFFF;

    pub const LV_PART_MAIN:       u32 = 0x000000;
    pub const LV_PART_SCROLLBAR:  u32 = 0x010000;
    pub const LV_PART_INDICATOR:  u32 = 0x020000;
    pub const LV_PART_KNOB:       u32 = 0x030000;

    pub const LV_ARC_MODE_NORMAL:      u32 = 0;
    pub const LV_ARC_MODE_SYMMETRICAL: u32 = 1;
    pub const LV_ARC_MODE_REVERSE:     u32 = 2;
}

#[cfg(all(no_zephyr, desktop_sim, not(test)))]
pub use desktop::*;

// ============================================================
//  Mock layer — active in two situations:
//    1. cfg(test)      - root crate runs `cargo test`
//    2. cfg(no_zephyr) - compiled as a dependency without Zephyr
//                        toolchain (ZEPHYR_BASE absent at build time)
// ============================================================
#[cfg(any(test, all(no_zephyr, not(desktop_sim))))]
mod mock {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::ffi::CStr;

    // ---------------------------------------------------------
    // Opaque C types
    // ---------------------------------------------------------
    pub struct lv_obj_t {
        _dummy: u8,
    }
    pub struct lv_event_t;
    pub struct lv_event_dsc_t;
    pub struct lv_font_t;
    pub struct lv_display_t;
    pub struct lv_theme_t;
    pub struct lv_indev_t;
    /// Fixed-size placeholder — layout doesn't matter for the mock (all ops are no-ops).
    pub struct lv_style_t {
        pub(crate) _pad: [u8; 16],
    }

    pub type lv_style_prop_t = u8;

    #[repr(C)]
    pub union lv_style_value_t {
        pub num: i32,
        pub ptr: *const core::ffi::c_void,
        pub color: lv_color_t,
    }

    #[repr(C)]
    pub struct lv_style_const_prop_t {
        pub prop: lv_style_prop_t,
        pub value: lv_style_value_t,
    }

    #[repr(C)]
    pub struct lv_timer_t {
        _opaque: [u8; 0],
    }

    #[derive(Copy, Clone, Default, Debug, PartialEq)]
    pub struct lv_color_t {
        pub blue: u8,
        pub green: u8,
        pub red: u8,
    }

    /// Mock animation struct — same size as the desktop version so that
    /// `MaybeUninit::<lv_anim_t>::uninit()` compiles identically under test.
    #[repr(C, align(8))]
    pub struct lv_anim_t {
        _data: [u8; 256],
    }

    // ---------------------------------------------------------
    // LVGL v9.2 constants (flex layout)
    // ---------------------------------------------------------
    pub const LV_FLEX_FLOW_ROW: u32 = 0x00;
    pub const LV_FLEX_FLOW_COLUMN: u32 = 0x01;
    pub const LV_LABEL_LONG_WRAP: u32 = 0;
    pub const LV_LABEL_LONG_DOT: u32 = 1;
    pub const LV_LABEL_LONG_SCROLL: u32 = 2;
    pub const LV_LABEL_LONG_SCROLL_CIRC: u32 = 3;
    pub const LV_LABEL_LONG_CLIP: u32 = 4;
    pub const LV_EVENT_CLICKED: u32 = 10;
    pub const LV_EVENT_SCROLL_END: u32 = 31;
    pub const LV_EVENT_VALUE_CHANGED: u32 = 35;
    pub const LV_EVENT_DELETE: u32 = 41;
    pub const LV_ANIM_REPEAT_INFINITE: u32 = 0xFFFF_FFFF;

    pub const LV_PART_MAIN:       u32 = 0x000000;
    pub const LV_PART_SCROLLBAR:  u32 = 0x010000;
    pub const LV_PART_INDICATOR:  u32 = 0x020000;
    pub const LV_PART_KNOB:       u32 = 0x030000;

    pub const LV_ARC_MODE_NORMAL:      u32 = 0;
    pub const LV_ARC_MODE_SYMMETRICAL: u32 = 1;
    pub const LV_ARC_MODE_REVERSE:     u32 = 2;

    // Static font symbols used by Font::montserrat_*().
    pub static lv_font_montserrat_48: lv_font_t = lv_font_t;
    // Static font symbols used by Font::montserrat_*() methods.
    pub static lv_font_montserrat_40: lv_font_t = lv_font_t;
    pub static lv_font_montserrat_32: lv_font_t = lv_font_t;
    pub static lv_font_montserrat_30: lv_font_t = lv_font_t;
    pub static lv_font_montserrat_24: lv_font_t = lv_font_t;
    pub static lv_font_montserrat_20: lv_font_t = lv_font_t;
    pub static lv_font_montserrat_14: lv_font_t = lv_font_t;

    // ---------------------------------------------------------
    // Fake object pool
    //
    // Returns a unique non-null pointer per call so that the
    // non-null assertions in every Widget::new() pass.
    // Uses a static Vec so pointer stability is guaranteed for
    // the lifetime of the pool (no realloc).
    // ---------------------------------------------------------
    thread_local! {
        static OBJ_IDX: Cell<usize> = const { Cell::new(0) };
        // The Vec is pre-allocated to its full capacity and fully populated
        // once, so it never reallocates afterwards — element addresses stay
        // stable for the lifetime of the pool, which is what lets us hand out
        // raw `*mut lv_obj_t` into it.
        static OBJ_BUF: RefCell<Vec<lv_obj_t>> = {
            let mut v = Vec::with_capacity(256);
            for _ in 0..256 { v.push(lv_obj_t { _dummy: 0 }); }
            RefCell::new(v)
        };
    }

    pub fn alloc_fake_obj() -> *mut lv_obj_t {
        OBJ_IDX.with(|idx| {
            let i = idx.get();
            assert!(
                i < 256,
                "fake object pool exhausted — call reset_obj_pool() between tests"
            );
            idx.set(i + 1);
            OBJ_BUF.with(|buf| {
                // SAFETY: index is within capacity; Vec never reallocates.
                &mut buf.borrow_mut()[i] as *mut lv_obj_t
            })
        })
    }

    fn register_child(parent: *mut lv_obj_t, child: *mut lv_obj_t) {
        if parent.is_null() {
            return;
        }
        CHILDREN.with(|m| {
            m.borrow_mut()
                .entry(parent as usize)
                .or_default()
                .push(child as usize);
        });
    }

    // ---------------------------------------------------------
    // Spy call log
    // ---------------------------------------------------------
    #[derive(Debug, PartialEq, Clone)]
    pub enum LvCall {
        AddEventCb {
            obj: usize,
            code: u32,
        },
        AddFlag {
            obj: usize,
            flag: u32,
        },
        AddState {
            obj: usize,
            state: u16,
        },
        Align {
            obj: usize,
            align: u32,
            x: i32,
            y: i32,
        },
        ButtonCreate {
            obj: usize,
            parent: usize,
        },
        ButtonMatrixCreate {
            obj: usize,
            parent: usize,
        },
        ButtonMatrixSetMap {
            obj: usize,
            labels: Vec<Vec<u8>>,
        },
        ButtonMatrixSetCtrlMap {
            obj: usize,
            ctrl: Vec<u32>,
        },
        ButtonMatrixSetButtonWidth {
            obj: usize,
            btn_id: u32,
            width: u32,
        },
        ButtonMatrixSetButtonCtrl {
            obj: usize,
            btn_id: u32,
            ctrl: u32,
        },
        ButtonMatrixClearButtonCtrl {
            obj: usize,
            btn_id: u32,
            ctrl: u32,
        },
        ButtonMatrixSetButtonCtrlAll {
            obj: usize,
            ctrl: u32,
        },
        ButtonMatrixClearButtonCtrlAll {
            obj: usize,
            ctrl: u32,
        },
        ButtonMatrixSetOneChecked {
            obj: usize,
            en: bool,
        },
        ButtonMatrixGetSelectedButton {
            obj: usize,
            ret: u32,
        },
        ButtonMatrixGetButtonText {
            obj: usize,
            btn_id: u32,
            text: Option<Vec<u8>>,
        },
        ButtonMatrixSetPopovers {
            obj: usize,
            en: bool,
        },
        DropdownClose {
            obj: usize,
        },
        DropdownCreate {
            obj: usize,
        },
        DropdownOpen {
            obj: usize,
        },
        DropdownSetDir {
            obj: usize,
            dir: u32,
        },
        DropdownSetOptions {
            obj: usize,
            options_bytes: Vec<u8>,
        },
        DropdownSetSelected {
            obj: usize,
            index: u16,
        },
        DropdownSetSymbol {
            obj: usize,
            symbol_bytes: Option<Vec<u8>>,
        },
        GroupFocusObj {
            obj: usize,
        },
        ImageButtonCreate {
            obj: usize,
        },
        ImageButtonSetSrc {
            obj: usize,
            state: u32,
            src: usize,
        },
        ImageCreate {
            obj: usize,
        },
        ImageSetOffset {
            obj: usize,
            x: i32,
            y: i32,
        },
        ImageSetPivot {
            obj: usize,
            x: i32,
            y: i32,
        },
        ImageSetRotation {
            obj: usize,
            angle: i32,
        },
        ImageSetScale {
            obj: usize,
            factor: u32,
        },
        ImageSetInnerAlign {
            obj: usize,
            align: u32,
        },
        ImageSetSrc {
            obj: usize,
            src: usize,
        },
        IndevActive,
        IndevWaitRelease {
            indev: usize,
        },
        ObjMoveToIndex {
            obj: usize,
            index: i32,
        },
        KeyboardCreate {
            obj: usize,
        },
        KeyboardSetMap {
            obj: usize,
            mode: u32,
        },
        KeyboardSetMode {
            obj: usize,
            mode: u32,
        },
        KeyboardSetTextarea {
            obj: usize,
            ta: usize,
        },
        LabelCreate {
            obj: usize,
            parent: usize,
        },
        LabelSetLongMode {
            label: usize,
            mode: u32,
        },
        LabelSetRecolor {
            label: usize,
            en: bool,
        },
        LabelSetText {
            obj: usize,
            text_bytes: Vec<u8>,
        },
        ObjCreate {
            obj: usize,
            parent: usize,
        },
        ObjClean {
            obj: usize,
        },
        ObjDelete {
            obj: usize,
        },
        ObjGetChildCount {
            obj: usize,
            ret: u32,
        },
        ObjGetChild {
            obj: usize,
            idx: i32,
            ret: usize,
        },
        ObjGetScrollBottom {
            obj: usize,
            ret: i32,
        },
        ObjGetScrollTop {
            obj: usize,
            ret: i32,
        },
        ObjGetUserData {
            obj: usize,
            ret: usize,
        },
        ObjScrollToView {
            obj: usize,
            anim: u32,
        },
        AsyncCall {
            user_data: usize,
        },
        ObjSetFlexFlow {
            obj: usize,
            flow: u32,
        },
        ObjSetFlexAlign {
            obj: usize,
            main: u32,
            cross: u32,
            track: u32,
        },
        ObjSetHeight {
            obj: usize,
            h: i32,
        },
        ObjSetPos {
            obj: usize,
            x: i32,
            y: i32,
        },
        ObjSetScrollbarMode {
            obj: usize,
            mode: u32,
        },
        ObjSetSize {
            obj: usize,
            w: i32,
            h: i32,
        },
        ObjSetUserData {
            obj: usize,
            data: usize,
        },
        ObjSetWidth {
            obj: usize,
            w: i32,
        },
        QrCodeCreate {
            obj: usize,
        },
        QrCodeSetDarkColor {
            obj: usize,
            color: lv_color_t,
        },
        QrCodeSetLightColor {
            obj: usize,
            color: lv_color_t,
        },
        QrCodeSetSize {
            obj: usize,
            size: i32,
        },
        QrCodeUpdate {
            obj: usize,
            data_len: u32,
        },
        RemoveEventCbWithUserData {
            obj: usize,
            user_data: usize,
        },
        RemoveFlag {
            obj: usize,
            flag: u32,
        },
        RemoveState {
            obj: usize,
            state: u16,
        },
        SetStyleBgColor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleBgImageOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleBgImageRecolor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleBgImageRecolorOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleImageRecolor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleImageRecolorOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleBgImageSrc {
            obj: usize,
            src: usize,
        },
        SetStyleBgImageTiled {
            obj: usize,
            tiled: bool,
        },
        SetStyleBgOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleBorderColor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleBorderOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleBorderSide {
            obj: usize,
            value: u32,
        },
        SetStyleBorderWidth {
            obj: usize,
            value: i32,
        },
        SetStyleMaxHeight {
            obj: usize,
            value: i32,
        },
        SetStyleTranslateY {
            obj: usize,
            value: i32,
        },
        SetStyleTransformRotation {
            obj: usize,
            angle: i32,
        },
        SetStyleOpa {
            obj: usize,
            opa: u8,
        },
        SetStylePadBottom {
            obj: usize,
            value: i32,
        },
        SetStylePadLeft {
            obj: usize,
            value: i32,
        },
        SetStylePadRight {
            obj: usize,
            value: i32,
        },
        SetStylePadRow {
            obj: usize,
            value: i32,
        },
        SetStylePadColumn {
            obj: usize,
            value: i32,
        },
        SetStylePadTop {
            obj: usize,
            value: i32,
        },
        SetStyleMarginTop {
            obj: usize,
            value: i32,
        },
        SetStyleMarginLeft {
            obj: usize,
            value: i32,
        },
        SetStyleMarginBottom {
            obj: usize,
            value: i32,
        },
        SetStyleOutlineColor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleOutlineOpa {
            obj: usize,
            opa: u8,
        },
        SetStyleOutlinePad {
            obj: usize,
            value: i32,
        },
        SetStyleOutlineWidth {
            obj: usize,
            value: i32,
        },
        SetStyleRadius {
            obj: usize,
            value: i32,
        },
        SetStyleTextFont {
            obj: usize,
        },
        SetStyleTextColor {
            obj: usize,
            color: lv_color_t,
        },
        SetStyleTextOpa {
            obj: usize,
            opa: u8,
        },
        SpinnerCreate {
            obj: usize,
            parent: usize,
        },
        SpinnerSetAnimParams {
            obj: usize,
            spin_ms: u32,
            arc_length_deg: u32,
        },
        SpyEmitEvent {
            obj: usize,
            code: u32,
        },
        TextAreaCreate {
            obj: usize,
            parent: usize,
        },
        TextAreaSetMaxLength {
            obj: usize,
            max: u32,
        },
        TextAreaSetOneLine {
            obj: usize,
            en: bool,
        },
        TextAreaSetPasswordMode {
            obj: usize,
            en: bool,
        },
        TextAreaSetPlaceholder {
            obj: usize,
            text: Vec<u8>,
        },
        TextAreaSetText {
            obj: usize,
            text: Vec<u8>,
        },
        TimerCreate {
            handle: usize,
            period_ms: u32,
            user_data: usize,
        },
        TimerDelete {
            handle: usize,
        },
        TimerPause {
            handle: usize,
        },
        TimerReset {
            handle: usize,
        },
        TimerResume {
            handle: usize,
        },
        TimerSetPeriod {
            handle: usize,
            period_ms: u32,
        },
        TimerSetRepeatCount {
            handle: usize,
            count: i32,
        },
        AnimInit,
        AnimSetVar { var: usize },
        AnimSetExecCb { cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)> },
        AnimSetValues { start: i32, end: i32 },
        AnimSetDuration { ms: u32 },
        AnimSetPathCb { cb: Option<unsafe extern "C" fn(*const lv_anim_t) -> i32> },
        AnimSetCompletedCb { cb: Option<unsafe extern "C" fn(*mut lv_anim_t)> },
        AnimStart,
        AnimDelete { var: usize, cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)> },
        AnimSetRepeatCount {
            count: u32,
        },
        ArcCreate { obj: usize, parent: usize },
        ArcSetRange { obj: usize, min: i32, max: i32 },
        ArcSetValue { obj: usize, value: i32 },
        ArcSetBgAngles { obj: usize, start: u16, end: u16 },
        ArcSetAngles { obj: usize, start: u16, end: u16 },
        ArcSetRotation { obj: usize, rotation: u16 },
        ArcSetMode { obj: usize, mode: u32 },
        ArcSetChangeRate { obj: usize, rate: u32 },
        ObjRemoveStyleAll { obj: usize },
        StyleArcColor { obj: usize, color: lv_color_t, selector: u32 },
        StyleArcWidth { obj: usize, width: i32, selector: u32 },
        StyleArcOpa { obj: usize, opa: u8, selector: u32 },
        StyleArcRounded { obj: usize, rounded: bool, selector: u32 },
        RemoveLocalStyleProp { obj: usize, prop: u8, selector: u32 },
    }

    thread_local! {
        pub static SPY:               RefCell<Vec<LvCall>>         = RefCell::new(Vec::new());
        static OBJ_STATE:         RefCell<HashMap<usize, u16>> = RefCell::new(HashMap::new());
        static OBJ_FLAGS:         RefCell<HashMap<usize, u32>> = RefCell::new(HashMap::new());
        static DROPDOWN_SELECTED: RefCell<HashMap<usize, u16>> = RefCell::new(HashMap::new());
        static BUTTONMATRIX_MAPS:
            RefCell<HashMap<usize, Vec<*const core::ffi::c_char>>> = RefCell::new(HashMap::new());
        static BUTTONMATRIX_CTRLS:
            RefCell<HashMap<usize, Vec<u32>>> = RefCell::new(HashMap::new());
        static BUTTONMATRIX_SELECTED:
            RefCell<HashMap<usize, u32>> = RefCell::new(HashMap::new());
        // Pending x value buffered between lv_image_set_offset_x and lv_image_set_offset_y
        static PENDING_OFFSET_X:  Cell<(usize, i32)>           = const { Cell::new((0, 0)) };
        static ARC_VALUE: RefCell<HashMap<usize, i32>> = RefCell::new(HashMap::new());
    }

    /// Drain and return all recorded calls since the last reset/drain.
    pub fn spy_drain() -> Vec<LvCall> {
        SPY.with(|s| s.borrow_mut().drain(..).collect())
    }

    /// Reset object pool, state/flag maps, and spy log.
    /// Call at the start of every test that creates widgets.
    pub fn reset_obj_pool() {
        OBJ_IDX.with(|idx| idx.set(0));
        OBJ_STATE.with(|m| m.borrow_mut().clear());
        OBJ_FLAGS.with(|m| m.borrow_mut().clear());
        DROPDOWN_SELECTED.with(|m| m.borrow_mut().clear());
        BUTTONMATRIX_MAPS.with(|m| m.borrow_mut().clear());
        BUTTONMATRIX_CTRLS.with(|m| m.borrow_mut().clear());
        BUTTONMATRIX_SELECTED.with(|m| m.borrow_mut().clear());
        PENDING_OFFSET_X.with(|cell| cell.set((0, 0)));
        ARC_VALUE.with(|m| m.borrow_mut().clear());
        CHILDREN.with(|m| m.borrow_mut().clear());
        EVENT_REG.with(|m| m.borrow_mut().clear());
        SPY.with(|s| s.borrow_mut().clear());
    }

    // ---------------------------------------------------------
    // SearchBar (§8) — extended spy registries
    // ---------------------------------------------------------
    use core::ffi::c_void;

    #[derive(Clone, Copy)]
    pub struct EventReg {
        pub code: u32,
        pub cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        pub user_data: *mut c_void,
    }

    pub struct TimerReg {
        pub period_ms: u32,
        pub repeat_count: i32, // §10: signed; -1 = infinite
        pub cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
        pub user_data: *mut c_void,
        pub paused: bool,
    }

    thread_local! {
        pub(crate) static EVENT_REG:
            RefCell<HashMap<usize, Vec<EventReg>>> = RefCell::new(HashMap::new());
        pub(crate) static USER_DATA:
            RefCell<HashMap<usize, usize>>         = RefCell::new(HashMap::new());
        pub(crate) static TEXTAREA_TEXT:
            RefCell<HashMap<usize, alloc::ffi::CString>> = RefCell::new(HashMap::new());
        pub(crate) static TIMER_REG:
            RefCell<HashMap<usize, TimerReg>>      = RefCell::new(HashMap::new());
        pub(crate) static NEXT_TIMER_HANDLE:
            Cell<usize>                            = const { Cell::new(0x1000) };
        pub(crate) static NEXT_SCROLL_BOTTOM:      Cell<i32> = const { Cell::new(0) };
        pub(crate) static NEXT_SCROLL_TOP:         Cell<i32> = const { Cell::new(0) };
        pub(crate) static CHILD_COUNTS:
            RefCell<HashMap<usize, u32>>           = RefCell::new(HashMap::new());
        pub(crate) static CHILDREN:
            RefCell<HashMap<usize, Vec<usize>>>    = RefCell::new(HashMap::new());
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
        TEXTAREA_TEXT.with(|m| m.borrow_mut().clear());
        TIMER_REG.with(|m| m.borrow_mut().clear());
        NEXT_TIMER_HANDLE.with(|c| c.set(0x1000));
        NEXT_SCROLL_BOTTOM.with(|c| c.set(0));
        NEXT_SCROLL_TOP.with(|c| c.set(0));
        CHILD_COUNTS.with(|m| m.borrow_mut().clear());
        CHILDREN.with(|m| m.borrow_mut().clear());
        CURRENT_EVENT.with(|c| c.set((0, 0, 0)));
        ANIM_USER_DATA.with(|m| m.borrow_mut().clear());
    }

    /// RAII fixture: resets spy state at construction (so a previous
    /// panicking test cannot poison this one) and again on Drop.
    pub struct SpyFixture(());
    impl SpyFixture {
        pub fn new() -> Self {
            reset_all_thread_local_spy_state();
            SpyFixture(())
        }
    }
    impl Default for SpyFixture {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Drop for SpyFixture {
        fn drop(&mut self) {
            reset_all_thread_local_spy_state();
        }
    }

    // ---- Scroll injection helpers (used by pagination tests) ----
    pub fn set_next_scroll_bottom(px: i32) {
        NEXT_SCROLL_BOTTOM.with(|c| c.set(px));
    }
    pub fn set_next_scroll_top(px: i32) {
        NEXT_SCROLL_TOP.with(|c| c.set(px));
    }
    pub fn set_child_count(obj: *mut lv_obj_t, n: u32) {
        CHILD_COUNTS.with(|m| {
            m.borrow_mut().insert(obj as usize, n);
        });
    }

    // ---- Synthesized event firing ----
    pub fn spy_emit_event(obj: *mut lv_obj_t, code: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SpyEmitEvent {
                obj: obj as usize,
                code,
            })
        });
        let regs: Vec<EventReg> =
            EVENT_REG.with(|m| m.borrow().get(&(obj as usize)).cloned().unwrap_or_default());
        for r in regs {
            if r.code != 0 /* LV_EVENT_ALL */ && r.code != code {
                continue;
            }
            let prev =
                CURRENT_EVENT.with(|c| c.replace((obj as usize, code, r.user_data as usize)));
            if let Some(cb) = r.cb {
                // Synthetic event_t is a type-erased token; the spy
                // accessors read CURRENT_EVENT instead of dereferencing it.
                unsafe {
                    cb(core::ptr::dangling_mut::<lv_event_t>());
                }
            }
            CURRENT_EVENT.with(|c| c.set(prev));
        }
    }

    pub fn spy_fire_timer(handle: *mut lv_timer_t) {
        let h = handle as usize;
        let action: Option<(
            Option<unsafe extern "C" fn(*mut lv_timer_t)>,
            bool, /*remove*/
        )> = TIMER_REG.with(|m| {
            let mut m = m.borrow_mut();
            let Some(t) = m.get_mut(&h) else {
                return None;
            };
            if t.paused {
                return Some((None, false));
            }
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
                if let Some(cb) = cb {
                    unsafe {
                        cb(handle);
                    }
                }
                if remove {
                    TIMER_REG.with(|m| {
                        m.borrow_mut().remove(&h);
                    });
                }
            }
        }
    }

    pub fn spy_live_timer_handles() -> Vec<usize> {
        TIMER_REG.with(|m| m.borrow().keys().copied().collect())
    }

    // ---------------------------------------------------------
    // Object creation
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }
    pub unsafe fn lv_button_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }
    pub unsafe fn lv_label_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::LabelCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }
    pub unsafe fn lv_spinner_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SpinnerCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }
    pub unsafe fn lv_spinner_set_anim_params(
        obj: *mut lv_obj_t,
        spin_ms: u32,
        arc_length_deg: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SpinnerSetAnimParams {
                obj: obj as usize,
                spin_ms,
                arc_length_deg,
            })
        });
    }
    pub unsafe fn lv_arc_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        ARC_VALUE.with(|m| { m.borrow_mut().insert(obj as usize, 0); });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ArcCreate { obj: obj as usize, parent: parent as usize })
        });
        obj
    }
    pub unsafe fn lv_arc_set_range(obj: *mut lv_obj_t, min: i32, max: i32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetRange { obj: obj as usize, min, max }));
    }
    pub unsafe fn lv_arc_set_value(obj: *mut lv_obj_t, value: i32) {
        ARC_VALUE.with(|m| { m.borrow_mut().insert(obj as usize, value); });
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetValue { obj: obj as usize, value }));
    }
    pub unsafe fn lv_arc_get_value(obj: *mut lv_obj_t) -> i32 {
        ARC_VALUE.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0))
    }
    pub unsafe fn lv_arc_set_bg_angles(obj: *mut lv_obj_t, start: u16, end: u16) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetBgAngles { obj: obj as usize, start, end }));
    }
    pub unsafe fn lv_arc_set_angles(obj: *mut lv_obj_t, start: u16, end: u16) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetAngles { obj: obj as usize, start, end }));
    }
    pub unsafe fn lv_arc_set_rotation(obj: *mut lv_obj_t, rotation: u16) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetRotation { obj: obj as usize, rotation }));
    }
    pub unsafe fn lv_arc_set_mode(obj: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetMode { obj: obj as usize, mode }));
    }
    pub unsafe fn lv_arc_set_change_rate(obj: *mut lv_obj_t, rate: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ArcSetChangeRate { obj: obj as usize, rate }));
    }
    pub unsafe fn lv_obj_remove_style_all(obj: *mut lv_obj_t) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjRemoveStyleAll { obj: obj as usize }));
    }
    pub unsafe fn lv_obj_set_style_arc_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::StyleArcColor { obj: obj as usize, color, selector }));
    }
    pub unsafe fn lv_obj_set_style_arc_width(obj: *mut lv_obj_t, width: i32, selector: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::StyleArcWidth { obj: obj as usize, width, selector }));
    }
    pub unsafe fn lv_obj_set_style_arc_opa(obj: *mut lv_obj_t, opa: u8, selector: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::StyleArcOpa { obj: obj as usize, opa, selector }));
    }
    pub unsafe fn lv_obj_set_style_arc_rounded(obj: *mut lv_obj_t, rounded: bool, selector: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::StyleArcRounded { obj: obj as usize, rounded, selector }));
    }
    pub unsafe fn lv_screen_active() -> *mut lv_obj_t {
        alloc_fake_obj()
    }
    pub unsafe fn lv_dropdown_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::DropdownCreate { obj: obj as usize })
        });
        obj
    }
    pub unsafe fn lv_dropdown_set_options(obj: *mut lv_obj_t, options: *const core::ffi::c_char) {
        let bytes = unsafe { CStr::from_ptr(options) }
            .to_bytes_with_nul()
            .to_vec();
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::DropdownSetOptions {
                obj: obj as usize,
                options_bytes: bytes,
            })
        });
    }
    pub unsafe fn lv_dropdown_set_selected(obj: *mut lv_obj_t, sel_opt: u16) {
        DROPDOWN_SELECTED.with(|m| m.borrow_mut().insert(obj as usize, sel_opt));
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::DropdownSetSelected {
                obj: obj as usize,
                index: sel_opt,
            })
        });
    }
    pub unsafe fn lv_dropdown_get_selected(obj: *mut lv_obj_t) -> u16 {
        DROPDOWN_SELECTED.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0))
    }
    pub unsafe fn lv_dropdown_open(obj: *mut lv_obj_t) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::DropdownOpen { obj: obj as usize })
        });
    }
    pub unsafe fn lv_dropdown_close(obj: *mut lv_obj_t) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::DropdownClose { obj: obj as usize })
        });
    }
    pub unsafe fn lv_dropdown_set_dir(obj: *mut lv_obj_t, dir: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::DropdownSetDir {
                obj: obj as usize,
                dir,
            })
        });
    }
    pub unsafe fn lv_dropdown_set_symbol(obj: *mut lv_obj_t, symbol: *const core::ffi::c_void) {
        let bytes = if symbol.is_null() {
            None
        } else {
            // SAFETY: caller guarantees a valid NUL-terminated string when non-null.
            Some(
                unsafe { CStr::from_ptr(symbol as *const core::ffi::c_char) }
                    .to_bytes_with_nul()
                    .to_vec(),
            )
        };
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::DropdownSetSymbol {
                obj: obj as usize,
                symbol_bytes: bytes,
            })
        });
    }

    // ---------------------------------------------------------
    // Alignment
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_align(obj: *mut lv_obj_t, align: u32, x_ofs: i32, y_ofs: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::Align {
                obj: obj as usize,
                align,
                x: x_ofs,
                y: y_ofs,
            })
        });
    }

    // ---------------------------------------------------------
    // Event callback (populates EVENT_REG so spy_emit_event can dispatch)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_add_event_cb(
        obj: *mut lv_obj_t,
        cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
        code: u32,
        user_data: *mut core::ffi::c_void,
    ) -> *mut lv_event_dsc_t {
        EVENT_REG.with(|m| {
            m.borrow_mut()
                .entry(obj as usize)
                .or_default()
                .push(EventReg {
                    code,
                    cb,
                    user_data,
                });
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::AddEventCb {
                obj: obj as usize,
                code,
            })
        });
        core::ptr::null_mut()
    }

    // ---------------------------------------------------------
    // State (stateful mock for roundtrip tests)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_add_state(obj: *mut lv_obj_t, state: u16) {
        let key = obj as usize;
        OBJ_STATE.with(|m| {
            let prev = m.borrow().get(&key).copied().unwrap_or(0);
            m.borrow_mut().insert(key, prev | state);
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::AddState { obj: key, state }));
    }
    pub unsafe fn lv_obj_remove_state(obj: *mut lv_obj_t, state: u16) {
        let key = obj as usize;
        OBJ_STATE.with(|m| {
            let prev = m.borrow().get(&key).copied().unwrap_or(0);
            m.borrow_mut().insert(key, prev & !state);
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::RemoveState { obj: key, state }));
    }
    pub unsafe fn lv_obj_has_state(obj: *mut lv_obj_t, state: u16) -> bool {
        OBJ_STATE.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0) & state != 0)
    }

    // ---------------------------------------------------------
    // Flags (stateful mock for roundtrip tests)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_add_flag(obj: *mut lv_obj_t, flag: u32) {
        let key = obj as usize;
        OBJ_FLAGS.with(|m| {
            let prev = m.borrow().get(&key).copied().unwrap_or(0);
            m.borrow_mut().insert(key, prev | flag);
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::AddFlag { obj: key, flag }));
    }
    pub unsafe fn lv_obj_remove_flag(obj: *mut lv_obj_t, flag: u32) {
        let key = obj as usize;
        OBJ_FLAGS.with(|m| {
            let prev = m.borrow().get(&key).copied().unwrap_or(0);
            m.borrow_mut().insert(key, prev & !flag);
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::RemoveFlag { obj: key, flag }));
    }
    pub unsafe fn lv_obj_has_flag(obj: *mut lv_obj_t, flag: u32) -> bool {
        OBJ_FLAGS.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0) & flag != 0)
    }

    // ---------------------------------------------------------
    // Label text
    // ---------------------------------------------------------
    pub unsafe fn lv_label_set_text(obj: *mut lv_obj_t, text: *const core::ffi::c_char) {
        // SAFETY: the caller (to_null_terminated) guarantees a valid NUL-terminated buffer.
        let bytes = unsafe { CStr::from_ptr(text) }.to_bytes_with_nul().to_vec();
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::LabelSetText {
                obj: obj as usize,
                text_bytes: bytes,
            })
        });
    }

    // ---------------------------------------------------------
    // Text font
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_style_text_font(
        obj: *mut lv_obj_t,
        _font: *const lv_font_t,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::SetStyleTextFont { obj: obj as usize })
        });
    }

    // ---------------------------------------------------------
    // Screen load (no-ops)
    // ---------------------------------------------------------
    pub unsafe fn lv_screen_load(_obj: *mut lv_obj_t) {}
    pub unsafe fn lv_screen_load_anim(
        _obj: *mut lv_obj_t,
        _anim: u32,
        _dur: u32,
        _delay: u32,
        _del: bool,
    ) {
    }

    // ---------------------------------------------------------
    // Flex layout
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_flex_flow(obj: *mut lv_obj_t, flow: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetFlexFlow {
                obj: obj as usize,
                flow,
            })
        });
    }
    pub unsafe fn lv_obj_set_flex_align(obj: *mut lv_obj_t, main: u32, cross: u32, track: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetFlexAlign {
                obj: obj as usize,
                main,
                cross,
                track,
            })
        });
    }
    pub unsafe fn lv_obj_set_flex_grow(_obj: *mut lv_obj_t, _grow: u8) {}

    // ---------------------------------------------------------
    // Sizing
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_width(obj: *mut lv_obj_t, w: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetWidth {
                obj: obj as usize,
                w,
            })
        });
    }
    pub unsafe fn lv_obj_set_height(obj: *mut lv_obj_t, h: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetHeight {
                obj: obj as usize,
                h,
            })
        });
    }
    pub unsafe fn lv_obj_set_size(obj: *mut lv_obj_t, w: i32, h: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetSize {
                obj: obj as usize,
                w,
                h,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_width(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_min_width(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_max_width(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_min_height(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_max_height(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleMaxHeight {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_translate_y(obj: *mut lv_obj_t, value: i32, _selector: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleTranslateY {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_transform_rotation(
        obj: *mut lv_obj_t,
        angle: i32,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleTransformRotation {
                obj: obj as usize,
                angle,
            })
        });
    }

    // ---------------------------------------------------------
    // Padding (no-ops)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_style_pad_row(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadRow {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_pad_column(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadColumn {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_pad_top(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadTop {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_margin_top(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleMarginTop {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_margin_left(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleMarginLeft {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_margin_bottom(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleMarginBottom {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_pad_bottom(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadBottom {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_pad_left(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadLeft {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_pad_right(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStylePadRight {
                obj: obj as usize,
                value,
            })
        });
    }

    // ---------------------------------------------------------
    // Style: background, text, shape, border, outline, shadow (no-ops)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgOpa {
                obj: obj as usize,
                opa,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_text_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleTextColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_text_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleTextOpa {
                obj: obj as usize,
                opa,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_radius(obj: *mut lv_obj_t, value: i32, _selector: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleRadius {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_length(_obj: *mut lv_obj_t, _value: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleOpa {
                obj: obj as usize,
                opa,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_border_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBorderColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_border_width(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBorderWidth {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_remove_local_style_prop(
        obj: *mut lv_obj_t,
        prop: u8,
        selector: u32,
    ) -> bool {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::RemoveLocalStyleProp {
                obj: obj as usize,
                prop,
                selector,
            })
        });
        true
    }
    pub unsafe fn lv_obj_set_style_border_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBorderOpa {
                obj: obj as usize,
                opa,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_border_side(obj: *mut lv_obj_t, value: u32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBorderSide {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_outline_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleOutlineColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_outline_width(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleOutlineWidth {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_outline_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleOutlineOpa {
                obj: obj as usize,
                opa,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_outline_pad(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleOutlinePad {
                obj: obj as usize,
                value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_shadow_color(_: *mut lv_obj_t, _: lv_color_t, _: u32) {}
    pub unsafe fn lv_obj_set_style_shadow_width(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_shadow_opa(_: *mut lv_obj_t, _: u8, _: u32) {}
    pub unsafe fn lv_obj_set_style_shadow_offset_x(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_shadow_offset_y(_: *mut lv_obj_t, _: i32, _: u32) {}
    pub unsafe fn lv_obj_set_style_shadow_spread(_: *mut lv_obj_t, _: i32, _: u32) {}

    // ---------------------------------------------------------
    // Image widget
    // ---------------------------------------------------------
    pub unsafe fn lv_image_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::ImageCreate { obj: obj as usize })
        });
        obj
    }
    pub unsafe fn lv_image_set_src(obj: *mut lv_obj_t, src: *const core::ffi::c_void) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetSrc {
                obj: obj as usize,
                src: src as usize,
            })
        });
    }
    pub unsafe fn lv_image_set_offset_x(obj: *mut lv_obj_t, x: i32) {
        // Buffer (obj, x) until lv_image_set_offset_y completes the pair.
        PENDING_OFFSET_X.with(|cell| cell.set((obj as usize, x)));
    }
    pub unsafe fn lv_image_set_offset_y(obj: *mut lv_obj_t, y: i32) {
        // Emit the combined ImageSetOffset spy record.
        let (pending_obj, x) = PENDING_OFFSET_X.with(|cell| cell.get());
        debug_assert_eq!(
            pending_obj, obj as usize,
            "lv_image_set_offset_y called without matching lv_image_set_offset_x for the same object"
        );
        PENDING_OFFSET_X.with(|cell| cell.set((0, 0)));
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetOffset {
                obj: obj as usize,
                x,
                y,
            })
        });
    }
    pub unsafe fn lv_image_set_scale(obj: *mut lv_obj_t, factor: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetScale {
                obj: obj as usize,
                factor,
            })
        });
    }
    pub unsafe fn lv_image_set_rotation(obj: *mut lv_obj_t, angle: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetRotation {
                obj: obj as usize,
                angle,
            })
        });
    }
    pub unsafe fn lv_image_set_pivot(obj: *mut lv_obj_t, x: i32, y: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetPivot {
                obj: obj as usize,
                x,
                y,
            })
        });
    }
    pub unsafe fn lv_image_set_inner_align(obj: *mut lv_obj_t, align: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageSetInnerAlign {
                obj: obj as usize,
                align,
            })
        });
    }

    // ---------------------------------------------------------
    // Image button widget
    // ---------------------------------------------------------
    pub unsafe fn lv_imagebutton_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::ImageButtonCreate { obj: obj as usize })
        });
        obj
    }
    pub unsafe fn lv_imagebutton_set_src(
        obj: *mut lv_obj_t,
        state: u32,
        _src_left: *const core::ffi::c_void,
        src_mid: *const core::ffi::c_void,
        _src_right: *const core::ffi::c_void,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ImageButtonSetSrc {
                obj: obj as usize,
                state,
                src: src_mid as usize,
            })
        });
    }

    // ---------------------------------------------------------
    // Style — background image (mocked setters record spy calls)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_style_bg_image_src(
        obj: *mut lv_obj_t,
        value: *const core::ffi::c_void,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageSrc {
                obj: obj as usize,
                src: value as usize,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_bg_image_tiled(obj: *mut lv_obj_t, value: bool, _selector: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageTiled {
                obj: obj as usize,
                tiled: value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_bg_image_opa(obj: *mut lv_obj_t, value: u8, _selector: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageOpa {
                obj: obj as usize,
                opa: value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_bg_image_recolor(
        obj: *mut lv_obj_t,
        value: lv_color_t,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageRecolor {
                obj: obj as usize,
                color: value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_bg_image_recolor_opa(
        obj: *mut lv_obj_t,
        value: u8,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageRecolorOpa {
                obj: obj as usize,
                opa: value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_image_recolor(
        obj: *mut lv_obj_t,
        value: lv_color_t,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleImageRecolor {
                obj: obj as usize,
                color: value,
            })
        });
    }
    pub unsafe fn lv_obj_set_style_image_recolor_opa(
        obj: *mut lv_obj_t,
        value: u8,
        _selector: u32,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleImageRecolorOpa {
                obj: obj as usize,
                opa: value,
            })
        });
    }

    // ---------------------------------------------------------
    // QR code
    // ---------------------------------------------------------
    /// `lv_result_t_LV_RESULT_OK` value — matches bindgen output for LVGL C enum (1)
    pub const lv_result_t_LV_RESULT_OK: u32 = 1;

    pub unsafe fn lv_qrcode_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::QrCodeCreate { obj: obj as usize })
        });
        obj
    }
    pub unsafe fn lv_qrcode_set_size(obj: *mut lv_obj_t, size: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::QrCodeSetSize {
                obj: obj as usize,
                size,
            })
        });
    }
    pub unsafe fn lv_qrcode_set_dark_color(obj: *mut lv_obj_t, color: lv_color_t) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::QrCodeSetDarkColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_qrcode_set_light_color(obj: *mut lv_obj_t, color: lv_color_t) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::QrCodeSetLightColor {
                obj: obj as usize,
                color,
            })
        });
    }
    pub unsafe fn lv_qrcode_update(
        obj: *mut lv_obj_t,
        _data: *const core::ffi::c_void,
        data_len: u32,
    ) -> u32 {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::QrCodeUpdate {
                obj: obj as usize,
                data_len,
            })
        });
        lv_result_t_LV_RESULT_OK
    }

    // ---------------------------------------------------------
    // Keyboard widget
    // ---------------------------------------------------------
    pub unsafe fn lv_keyboard_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::KeyboardCreate { obj: obj as usize })
        });
        obj
    }
    pub unsafe fn lv_keyboard_set_mode(obj: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::KeyboardSetMode {
                obj: obj as usize,
                mode,
            })
        });
    }
    pub unsafe fn lv_keyboard_set_textarea(obj: *mut lv_obj_t, ta: *mut lv_obj_t) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::KeyboardSetTextarea {
                obj: obj as usize,
                ta: ta as usize,
            })
        });
    }
    pub unsafe fn lv_keyboard_get_textarea(_obj: *mut lv_obj_t) -> *mut lv_obj_t {
        core::ptr::null_mut()
    }
    pub unsafe fn lv_keyboard_set_map(
        obj: *mut lv_obj_t,
        mode: u32,
        map: *const *const core::ffi::c_char,
        _ctrl_map: *const u32,
    ) {
        // Persist the map into BUTTONMATRIX_MAPS so subsequent calls to
        // `lv_buttonmatrix_get_button_text` (used e.g. by
        // `Keyboard::set_continue_enabled`) can resolve key labels in tests.
        if !map.is_null() {
            let (pointers, _labels) = unsafe { read_buttonmatrix_map(map) };
            BUTTONMATRIX_MAPS.with(|m| {
                m.borrow_mut().insert(obj as usize, pointers);
            });
        }
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::KeyboardSetMap {
                obj: obj as usize,
                mode,
            })
        });
    }

    const LV_BUTTONMATRIX_BUTTON_NONE: u32 = 0xFFFF;

    unsafe fn read_buttonmatrix_map(
        map: *const *const core::ffi::c_char,
    ) -> (Vec<*const core::ffi::c_char>, Vec<Vec<u8>>) {
        let mut pointers = Vec::new();
        let mut labels = Vec::new();
        let mut index = 0usize;

        loop {
            let ptr = unsafe { *map.add(index) };
            let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes_with_nul().to_vec();
            pointers.push(ptr);
            labels.push(bytes.clone());
            index += 1;
            if bytes == b"\0" {
                break;
            }
        }

        (pointers, labels)
    }

    pub unsafe fn lv_buttonmatrix_get_selected_button(obj: *mut lv_obj_t) -> u32 {
        let ret = BUTTONMATRIX_SELECTED.with(|m| {
            m.borrow()
                .get(&(obj as usize))
                .copied()
                .unwrap_or(LV_BUTTONMATRIX_BUTTON_NONE)
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixGetSelectedButton {
                obj: obj as usize,
                ret,
            })
        });
        ret
    }

    pub unsafe fn lv_buttonmatrix_get_button_text(
        obj: *const lv_obj_t,
        btn_id: u32,
    ) -> *const core::ffi::c_char {
        let obj_key = obj as usize;
        let ptr = BUTTONMATRIX_MAPS.with(|maps| {
            let maps = maps.borrow();
            let Some(entries) = maps.get(&obj_key) else {
                return core::ptr::null();
            };

            let mut logical_id = 0u32;
            for entry in entries {
                let bytes = unsafe { CStr::from_ptr(*entry) }.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                if bytes == b"\n" {
                    continue;
                }
                if logical_id == btn_id {
                    return *entry;
                }
                logical_id += 1;
            }

            core::ptr::null()
        });

        let text = if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr) }.to_bytes_with_nul().to_vec())
        };
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixGetButtonText {
                obj: obj_key,
                btn_id,
                text,
            })
        });
        ptr
    }

    pub unsafe fn lv_buttonmatrix_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }

    pub unsafe fn lv_buttonmatrix_set_map(
        obj: *mut lv_obj_t,
        map: *const *const core::ffi::c_char,
    ) {
        let (pointers, labels) = unsafe { read_buttonmatrix_map(map) };
        BUTTONMATRIX_MAPS.with(|m| {
            m.borrow_mut().insert(obj as usize, pointers);
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetMap {
                obj: obj as usize,
                labels,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_set_ctrl_map(obj: *mut lv_obj_t, ctrl_map: *const u32) {
        let button_count = BUTTONMATRIX_MAPS.with(|maps| {
            maps.borrow()
                .get(&(obj as usize))
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| {
                            let bytes = unsafe { CStr::from_ptr(**entry) }.to_bytes();
                            !bytes.is_empty() && bytes != b"\n"
                        })
                        .count()
                })
                .unwrap_or(0)
        });

        let ctrl = unsafe { core::slice::from_raw_parts(ctrl_map, button_count) }.to_vec();
        BUTTONMATRIX_CTRLS.with(|m| {
            m.borrow_mut().insert(obj as usize, ctrl.clone());
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetCtrlMap {
                obj: obj as usize,
                ctrl,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_set_button_width(obj: *mut lv_obj_t, btn_id: u32, width: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetButtonWidth {
                obj: obj as usize,
                btn_id,
                width,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_set_button_ctrl(obj: *mut lv_obj_t, btn_id: u32, ctrl: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetButtonCtrl {
                obj: obj as usize,
                btn_id,
                ctrl,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_clear_button_ctrl(obj: *mut lv_obj_t, btn_id: u32, ctrl: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixClearButtonCtrl {
                obj: obj as usize,
                btn_id,
                ctrl,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_set_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetButtonCtrlAll {
                obj: obj as usize,
                ctrl,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_clear_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixClearButtonCtrlAll {
                obj: obj as usize,
                ctrl,
            })
        });
    }

    pub unsafe fn lv_buttonmatrix_set_one_checked(obj: *mut lv_obj_t, en: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetOneChecked {
                obj: obj as usize,
                en,
            })
        });
    }

    // ---------------------------------------------------------
    // Object positioning & geometry
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_pos(obj: *mut lv_obj_t, x: i32, y: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetPos {
                obj: obj as usize,
                x,
                y,
            })
        });
    }
    pub unsafe fn lv_obj_get_x(_obj: *mut lv_obj_t) -> i32 {
        0
    }
    pub unsafe fn lv_obj_get_y(_obj: *mut lv_obj_t) -> i32 {
        0
    }
    pub unsafe fn lv_obj_get_width(_obj: *mut lv_obj_t) -> i32 {
        100
    }
    pub unsafe fn lv_obj_get_parent(_obj: *mut lv_obj_t) -> *mut lv_obj_t {
        core::ptr::null_mut()
    }

    // ---------------------------------------------------------
    // TextArea widget
    // ---------------------------------------------------------
    pub unsafe fn lv_textarea_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        register_child(parent, obj);
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaCreate {
                obj: obj as usize,
                parent: parent as usize,
            })
        });
        obj
    }
    pub unsafe fn lv_textarea_set_placeholder_text(
        obj: *mut lv_obj_t,
        txt: *const core::ffi::c_char,
    ) {
        // SAFETY: caller guarantees a valid NUL-terminated buffer.
        let bytes = unsafe { CStr::from_ptr(txt) }.to_bytes_with_nul().to_vec();
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaSetPlaceholder {
                obj: obj as usize,
                text: bytes,
            })
        });
    }
    pub unsafe fn lv_textarea_set_max_length(obj: *mut lv_obj_t, num: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaSetMaxLength {
                obj: obj as usize,
                max: num,
            })
        });
    }
    pub unsafe fn lv_textarea_set_one_line(obj: *mut lv_obj_t, en: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaSetOneLine {
                obj: obj as usize,
                en,
            })
        });
    }
    pub unsafe fn lv_textarea_set_password_mode(obj: *mut lv_obj_t, en: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaSetPasswordMode {
                obj: obj as usize,
                en,
            })
        });
    }
    pub unsafe fn lv_textarea_set_text(obj: *mut lv_obj_t, txt: *const core::ffi::c_char) {
        // SAFETY: caller guarantees a valid NUL-terminated buffer.
        let cstr = unsafe { CStr::from_ptr(txt) };
        let owned = alloc::ffi::CString::from(cstr);
        let bytes = owned.as_bytes_with_nul().to_vec();
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TextAreaSetText {
                obj: obj as usize,
                text: bytes,
            })
        });
        TEXTAREA_TEXT.with(|m| {
            m.borrow_mut().insert(obj as usize, owned);
        });
    }
    pub unsafe fn lv_textarea_get_text(obj: *mut lv_obj_t) -> *const core::ffi::c_char {
        TEXTAREA_TEXT.with(|m| {
            m.borrow()
                .get(&(obj as usize))
                .map(|s| s.as_ptr())
                .unwrap_or_else(|| {
                    // LVGL returns a stable empty string, not null. Use a static C "".
                    static EMPTY: &[u8] = b"\0";
                    EMPTY.as_ptr() as *const core::ffi::c_char
                })
        })
    }
    pub unsafe fn lv_textarea_delete_char(_obj: *mut lv_obj_t) {}
    pub unsafe fn lv_textarea_add_char(_obj: *mut lv_obj_t, _c: u32) {}
    pub unsafe fn lv_textarea_add_text(_obj: *mut lv_obj_t, _txt: *const core::ffi::c_char) {}
    pub unsafe fn lv_textarea_cursor_left(_obj: *mut lv_obj_t) {}
    pub unsafe fn lv_textarea_cursor_right(_obj: *mut lv_obj_t) {}

    // ---------------------------------------------------------
    // Object event management
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_send_event(
        _obj: *mut lv_obj_t,
        _event: u32,
        _param: *mut core::ffi::c_void,
    ) -> u32 {
        lv_result_t_LV_RESULT_OK
    }
    pub unsafe fn lv_obj_remove_event_cb(
        obj: *mut lv_obj_t,
        cb: Option<unsafe extern "C" fn(*mut lv_event_t)>,
    ) -> bool {
        EVENT_REG.with(|m| {
            if let Some(v) = m.borrow_mut().get_mut(&(obj as usize)) {
                v.retain(|r| r.cb.map(|f| f as usize) != cb.map(|f| f as usize));
            }
        });
        true
    }
    pub unsafe extern "C" fn lv_keyboard_def_event_cb(_e: *mut lv_event_t) {}

    // ---------------------------------------------------------
    // Keyboard popovers
    // ---------------------------------------------------------
    pub unsafe fn lv_keyboard_set_popovers(obj: *mut lv_obj_t, en: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ButtonMatrixSetPopovers {
                obj: obj as usize,
                en,
            })
        });
    }

    // ---------------------------------------------------------
    // Delete
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_delete(obj: *mut lv_obj_t) {
        let children_to_delete =
            CHILDREN.with(|m| m.borrow().get(&(obj as usize)).cloned().unwrap_or_default());
        for child in children_to_delete {
            unsafe {
                lv_obj_delete(child as *mut lv_obj_t);
            }
        }

        spy_emit_event(obj, LV_EVENT_DELETE);

        CHILDREN.with(|m| {
            let mut children = m.borrow_mut();
            children.remove(&(obj as usize));
            for list in children.values_mut() {
                list.retain(|child| *child != obj as usize);
            }
        });
        EVENT_REG.with(|m| {
            m.borrow_mut().remove(&(obj as usize));
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjDelete { obj: obj as usize }));
    }

    // ---------------------------------------------------------
    // Style objects (all no-ops in mock)
    // ---------------------------------------------------------
    pub unsafe fn lv_style_init(_style: *mut lv_style_t) {}
    pub unsafe fn lv_style_reset(_style: *mut lv_style_t) {}
    pub unsafe fn lv_obj_add_style(_obj: *mut lv_obj_t, _style: *const lv_style_t, _selector: u32) {
    }
    pub unsafe fn lv_style_set_bg_color(_style: *mut lv_style_t, _value: lv_color_t) {}
    pub unsafe fn lv_style_set_bg_opa(_style: *mut lv_style_t, _value: u8) {}
    pub unsafe fn lv_style_set_text_color(_style: *mut lv_style_t, _value: lv_color_t) {}
    pub unsafe fn lv_style_set_text_font(_style: *mut lv_style_t, _value: *const lv_font_t) {}
    pub unsafe fn lv_style_set_border_color(_style: *mut lv_style_t, _value: lv_color_t) {}
    pub unsafe fn lv_style_set_border_width(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_radius(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_opa(_style: *mut lv_style_t, _value: u8) {}
    pub unsafe fn lv_style_set_width(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_height(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_pad_top(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_pad_bottom(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_pad_left(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_pad_right(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_bg_image_src(
        style: *mut lv_style_t,
        value: *const core::ffi::c_void,
    ) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageSrc {
                obj: style as usize,
                src: value as usize,
            })
        });
    }
    pub unsafe fn lv_style_set_bg_image_opa(style: *mut lv_style_t, value: u8) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageOpa {
                obj: style as usize,
                opa: value,
            })
        });
    }
    pub unsafe fn lv_style_set_bg_image_tiled(style: *mut lv_style_t, value: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::SetStyleBgImageTiled {
                obj: style as usize,
                tiled: value,
            })
        });
    }
    pub unsafe fn lv_style_set_pad_row(_style: *mut lv_style_t, _value: i32) {}
    pub unsafe fn lv_style_set_pad_column(_style: *mut lv_style_t, _value: i32) {}

    // ---------------------------------------------------------
    // Theme (no-op in mock — theme init has no visible effect)
    // ---------------------------------------------------------
    pub unsafe fn lv_theme_default_init(
        _disp: *mut lv_display_t,
        _color_primary: lv_color_t,
        _color_secondary: lv_color_t,
        _dark: bool,
        _font: *const lv_font_t,
    ) -> *mut lv_theme_t {
        core::ptr::null_mut()
    }

    // ---------------------------------------------------------
    // Color helpers (return default color — not tested)
    // ---------------------------------------------------------
    pub unsafe fn lv_color_hex(val: u32) -> lv_color_t {
        lv_color_t {
            red: ((val >> 16) & 0xFF) as u8,
            green: ((val >> 8) & 0xFF) as u8,
            blue: (val & 0xFF) as u8,
        }
    }
    pub unsafe fn lv_color_make(_r: u8, _g: u8, _b: u8) -> lv_color_t {
        lv_color_t::default()
    }
    pub unsafe fn lv_color_white() -> lv_color_t {
        lv_color_t::default()
    }
    pub unsafe fn lv_color_black() -> lv_color_t {
        lv_color_t::default()
    }
    pub unsafe fn lv_palette_main(_p: u32) -> lv_color_t {
        lv_color_t::default()
    }
    pub unsafe fn lv_palette_lighten(_p: u32, _level: u8) -> lv_color_t {
        lv_color_t::default()
    }
    pub unsafe fn lv_palette_darken(_p: u32, _level: u8) -> lv_color_t {
        lv_color_t::default()
    }

    // ---------------------------------------------------------
    // Size helper
    // ---------------------------------------------------------
    pub unsafe fn lv_pct(v: i32) -> i32 {
        v
    }

    // ---------------------------------------------------------
    // Event accessors
    // ---------------------------------------------------------
    pub unsafe fn lv_event_get_user_data(_e: *mut lv_event_t) -> *mut core::ffi::c_void {
        CURRENT_EVENT.with(|c| c.get().2) as *mut core::ffi::c_void
    }
    pub unsafe fn lv_event_get_code(_e: *mut lv_event_t) -> u32 {
        CURRENT_EVENT.with(|c| c.get().1)
    }
    pub unsafe fn lv_event_get_target(_e: *mut lv_event_t) -> *mut core::ffi::c_void {
        CURRENT_EVENT.with(|c| c.get().0) as *mut core::ffi::c_void
    }

    // ---------------------------------------------------------
    // Y-coordinate helpers (used by slide animations)
    // ---------------------------------------------------------
    pub unsafe fn lv_obj_set_y(_obj: *mut lv_obj_t, _y: i32) {}
    pub unsafe fn lv_obj_get_height(_obj: *mut lv_obj_t) -> i32 {
        480
    }

    // ---------------------------------------------------------
    // Animation API
    // ---------------------------------------------------------
    pub unsafe fn lv_anim_init(a: *mut lv_anim_t) {
        core::ptr::write_bytes(a as *mut u8, 0, 256);
        // Match LVGL's zero-initialized semantics: any previous user_data
        // tracked out-of-band for this same pointer (e.g. when a stack slot
        // gets reused for a new animation) must not leak into the new anim.
        ANIM_USER_DATA.with(|m| { m.borrow_mut().remove(&(a as usize)); });
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimInit));
    }
    pub unsafe fn lv_anim_set_var(_a: *mut lv_anim_t, var: *mut core::ffi::c_void) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetVar { var: var as usize }));
    }
    pub unsafe fn lv_anim_set_exec_cb(
        _a: *mut lv_anim_t,
        cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)>,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetExecCb { cb }));
    }
    pub unsafe fn lv_anim_set_values(_a: *mut lv_anim_t, start: i32, end: i32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetValues { start, end }));
    }
    pub unsafe fn lv_anim_set_duration(_a: *mut lv_anim_t, duration: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetDuration { ms: duration }));
    }
    pub unsafe fn lv_anim_set_path_cb(
        _a: *mut lv_anim_t,
        cb: Option<unsafe extern "C" fn(*const lv_anim_t) -> i32>,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetPathCb { cb }));
    }
    pub unsafe fn lv_anim_set_completed_cb(
        _a: *mut lv_anim_t,
        cb: Option<unsafe extern "C" fn(*mut lv_anim_t)>,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetCompletedCb { cb }));
    }
    pub unsafe fn lv_anim_set_repeat_count(_a: *mut lv_anim_t, count: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetRepeatCount { count }));
    }
    pub unsafe fn lv_anim_start(a: *const lv_anim_t) -> *mut lv_anim_t {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimStart));
        a as *mut lv_anim_t
    }
    pub unsafe fn lv_anim_delete(
        var: *mut core::ffi::c_void,
        exec_cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)>,
    ) -> u32 {
        SPY.with(|s| s.borrow_mut().push(LvCall::AnimDelete { var: var as usize, cb: exec_cb }));
        0
    }
    pub unsafe extern "C" fn lv_anim_path_ease_in(_a: *const lv_anim_t) -> i32 {
        0
    }
    pub unsafe extern "C" fn lv_anim_path_ease_out(_a: *const lv_anim_t) -> i32 {
        0
    }
    pub unsafe extern "C" fn lv_anim_path_linear(_a: *const lv_anim_t) -> i32 { 0 }
    pub unsafe extern "C" fn lv_anim_path_ease_in_out(_a: *const lv_anim_t) -> i32 { 0 }
    pub unsafe extern "C" fn lv_anim_path_overshoot(_a: *const lv_anim_t) -> i32 { 0 }
    pub unsafe extern "C" fn lv_anim_path_bounce(_a: *const lv_anim_t) -> i32 { 0 }
    pub unsafe extern "C" fn lv_anim_path_step(_a: *const lv_anim_t) -> i32 { 0 }
    // Per-anim user_data, keyed by the `lv_anim_t` pointer cast to usize, so
    // multiple animations don't clobber each other's `user_data` value.
    thread_local! {
        static ANIM_USER_DATA: core::cell::RefCell<
            std::collections::HashMap<usize, *mut core::ffi::c_void>,
        > = core::cell::RefCell::new(std::collections::HashMap::new());
    }
    pub unsafe fn lv_anim_set_user_data(a: *mut lv_anim_t, user_data: *mut core::ffi::c_void) {
        ANIM_USER_DATA.with(|m| { m.borrow_mut().insert(a as usize, user_data); });
    }
    pub unsafe fn lv_anim_get_user_data(a: *const lv_anim_t) -> *mut core::ffi::c_void {
        ANIM_USER_DATA.with(|m| {
            m.borrow().get(&(a as usize)).copied().unwrap_or(core::ptr::null_mut())
        })
    }

    // ---------------------------------------------------------
    // SearchBar (§8) shim implementations
    // ---------------------------------------------------------

    // -------- Timers --------
    pub unsafe fn lv_timer_create(
        cb: Option<unsafe extern "C" fn(*mut lv_timer_t)>,
        period_ms: u32,
        user_data: *mut c_void,
    ) -> *mut lv_timer_t {
        let handle = NEXT_TIMER_HANDLE.with(|c| {
            let h = c.get();
            c.set(h + 8);
            h
        });
        TIMER_REG.with(|m| {
            m.borrow_mut().insert(
                handle,
                TimerReg {
                    period_ms,
                    repeat_count: -1,
                    cb,
                    user_data,
                    paused: false,
                },
            );
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TimerCreate {
                handle,
                period_ms,
                user_data: user_data as usize,
            })
        });
        handle as *mut lv_timer_t
    }
    pub unsafe fn lv_timer_get_user_data(t: *mut lv_timer_t) -> *mut c_void {
        TIMER_REG.with(|m| {
            m.borrow()
                .get(&(t as usize))
                .map(|r| r.user_data)
                .unwrap_or(core::ptr::null_mut())
        })
    }
    pub unsafe fn lv_timer_set_period(t: *mut lv_timer_t, period_ms: u32) {
        TIMER_REG.with(|m| {
            if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
                tr.period_ms = period_ms;
            }
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TimerSetPeriod {
                handle: t as usize,
                period_ms,
            })
        });
    }
    pub unsafe fn lv_timer_reset(t: *mut lv_timer_t) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::TimerReset { handle: t as usize })
        });
    }
    pub unsafe fn lv_timer_set_repeat_count(t: *mut lv_timer_t, count: i32) {
        TIMER_REG.with(|m| {
            if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
                tr.repeat_count = count;
            }
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::TimerSetRepeatCount {
                handle: t as usize,
                count,
            })
        });
    }
    pub unsafe fn lv_timer_pause(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| {
            if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
                tr.paused = true;
            }
        });
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::TimerPause { handle: t as usize })
        });
    }
    pub unsafe fn lv_timer_resume(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| {
            if let Some(tr) = m.borrow_mut().get_mut(&(t as usize)) {
                tr.paused = false;
            }
        });
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::TimerResume { handle: t as usize })
        });
    }
    pub unsafe fn lv_timer_delete(t: *mut lv_timer_t) {
        TIMER_REG.with(|m| {
            m.borrow_mut().remove(&(t as usize));
        });
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::TimerDelete { handle: t as usize })
        });
    }

    // -------- Scroll geometry --------
    pub unsafe fn lv_obj_get_scroll_bottom(obj: *mut lv_obj_t) -> i32 {
        let v = NEXT_SCROLL_BOTTOM.with(|c| c.get());
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjGetScrollBottom {
                obj: obj as usize,
                ret: v,
            })
        });
        v
    }
    pub unsafe fn lv_obj_get_scroll_top(obj: *mut lv_obj_t) -> i32 {
        let v = NEXT_SCROLL_TOP.with(|c| c.get());
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjGetScrollTop {
                obj: obj as usize,
                ret: v,
            })
        });
        v
    }
    pub unsafe fn lv_obj_set_scrollbar_mode(obj: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetScrollbarMode {
                obj: obj as usize,
                mode,
            })
        });
    }
    pub unsafe fn lv_obj_scroll_to_view(obj: *mut lv_obj_t, anim: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjScrollToView {
                obj: obj as usize,
                anim,
            })
        });
    }
    /// Test stub: synchronously invokes the callback. The production binding
    /// (cfg(feature = "desktop")) defers to LVGL's `lv_async_call`; tests
    /// assert behaviour by running the callback inline so we don't need to
    /// pump a fake `lv_timer_handler`.
    pub unsafe fn lv_async_call(
        cb: Option<unsafe extern "C" fn(*mut c_void)>,
        user_data: *mut c_void,
    ) -> i32 {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::AsyncCall {
                user_data: user_data as usize,
            })
        });
        if let Some(f) = cb {
            unsafe {
                f(user_data);
            }
        }
        0
    }

    // -------- User data --------
    pub unsafe fn lv_obj_set_user_data(obj: *mut lv_obj_t, ud: *mut c_void) {
        USER_DATA.with(|m| {
            m.borrow_mut().insert(obj as usize, ud as usize);
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjSetUserData {
                obj: obj as usize,
                data: ud as usize,
            })
        });
    }
    pub unsafe fn lv_obj_get_user_data(obj: *mut lv_obj_t) -> *mut c_void {
        let v = USER_DATA.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0));
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjGetUserData {
                obj: obj as usize,
                ret: v,
            })
        });
        v as *mut c_void
    }

    // -------- Label extras --------
    pub unsafe fn lv_label_set_long_mode(label: *mut lv_obj_t, mode: u32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::LabelSetLongMode {
                label: label as usize,
                mode,
            })
        });
    }
    pub unsafe fn lv_label_set_recolor(label: *mut lv_obj_t, en: bool) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::LabelSetRecolor {
                label: label as usize,
                en,
            })
        });
    }

    pub unsafe fn lv_obj_clean(obj: *mut lv_obj_t) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjClean { obj: obj as usize }));
        // Tests don't rely on actual child removal; the label create count
        // in the spy is what matters.
    }

    // -------- Children & focus --------
    pub unsafe fn lv_obj_get_child_count(obj: *mut lv_obj_t) -> u32 {
        let tracked = CHILDREN.with(|m| m.borrow().get(&(obj as usize)).map(|v| v.len() as u32));
        let v = tracked.unwrap_or_else(|| {
            CHILD_COUNTS.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0))
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjGetChildCount {
                obj: obj as usize,
                ret: v,
            })
        });
        v
    }
    pub unsafe fn lv_obj_get_child(obj: *mut lv_obj_t, idx: i32) -> *mut lv_obj_t {
        let ret = CHILDREN.with(|m| {
            m.borrow()
                .get(&(obj as usize))
                .and_then(|children| {
                    usize::try_from(idx)
                        .ok()
                        .and_then(|idx| children.get(idx).copied())
                })
                .unwrap_or(0)
        });
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjGetChild {
                obj: obj as usize,
                idx,
                ret,
            })
        });
        ret as *mut lv_obj_t
    }
    pub unsafe fn lv_group_focus_obj(obj: *mut lv_obj_t) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::GroupFocusObj { obj: obj as usize })
        });
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
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::RemoveEventCbWithUserData {
                obj: obj as usize,
                user_data: user_data as usize,
            })
        });
    }

    // ---------------------------------------------------------
    // Input device API — mock
    // ---------------------------------------------------------
    /// Mock-only: which `lv_indev_t` pointer the next `lv_indev_active()`
    /// call should return. Defaults to a non-null sentinel so callers'
    /// null-checks pass during tests; set to null to simulate "no active
    /// indev" (e.g. event fired outside a real touch).
    thread_local! {
        static MOCK_ACTIVE_INDEV: Cell<*mut lv_indev_t> =
            Cell::new(0x1000_0000 as *mut lv_indev_t);
    }

    /// Test helper: override the pointer returned by `lv_indev_active`.
    pub fn set_active_indev_for_test(p: *mut lv_indev_t) {
        MOCK_ACTIVE_INDEV.with(|c| c.set(p));
    }

    pub unsafe fn lv_indev_active() -> *mut lv_indev_t {
        SPY.with(|s| s.borrow_mut().push(LvCall::IndevActive));
        MOCK_ACTIVE_INDEV.with(|c| c.get())
    }

    pub unsafe fn lv_indev_wait_release(indev: *mut lv_indev_t) {
        SPY.with(|s| {
            s.borrow_mut()
                .push(LvCall::IndevWaitRelease { indev: indev as usize })
        });
    }

    pub unsafe fn lv_obj_move_to_index(obj: *mut lv_obj_t, index: i32) {
        SPY.with(|s| {
            s.borrow_mut().push(LvCall::ObjMoveToIndex {
                obj: obj as usize,
                index,
            })
        });
    }
}

#[cfg(any(test, all(no_zephyr, not(desktop_sim))))]
pub use mock::*;

// ============================================================
//  Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task1_new_symbols_referenced() {
        // This test exists so that any "must add" symbol from spec §8 missing
        // from bindings.conf produces a compile error rather than a runtime
        // surprise. Each function reference forces bindgen to have emitted it
        // for the Zephyr build, and forces our desktop-sim shim to declare it.
        use crate::c_bindings::*;
        let _ = lv_timer_create as unsafe fn(_, _, _) -> *mut lv_timer_t;
        let _ = lv_timer_set_period as unsafe fn(*mut lv_timer_t, u32);
        let _ = lv_timer_reset as unsafe fn(*mut lv_timer_t);
        let _ = lv_timer_set_repeat_count as unsafe fn(*mut lv_timer_t, i32);
        let _ = lv_timer_pause as unsafe fn(*mut lv_timer_t);
        let _ = lv_timer_resume as unsafe fn(*mut lv_timer_t);
        let _ = lv_timer_delete as unsafe fn(*mut lv_timer_t);
        let _ = lv_obj_get_scroll_bottom as unsafe fn(*mut lv_obj_t) -> i32;
        let _ = lv_obj_get_scroll_top as unsafe fn(*mut lv_obj_t) -> i32;
        let _ = lv_obj_set_scrollbar_mode as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_obj_scroll_to_view as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_async_call
            as unsafe fn(
                Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
                *mut core::ffi::c_void,
            ) -> i32;
        let _ = lv_obj_set_user_data as unsafe fn(*mut lv_obj_t, *mut core::ffi::c_void);
        let _ = lv_obj_get_user_data as unsafe fn(*mut lv_obj_t) -> *mut core::ffi::c_void;
        let _ = lv_label_set_long_mode as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_label_set_recolor as unsafe fn(*mut lv_obj_t, bool);
        let _ = lv_obj_get_child_count as unsafe fn(*mut lv_obj_t) -> u32;
        let _ = lv_obj_remove_event_cb_with_user_data
            as unsafe fn(
                *mut lv_obj_t,
                Option<unsafe extern "C" fn(*mut lv_event_t)>,
                *mut core::ffi::c_void,
            );
        let _ = lv_group_focus_obj as unsafe fn(*mut lv_obj_t);
        let _ = lv_buttonmatrix_set_ctrl_map as unsafe fn(*mut lv_obj_t, *const u32);
        let _ = lv_buttonmatrix_set_button_width as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_set_button_ctrl as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_clear_button_ctrl as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_set_button_ctrl_all as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_buttonmatrix_clear_button_ctrl_all as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_buttonmatrix_set_one_checked as unsafe fn(*mut lv_obj_t, bool);

        let _ = lv_spinner_set_anim_params as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_obj_get_child as unsafe fn(*mut lv_obj_t, i32) -> *mut lv_obj_t;
        let _ = lv_obj_set_style_transform_rotation as unsafe fn(*mut lv_obj_t, i32, u32);
        let _ = lv_anim_set_repeat_count as unsafe fn(*mut lv_anim_t, u32);
        let _ = lv_anim_delete as unsafe fn(
            *mut core::ffi::c_void,
            Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32)>,
        ) -> u32;
        let _ = lv_obj_delete as unsafe fn(*mut lv_obj_t);
        let _ = LV_ANIM_REPEAT_INFINITE;
        let _ = lv_arc_create as unsafe fn(*mut lv_obj_t) -> *mut lv_obj_t;
        let _ = lv_arc_set_range as unsafe fn(*mut lv_obj_t, i32, i32);
        let _ = lv_arc_set_value as unsafe fn(*mut lv_obj_t, i32);
        let _ = lv_arc_get_value as unsafe fn(*mut lv_obj_t) -> i32;
        let _ = lv_arc_set_bg_angles as unsafe fn(*mut lv_obj_t, u16, u16);
        let _ = lv_arc_set_angles as unsafe fn(*mut lv_obj_t, u16, u16);
        let _ = lv_arc_set_rotation as unsafe fn(*mut lv_obj_t, u16);
        let _ = lv_arc_set_mode as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_arc_set_change_rate as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_obj_remove_style_all as unsafe fn(*mut lv_obj_t);
        let _ = lv_obj_set_style_arc_color as unsafe fn(*mut lv_obj_t, lv_color_t, u32);
        let _ = lv_obj_set_style_arc_width as unsafe fn(*mut lv_obj_t, i32, u32);
        let _ = lv_obj_set_style_arc_opa as unsafe fn(*mut lv_obj_t, u8, u32);
        let _ = lv_obj_set_style_arc_rounded as unsafe fn(*mut lv_obj_t, bool, u32);
        let _ = lv_anim_path_linear as unsafe extern "C" fn(*const lv_anim_t) -> i32;
        let _ = lv_anim_path_ease_in_out as unsafe extern "C" fn(*const lv_anim_t) -> i32;
        let _ = lv_anim_path_overshoot as unsafe extern "C" fn(*const lv_anim_t) -> i32;
        let _ = lv_anim_path_bounce as unsafe extern "C" fn(*const lv_anim_t) -> i32;
        let _ = lv_anim_path_step as unsafe extern "C" fn(*const lv_anim_t) -> i32;
        let _ = lv_anim_set_user_data as unsafe fn(*mut lv_anim_t, *mut core::ffi::c_void);
        let _ = lv_anim_get_user_data as unsafe fn(*const lv_anim_t) -> *mut core::ffi::c_void;
        let _ = lv_indev_active as unsafe fn() -> *mut lv_indev_t;
        let _ = lv_indev_wait_release as unsafe fn(*mut lv_indev_t);
        let _ = lv_obj_move_to_index as unsafe fn(*mut lv_obj_t, i32);
    }

    #[test]
    fn indev_wrapper_symbols_are_allowlisted_for_bindgen() {
        let bindings_conf = std::fs::read_to_string("src/lvgl/bindings.conf")
            .expect("src/lvgl/bindings.conf should be readable");
        for symbol in ["lv_indev_active", "lv_indev_wait_release"] {
            assert!(
                bindings_conf.contains(symbol),
                "{symbol} must be listed in src/lvgl/bindings.conf for Zephyr bindgen builds"
            );
        }
    }

    #[test]
    fn obj_move_to_index_is_allowlisted_for_bindgen() {
        let bindings_conf = std::fs::read_to_string("src/lvgl/bindings.conf")
            .expect("src/lvgl/bindings.conf should be readable");
        assert!(
            bindings_conf.contains("lv_obj_move_to_index"),
            "lv_obj_move_to_index must be listed in src/lvgl/bindings.conf for Zephyr bindgen builds"
        );
    }

    #[test]
    fn buttonmatrix_wrapper_symbols_are_allowlisted_for_bindgen() {
        let bindings_conf = std::fs::read_to_string("src/lvgl/bindings.conf")
            .expect("src/lvgl/bindings.conf should be readable");
        for symbol in [
            "lv_buttonmatrix_create",
            "lv_buttonmatrix_set_map",
            "lv_buttonmatrix_set_ctrl_map",
            "lv_buttonmatrix_set_button_width",
            "lv_buttonmatrix_set_button_ctrl",
            "lv_buttonmatrix_clear_button_ctrl",
            "lv_buttonmatrix_set_button_ctrl_all",
            "lv_buttonmatrix_clear_button_ctrl_all",
            "lv_buttonmatrix_set_one_checked",
            "lv_buttonmatrix_get_selected_button",
            "lv_buttonmatrix_get_button_text",
        ] {
            assert!(
                bindings_conf.contains(symbol),
                "{symbol} must be listed in src/lvgl/bindings.conf for Zephyr bindgen builds"
            );
        }
    }

    #[test]
    fn loading_wrapper_symbols_are_allowlisted_for_bindgen() {
        let bindings_conf = std::fs::read_to_string("src/lvgl/bindings.conf")
            .expect("src/lvgl/bindings.conf should be readable");
        for symbol in [
            "lv_spinner_create",
            "lv_spinner_set_anim_params",
            "lv_obj_get_child",
            "lv_obj_set_style_transform_rotation",
            "lv_anim_set_repeat_count",
            "LV_ANIM_REPEAT_INFINITE",
            "LV_EVENT_DELETE",
        ] {
            assert!(
                bindings_conf.contains(symbol),
                "{symbol} must be listed in src/lvgl/bindings.conf for Zephyr bindgen builds"
            );
        }
    }

    #[test]
    fn image_recolor_symbols_are_allowlisted_for_bindgen() {
        let bindings_conf = std::fs::read_to_string("src/lvgl/bindings.conf")
            .expect("src/lvgl/bindings.conf should be readable");
        for symbol in [
            "lv_obj_set_style_image_recolor",
            "lv_obj_set_style_image_recolor_opa",
        ] {
            assert!(
                bindings_conf.contains(symbol),
                "{symbol} must be listed in src/lvgl/bindings.conf for Zephyr bindgen builds"
            );
        }
    }

    #[test]
    fn task1_event_registry_dispatches() {
        let _fx = SpyFixture::new();
        static FIRES: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        unsafe extern "C" fn cb(_e: *mut lv_event_t) {
            FIRES.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        }
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        unsafe {
            lv_obj_add_event_cb(
                obj,
                Some(cb),
                7, /* arbitrary code */
                core::ptr::null_mut(),
            );
        }
        spy_emit_event(obj, 7);
        spy_emit_event(obj, 8); // wrong code, no fire
        assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn task1_timer_repeat_count_branches() {
        let _fx = SpyFixture::new();
        static FIRES: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        unsafe extern "C" fn cb(_t: *mut lv_timer_t) {
            FIRES.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        }
        let t = unsafe { lv_timer_create(Some(cb), 250, core::ptr::null_mut()) };
        // default repeat_count = -1 (infinity): fires forever
        spy_fire_timer(t);
        spy_fire_timer(t);
        assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 2);
        unsafe {
            lv_timer_set_repeat_count(t, 1);
        }
        spy_fire_timer(t); // fires once, then auto-removes
        spy_fire_timer(t); // no-op
        assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 3);
        assert!(spy_live_timer_handles().is_empty());
    }

    #[test]
    fn task1_user_data_roundtrip() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let ptr: *mut core::ffi::c_void = 0xDEADBEEFusize as _;
        unsafe {
            lv_obj_set_user_data(obj, ptr);
        }
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
        let obj_addr;
        {
            let _fx = SpyFixture::new();
            let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
            obj_addr = obj as usize;
            unsafe {
                lv_obj_set_user_data(obj, 0x1 as *mut _);
            }
            assert!(USER_DATA.with(|m| m.borrow().contains_key(&obj_addr)));
        } // _fx drops here — registries cleared
        assert!(USER_DATA.with(|m| m.borrow().is_empty()));
        assert!(EVENT_REG.with(|m| m.borrow().is_empty()));
        assert!(TIMER_REG.with(|m| m.borrow().is_empty()));
    }

    #[test]
    fn task1_parallel_isolation() {
        // risk #37 — spy state lives in thread_local!; parallel cargo test
        // workers must not corrupt each other.
        let handles: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| {
                    let _fx = SpyFixture::new();
                    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
                    unsafe {
                        lv_obj_set_user_data(obj, 0xAA as *mut _);
                    }
                    let v = unsafe { lv_obj_get_user_data(obj) } as usize;
                    assert_eq!(v, 0xAA);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn parcel_locker_geometry_spy_records_position() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        unsafe {
            lv_obj_set_pos(obj, 12, 34);
        }
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ObjSetPos { obj: recorded, x: 12, y: 34 }
                    if *recorded == obj as usize
            )),
            "expected ObjSetPos for overlay geometry, got: {:?}",
            calls
        );
    }

    #[test]
    fn parcel_locker_style_spy_records_visual_calls() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let green = unsafe { lv_color_hex(0x00AA00) };
        let blue = unsafe { lv_color_hex(0x00AEEF) };

        unsafe {
            lv_obj_set_style_bg_color(obj, green, 0);
            lv_obj_set_style_bg_opa(obj, 80, 0);
            lv_obj_set_style_border_width(obj, 2, 0);
            lv_obj_set_style_outline_color(obj, blue, 0);
            lv_obj_set_style_outline_width(obj, 3, 0);
            lv_obj_set_style_opa(obj, 160, 0);
        }

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleBgColor { obj: recorded, color }
                    if *recorded == obj as usize && *color == green
            )),
            "expected SetStyleBgColor, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleBgOpa { obj: recorded, opa: 80 }
                    if *recorded == obj as usize
            )),
            "expected SetStyleBgOpa, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleBorderWidth { obj: recorded, value: 2 }
                    if *recorded == obj as usize
            )),
            "expected SetStyleBorderWidth, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleOutlineColor { obj: recorded, color }
                    if *recorded == obj as usize && *color == blue
            )),
            "expected SetStyleOutlineColor, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleOutlineWidth { obj: recorded, value: 3 }
                    if *recorded == obj as usize
            )),
            "expected SetStyleOutlineWidth, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleOpa { obj: recorded, opa: 160 }
                    if *recorded == obj as usize
            )),
            "expected SetStyleOpa, got: {:?}",
            calls
        );
    }

    #[test]
    fn loading_bindings_record_spinner_create() {
        let _fx = SpyFixture::new();
        let parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let spinner = unsafe { lv_spinner_create(parent) };

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SpinnerCreate { obj, parent: p }
                    if *obj == spinner as usize && *p == parent as usize
            )),
            "expected SpinnerCreate, got: {:?}",
            calls
        );
    }

    #[test]
    fn mock_object_pool_allocates_unique_object_pointers() {
        let _fx = SpyFixture::new();
        let screen = unsafe { lv_screen_active() };
        let button = unsafe { lv_button_create(screen) };
        let label = unsafe { lv_label_create(button) };

        assert_ne!(screen, button);
        assert_ne!(button, label);
        assert_ne!(screen, label);
    }

    #[test]
    fn reset_obj_pool_clears_tracked_children() {
        let _fx = SpyFixture::new();
        let parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let _child = unsafe { lv_label_create(parent) };
        assert_eq!(unsafe { lv_obj_get_child_count(parent) }, 1);

        reset_obj_pool();

        let reused_parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
        assert_eq!(unsafe { lv_obj_get_child_count(reused_parent) }, 0);
    }

    #[test]
    fn loading_bindings_record_spinner_params_and_children() {
        let _fx = SpyFixture::new();
        let parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let child_a = unsafe { lv_label_create(parent) };
        let child_b = unsafe { lv_spinner_create(parent) };

        unsafe {
            lv_spinner_set_anim_params(child_b, 900, 90);
        }

        assert_eq!(unsafe { lv_obj_get_child_count(parent) }, 2);
        assert_eq!(unsafe { lv_obj_get_child(parent, 0) }, child_a);
        assert_eq!(unsafe { lv_obj_get_child(parent, 1) }, child_b);
        assert!(unsafe { lv_obj_get_child(parent, 2) }.is_null());

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SpinnerSetAnimParams { obj, spin_ms: 900, arc_length_deg: 90 }
                    if *obj == child_b as usize
            )),
            "expected SpinnerSetAnimParams for child_b, got: {:?}",
            calls
        );
    }

    #[test]
    fn mock_child_tracking_covers_parent_taking_widget_creators() {
        let _fx = SpyFixture::new();
        let parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let children = [
            unsafe { lv_dropdown_create(parent) },
            unsafe { lv_imagebutton_create(parent) },
            unsafe { lv_qrcode_create(parent) },
            unsafe { lv_keyboard_create(parent) },
            unsafe { lv_buttonmatrix_create(parent) },
            unsafe { lv_textarea_create(parent) },
        ];

        assert_eq!(
            unsafe { lv_obj_get_child_count(parent) },
            children.len() as u32
        );
        for (idx, child) in children.into_iter().enumerate() {
            assert_eq!(unsafe { lv_obj_get_child(parent, idx as i32) }, child);
        }
    }

    #[test]
    fn loading_bindings_record_delete_translate_angle_and_repeat_count() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };

        unsafe {
            lv_obj_set_style_translate_y(obj, 12, 0);
            lv_obj_set_style_transform_rotation(obj, 3600, 0);
            lv_obj_delete(obj);

            let mut anim = core::mem::MaybeUninit::<lv_anim_t>::uninit();
            lv_anim_init(anim.as_mut_ptr());
            lv_anim_set_repeat_count(anim.as_mut_ptr(), LV_ANIM_REPEAT_INFINITE);
        }

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleTranslateY { value: 12, .. })),
            "expected SetStyleTranslateY, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleTransformRotation { angle: 3600, .. })),
            "expected SetStyleTransformRotation, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(
                |c| matches!(c, LvCall::ObjDelete { obj: deleted } if *deleted == obj as usize)
            ),
            "expected ObjDelete for obj, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::AnimSetRepeatCount { count } if *count == LV_ANIM_REPEAT_INFINITE
            )),
            "expected AnimSetRepeatCount infinite, got: {:?}",
            calls
        );
    }

    #[test]
    fn anim_mock_records_full_call_sequence() {
        spy_drain(); // clear
        let mut a = core::mem::MaybeUninit::<lv_anim_t>::uninit();
        let ap = a.as_mut_ptr();
        unsafe {
            lv_anim_init(ap);
            lv_anim_set_var(ap, 0xABCD as *mut core::ffi::c_void);
            lv_anim_set_exec_cb(ap, None);
            lv_anim_set_values(ap, 0, 100);
            lv_anim_set_duration(ap, 500);
            lv_anim_set_path_cb(ap, Some(lv_anim_path_ease_out));
            lv_anim_set_completed_cb(ap, None);
            lv_anim_start(ap as *const _);
            let _ = lv_anim_delete(0xABCD as *mut core::ffi::c_void, None);
        }
        let calls = spy_drain();
        assert!(matches!(calls[0], LvCall::AnimInit));
        assert!(matches!(calls[1], LvCall::AnimSetVar { var } if var == 0xABCD));
        assert!(matches!(calls[2], LvCall::AnimSetExecCb { cb: None }));
        assert!(matches!(calls[3], LvCall::AnimSetValues { start: 0, end: 100 }));
        assert!(matches!(calls[4], LvCall::AnimSetDuration { ms: 500 }));
        assert!(matches!(calls[5], LvCall::AnimSetPathCb { cb: Some(_) }));
        assert!(matches!(calls[6], LvCall::AnimSetCompletedCb { cb: None }));
        assert!(matches!(calls[7], LvCall::AnimStart));
        assert!(matches!(calls[8], LvCall::AnimDelete { var, cb: None } if var == 0xABCD));
    }

    #[test]
    fn lv_anim_init_clears_stale_user_data_for_reused_pointer() {
        spy_drain();
        let mut a = core::mem::MaybeUninit::<lv_anim_t>::uninit();
        let ap = a.as_mut_ptr();
        unsafe {
            // Simulate a previous animation that left user_data behind in the
            // out-of-band ANIM_USER_DATA map for this same pointer (e.g. when
            // a stack slot is reused for a new animation).
            lv_anim_set_user_data(ap, 0xDEAD as *mut core::ffi::c_void);
            assert_eq!(lv_anim_get_user_data(ap), 0xDEAD as *mut core::ffi::c_void);
            // Reinitializing the anim must clear the stale entry, matching
            // LVGL's zero-initialized semantics.
            lv_anim_init(ap);
            assert!(
                lv_anim_get_user_data(ap).is_null(),
                "lv_anim_init must clear stale ANIM_USER_DATA for the reused pointer"
            );
        }
    }
}
