use crate::c_bindings;

use super::color::Color;
use super::font::Font;

/// Initialize the LVGL default theme on the default display.
///
/// Pass the brand primary and secondary colors plus the base font.
/// Call once at application startup, before creating any widgets.
/// Passing `NULL` for the display makes LVGL use the active default display.
pub fn init_default_theme(color_primary: Color, color_secondary: Color, dark: bool, font: &Font) {
    unsafe {
        c_bindings::lv_theme_default_init(
            core::ptr::null_mut(),
            color_primary.to_lv(),
            color_secondary.to_lv(),
            dark,
            font.as_ptr(),
        );
    }
}
