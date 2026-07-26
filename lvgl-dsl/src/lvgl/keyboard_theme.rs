use super::Font;

// ---------------------------------------------------------------------------
// LVGL part + state selectors for keyboard / buttonmatrix styling
// ---------------------------------------------------------------------------

/// Style selector targeting all keys at rest.
/// `LV_PART_ITEMS = 0x00050000`
pub(crate) const SELECTOR_KEY_NORMAL: u32 = 0x0005_0000;

/// Style selector targeting all keys in their pressed state.
/// `LV_PART_ITEMS | LV_STATE_PRESSED` where LVGL v9.3 `LV_STATE_PRESSED = 0x20`.
pub(crate) const SELECTOR_KEY_PRESSED: u32 = 0x0005_0020;

/// Style selector targeting action keys (Enter, Backspace, etc.).
///
/// Action keys are marked with `LV_BUTTONMATRIX_CTRL_CHECKED` in the ctrl
/// map, which makes LVGL apply `LV_STATE_CHECKED` during drawing.
/// `LV_PART_ITEMS | LV_STATE_CHECKED` where LVGL v9.3 `LV_STATE_CHECKED = 0x01`.
pub(crate) const SELECTOR_KEY_ACTION: u32 = 0x0005_0001;

/// Style selector targeting keys whose `LV_BUTTONMATRIX_CTRL_DISABLED`
/// flag has been set (e.g. via [`crate::lvgl::Keyboard::set_continue_enabled`]).
/// `LV_PART_ITEMS | LV_STATE_DISABLED` where LVGL v9.3 `LV_STATE_DISABLED = 0x80`.
pub(crate) const SELECTOR_KEY_DISABLED: u32 = 0x0005_0080;

// ---------------------------------------------------------------------------
// KeyboardTheme
// ---------------------------------------------------------------------------

/// A bundle of keyboard visual properties that can be applied atomically.
///
/// Stores raw 24-bit RGB hex values rather than [`crate::Color`] because
/// [`crate::Color::hex`] calls a C FFI function and cannot be used in `const`
/// contexts.  The values are converted to [`crate::Color`] inside
/// [`crate::Keyboard::theme`] at apply time.
///
/// ## Pre-built themes
///
/// Two ready-to-use themes are provided as associated constants:
///
/// ```rust
/// use jetbeep_lvgl_dsl::lvgl::prelude::*;
///
/// let kb = Keyboard::new(&screen);
/// kb.theme(&KeyboardTheme::DARK);
/// kb.theme(&KeyboardTheme::LIGHT);
/// ```
pub struct KeyboardTheme {
    /// 24-bit RGB hex for the keyboard background (e.g. `0x1E1E1E`).
    pub bg_hex: u32,
    /// 24-bit RGB hex for regular (non-action) key backgrounds.
    pub key_normal_hex: u32,
    /// 24-bit RGB hex for action key backgrounds (Enter, Backspace, Space).
    pub key_action_hex: u32,
    /// Uniform corner radius in pixels applied to every key.
    pub key_radius_px: i32,
    /// Optional font factory for key labels.
    ///
    /// Use `Some(Font::montserrat_14)` to supply a specific font, or
    /// `None` to leave the inherited font unchanged.
    pub font: Option<fn() -> Font>,
}

impl KeyboardTheme {
    /// Light theme — white keys on a near-white background.
    pub const LIGHT: KeyboardTheme = KeyboardTheme {
        bg_hex: 0xF5F5F5,
        key_normal_hex: 0xFFFFFF,
        key_action_hex: 0xE0E0E0,
        key_radius_px: 6,
        font: None,
    };

    /// Dark theme — dark grey keys on a near-black background.
    pub const DARK: KeyboardTheme = KeyboardTheme {
        bg_hex: 0x1E1E1E,
        key_normal_hex: 0x333333,
        key_action_hex: 0x555555,
        key_radius_px: 6,
        font: None,
    };
}

#[cfg(test)]
mod tests {
    use super::{
        KeyboardTheme, SELECTOR_KEY_ACTION, SELECTOR_KEY_DISABLED, SELECTOR_KEY_NORMAL,
        SELECTOR_KEY_PRESSED,
    };

    #[test]
    fn key_selectors_match_lvgl_9_3_part_and_state_values() {
        // LVGL v9.3.0 src/widgets/buttonmatrix/lv_buttonmatrix.h:36-42 (LV_PART_ITEMS = 5).
        // LVGL v9.3.0 src/core/lv_obj.h:47-55 (CHECKED=0x0001, PRESSED=0x0020, DISABLED=0x0080).
        assert_eq!(SELECTOR_KEY_NORMAL, 0x0005_0000);
        assert_eq!(SELECTOR_KEY_ACTION, 0x0005_0001);
        assert_eq!(SELECTOR_KEY_PRESSED, 0x0005_0020);
        assert_eq!(SELECTOR_KEY_DISABLED, 0x0005_0080);
    }

    #[test]
    fn light_theme_bg_is_bright() {
        // Sanity: light theme background should be a light colour.
        let lum = {
            let r = ((KeyboardTheme::LIGHT.bg_hex >> 16) & 0xFF) as u32;
            let g = ((KeyboardTheme::LIGHT.bg_hex >> 8) & 0xFF) as u32;
            let b = (KeyboardTheme::LIGHT.bg_hex & 0xFF) as u32;
            r + g + b
        };
        assert!(
            lum > 600,
            "LIGHT theme background should be bright, got luminance {lum}"
        );
    }

    #[test]
    fn dark_theme_bg_is_dark() {
        let lum = {
            let r = ((KeyboardTheme::DARK.bg_hex >> 16) & 0xFF) as u32;
            let g = ((KeyboardTheme::DARK.bg_hex >> 8) & 0xFF) as u32;
            let b = (KeyboardTheme::DARK.bg_hex & 0xFF) as u32;
            r + g + b
        };
        assert!(
            lum < 150,
            "DARK theme background should be dark, got luminance {lum}"
        );
    }

    #[test]
    fn light_has_no_font_override() {
        assert!(KeyboardTheme::LIGHT.font.is_none());
    }

    #[test]
    fn dark_has_no_font_override() {
        assert!(KeyboardTheme::DARK.font.is_none());
    }

    #[test]
    fn custom_theme_with_font() {
        use crate::lvgl::Font;
        let t = KeyboardTheme {
            bg_hex: 0x000000,
            key_normal_hex: 0x111111,
            key_action_hex: 0x222222,
            key_radius_px: 4,
            font: Some(Font::montserrat_14),
        };
        assert!(t.font.is_some());
        // Ensure the factory can be called without panicking.
        let _f = t.font.unwrap()();
    }
}
