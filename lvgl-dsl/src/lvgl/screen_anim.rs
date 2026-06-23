/// Mirrors `lv_screen_load_anim_t` from LVGL v9 (display/lv_display.h).
/// `FADE_ON` is a backward-compat C alias for `FadeIn` — omitted.
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum ScreenAnim {
    None = 0,
    OverLeft = 1,
    OverRight = 2,
    OverTop = 3,
    OverBottom = 4,
    MoveLeft = 5,
    MoveRight = 6,
    MoveTop = 7,
    MoveBottom = 8,
    FadeIn = 9,
    FadeOut = 10,
    OutLeft = 11,
    OutRight = 12,
    OutTop = 13,
    OutBottom = 14,
}
