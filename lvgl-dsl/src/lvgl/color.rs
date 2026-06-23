use crate::c_bindings;

use super::palette::Palette;

#[derive(Copy, Clone)]
pub struct Color {
    inner: c_bindings::lv_color_t,
}

impl Color {
    /// Create a color from a 24-bit hex value (e.g. `0xFF8800`).
    pub fn hex(val: u32) -> Color {
        Color {
            // SAFETY: lv_color_hex is a pure arithmetic function — no pointers.
            inner: unsafe { c_bindings::lv_color_hex(val) },
        }
    }

    /// Create a color from red, green, blue components.
    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            // SAFETY: lv_color_make is a pure arithmetic function — no pointers.
            inner: unsafe { c_bindings::lv_color_make(r, g, b) },
        }
    }

    pub fn white() -> Color {
        Color {
            inner: unsafe { c_bindings::lv_color_white() },
        }
    }

    pub fn black() -> Color {
        Color {
            inner: unsafe { c_bindings::lv_color_black() },
        }
    }

    pub fn palette(p: Palette) -> Color {
        Color {
            inner: unsafe { c_bindings::lv_palette_main(p as u32) },
        }
    }

    pub fn palette_light(p: Palette, level: u8) -> Color {
        Color {
            inner: unsafe { c_bindings::lv_palette_lighten(p as u32, level) },
        }
    }

    pub fn palette_dark(p: Palette, level: u8) -> Color {
        Color {
            inner: unsafe { c_bindings::lv_palette_darken(p as u32, level) },
        }
    }

    pub(super) fn to_lv(self) -> c_bindings::lv_color_t {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_is_copyable_for_style_structs() {
        let a = Color::hex(0xFF6600);
        let b = a;
        let _c = a;
        assert_eq!(b.to_lv(), Color::hex(0xFF6600).to_lv());
    }
}
