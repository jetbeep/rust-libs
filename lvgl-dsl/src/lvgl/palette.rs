/// Mirrors `lv_palette_t` from LVGL v9 (misc/lv_palette.h).
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum Palette {
    Red = 0,
    Pink = 1,
    Purple = 2,
    DeepPurple = 3,
    Indigo = 4,
    Blue = 5,
    LightBlue = 6,
    Cyan = 7,
    Teal = 8,
    Green = 9,
    LightGreen = 10,
    Lime = 11,
    Yellow = 12,
    Amber = 13,
    Orange = 14,
    DeepOrange = 15,
    Brown = 16,
    BlueGrey = 17,
    Grey = 18,
}
