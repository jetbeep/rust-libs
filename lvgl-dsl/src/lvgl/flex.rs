/// Mirrors `lv_flex_flow_t` from LVGL v9 (layouts/flex/lv_flex.h).
/// Values are bit-composed: COLUMN=1, WRAP=4, REVERSE=8.
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum FlexFlow {
    Row = 0,
    Column = 1,
    RowWrap = 4,
    ColumnWrap = 5,
    RowReverse = 8,
    ColumnReverse = 9,
    RowWrapReverse = 12,
    ColumnWrapReverse = 13,
}

/// Mirrors `lv_flex_align_t` from LVGL v9 (layouts/flex/lv_flex.h).
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum FlexAlign {
    Start = 0,
    End = 1,
    Center = 2,
    SpaceEvenly = 3,
    SpaceAround = 4,
    SpaceBetween = 5,
}

#[cfg(test)]
mod tests {
    use super::{FlexAlign, FlexFlow};

    #[test]
    fn row_is_zero() {
        assert_eq!(FlexFlow::Row as u32, 0);
    }
    #[test]
    fn column_is_one() {
        assert_eq!(FlexFlow::Column as u32, 1);
    }
    #[test]
    fn row_wrap_is_four() {
        assert_eq!(FlexFlow::RowWrap as u32, 4);
    }
    #[test]
    fn column_wrap_reverse_is_thirteen() {
        assert_eq!(FlexFlow::ColumnWrapReverse as u32, 13);
    }
    #[test]
    fn flex_align_space_between_is_five() {
        assert_eq!(FlexAlign::SpaceBetween as u32, 5);
    }
    #[test]
    fn flex_align_start_is_zero() {
        assert_eq!(FlexAlign::Start as u32, 0);
    }
}
