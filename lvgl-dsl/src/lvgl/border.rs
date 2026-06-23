use core::ops::BitOr;

/// Mirrors `lv_border_side_t` from LVGL v9 (misc/lv_style.h).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BorderSide(pub(crate) u32);

impl BorderSide {
    pub const NONE: BorderSide = BorderSide(0x00);
    pub const BOTTOM: BorderSide = BorderSide(0x01);
    pub const TOP: BorderSide = BorderSide(0x02);
    pub const LEFT: BorderSide = BorderSide(0x04);
    pub const RIGHT: BorderSide = BorderSide(0x08);
    pub const FULL: BorderSide = BorderSide(0x0F);
    pub const INTERNAL: BorderSide = BorderSide(0x10);
}

impl BitOr for BorderSide {
    type Output = BorderSide;
    fn bitor(self, rhs: BorderSide) -> BorderSide {
        BorderSide(self.0 | rhs.0)
    }
}
