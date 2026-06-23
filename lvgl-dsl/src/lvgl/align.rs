// Mirrors lv_align_t from LVGL v9 (misc/lv_area.h). Values must match exactly.
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
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

#[cfg(test)]
mod tests {
    use super::LvAlign;

    #[test]
    fn default_is_zero() {
        // Invariant: LvAlign::Default == LV_ALIGN_DEFAULT (0) in lv_area.h.
        assert_eq!(LvAlign::Default as u32, 0);
    }

    #[test]
    fn center_is_nine() {
        // Invariant: LvAlign::Center == LV_ALIGN_CENTER (9) in lv_area.h.
        assert_eq!(LvAlign::Center as u32, 9);
    }

    #[test]
    fn last_variant_is_twenty_one() {
        // Invariant: OutRightBottom is the last variant, value 21, matching the C header.
        assert_eq!(LvAlign::OutRightBottom as u32, 21);
    }

    #[test]
    fn top_left_is_one() {
        // Invariant: sequential from 0; TopLeft == 1.
        assert_eq!(LvAlign::TopLeft as u32, 1);
    }

    #[test]
    fn out_top_left_is_ten() {
        // Invariant: LvAlign::OutTopLeft == LV_ALIGN_OUT_TOP_LEFT (10) in lv_area.h.
        assert_eq!(LvAlign::OutTopLeft as u32, 10);
    }

    #[test]
    fn out_left_top_is_sixteen() {
        // Invariant: LvAlign::OutLeftTop == LV_ALIGN_OUT_LEFT_TOP (16) in lv_area.h.
        assert_eq!(LvAlign::OutLeftTop as u32, 16);
    }
}
