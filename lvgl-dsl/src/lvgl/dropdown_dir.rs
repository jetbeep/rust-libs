/// Direction in which the dropdown list opens.
///
/// Mirrors `lv_dir_t` from LVGL v9.3 (`misc/lv_area.h`).
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LvDropdownDir {
    /// Open the list below the button (`LV_DIR_BOTTOM`, v9.3 `lv_area.h:82`).
    Down = 8,
    /// Open the list above the button (`LV_DIR_TOP`, v9.3 `lv_area.h:81`).
    Up = 4,
    /// Open the list to the left of the button (`LV_DIR_LEFT`, v9.3 `lv_area.h:79`).
    Left = 1,
    /// Open the list to the right of the button (`LV_DIR_RIGHT`, v9.3 `lv_area.h:80`).
    Right = 2,
}

#[cfg(test)]
mod tests {
    use super::LvDropdownDir;

    #[test]
    fn dropdown_horizontal_dirs_match_lvgl_9_3_lv_dir_t_bitmask_values() {
        // LVGL v9.3.0 src/misc/lv_area.h:77-86.
        assert_eq!(LvDropdownDir::Left as u32, 1);
        assert_eq!(LvDropdownDir::Right as u32, 2);
    }

    #[test]
    fn dropdown_vertical_dirs_match_lvgl_9_3_lv_dir_t_bitmask_values() {
        // LVGL v9.3.0 src/misc/lv_area.h:77-86.
        assert_eq!(LvDropdownDir::Up as u32, 4);
        assert_eq!(LvDropdownDir::Down as u32, 8);
    }
}
