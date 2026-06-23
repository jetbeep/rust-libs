use crate::c_bindings;

#[derive(Copy, Clone)]
pub struct Font {
    font: *const c_bindings::lv_font_t,
}

impl Font {
  
    pub const fn montserrat_48() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_48` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_48,
        }
    }
  
    pub const fn montserrat_40() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_40` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_40,
        }
    }

    pub const fn montserrat_32() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_32` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_32,
        }
    }

    pub const fn montserrat_30() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_30` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_30,
        }
    }

    pub const fn montserrat_24() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_24` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_24,
        }
    }

    pub const fn montserrat_20() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_20` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_20,
        }
    }

    pub const fn montserrat_14() -> Font {
        Font {
            // SAFETY: `lv_font_montserrat_14` is a static LVGL font symbol — never dangling.
            font: &raw const c_bindings::lv_font_montserrat_14,
        }
    }

    pub const fn as_ptr(&self) -> *const c_bindings::lv_font_t {
        self.font
    }

    /// Build a `Font` from a raw `lv_font_t` symbol address.
    ///
    /// Used by app crates that ship their own custom fonts (e.g. Poppins
    /// generated via `lv_font_conv`). The pointed-to font must have static
    /// lifetime — i.e. it should be a linker-resolved global C symbol.
    ///
    /// SAFETY: caller asserts `ptr` is non-null and points to a valid
    /// `'static lv_font_t`.
    pub const unsafe fn from_raw(ptr: *const c_bindings::lv_font_t) -> Font {
        Font { font: ptr }
    }
}
