/// Direction in which the dropdown list opens.
///
/// Mirrors `lv_dropdown_dir_t` from LVGL v9 (`widgets/dropdown/lv_dropdown.h`).
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LvDropdownDir {
    /// Open the list below the button (default).
    Down = 0,
    /// Open the list above the button.
    Up = 1,
    /// Open the list to the left of the button.
    Left = 2,
    /// Open the list to the right of the button.
    Right = 3,
}

#[cfg(test)]
mod tests {
    use super::LvDropdownDir;

    #[test]
    fn down_is_zero() {
        assert_eq!(LvDropdownDir::Down as u32, 0);
    }
    #[test]
    fn up_is_one() {
        assert_eq!(LvDropdownDir::Up as u32, 1);
    }
    #[test]
    fn left_is_two() {
        assert_eq!(LvDropdownDir::Left as u32, 2);
    }
    #[test]
    fn right_is_three() {
        assert_eq!(LvDropdownDir::Right as u32, 3);
    }
}
