/// The value LVGL uses for `LV_RADIUS_CIRCLE` — renders a widget as a full
/// circle or pill (equivalent to `0x7FFF` in LVGL 9.x).
///
/// A local constant is used here instead of depending on the bindgen-generated
/// `LV_RADIUS_CIRCLE` so that `cargo test` works without a Zephyr toolchain.
pub(crate) const RADIUS_CIRCLE_VALUE: i32 = 0x7FFF;

/// A typed corner-radius value for LVGL widgets.
///
/// LVGL 9.3 applies a **single uniform radius** to all four corners via
/// `lv_obj_set_style_radius`.  Per-corner control is not available in this
/// version of LVGL.
///
/// # Example
///
/// ```rust
/// use jetbeep::prelude::*;
///
/// // Fully-rounded pill / circle shape
/// label.radius(CornerRadius::Full);
///
/// // 8 px radius — also accepts a plain i32 via From<i32>
/// label.radius(CornerRadius::Px(8));
/// label.radius(8);   // equivalent
///
/// // Remove any radius
/// label.radius(CornerRadius::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerRadius {
    /// No rounding — equivalent to a radius of `0`.
    None,
    /// Fully-rounded corners (pill / circle shape).
    ///
    /// Maps to `LV_RADIUS_CIRCLE` (`0x7FFF`).
    Full,
    /// Explicit pixel radius.
    Px(i32),
}

impl CornerRadius {
    /// Convert to the raw `int32_t` value expected by `lv_obj_set_style_radius`.
    #[inline]
    pub(crate) fn into_lv_value(self) -> i32 {
        match self {
            CornerRadius::None => 0,
            CornerRadius::Full => RADIUS_CIRCLE_VALUE,
            CornerRadius::Px(r) => r,
        }
    }
}

impl From<i32> for CornerRadius {
    /// Wrap a raw pixel value.  Passing `0x7FFF` will work but prefer
    /// [`CornerRadius::Full`] for clarity.
    #[inline]
    fn from(r: i32) -> Self {
        CornerRadius::Px(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_zero() {
        assert_eq!(CornerRadius::None.into_lv_value(), 0);
    }

    #[test]
    fn full_is_radius_circle() {
        assert_eq!(CornerRadius::Full.into_lv_value(), RADIUS_CIRCLE_VALUE);
    }

    #[test]
    fn px_passes_through() {
        assert_eq!(CornerRadius::Px(12).into_lv_value(), 12);
    }

    #[test]
    fn from_i32_wraps_as_px() {
        let r: CornerRadius = 16.into();
        assert_eq!(r, CornerRadius::Px(16));
    }
}
