use core::ops::BitOr;

// --- LvState ---

/// Mirrors `lv_state_t` (`uint16_t` bitfield) from LVGL v9 (core/lv_obj.h).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LvState(pub(crate) u16);

impl LvState {
    pub const DEFAULT: LvState = LvState(0x0000);
    pub const CHECKED: LvState = LvState(0x0001);
    pub const FOCUSED: LvState = LvState(0x0002);
    pub const FOCUS_KEY: LvState = LvState(0x0004);
    pub const EDITED: LvState = LvState(0x0008);
    pub const HOVERED: LvState = LvState(0x0010);
    pub const PRESSED: LvState = LvState(0x0020);
    pub const SCROLLED: LvState = LvState(0x0040);
    pub const DISABLED: LvState = LvState(0x0080);
}

impl BitOr for LvState {
    type Output = LvState;
    fn bitor(self, rhs: LvState) -> LvState {
        LvState(self.0 | rhs.0)
    }
}

// --- LvObjFlag ---

/// Mirrors `lv_obj_flag_t` (`uint32_t` bitfield) from LVGL v9 (core/lv_obj.h).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LvObjFlag(pub(crate) u32);

impl LvObjFlag {
    pub const HIDDEN: LvObjFlag = LvObjFlag(1 << 0);
    pub const CLICKABLE: LvObjFlag = LvObjFlag(1 << 1);
    pub const CLICK_FOCUSABLE: LvObjFlag = LvObjFlag(1 << 2);
    pub const CHECKABLE: LvObjFlag = LvObjFlag(1 << 3);
    pub const SCROLLABLE: LvObjFlag = LvObjFlag(1 << 4);
    pub const SCROLL_ELASTIC: LvObjFlag = LvObjFlag(1 << 5);
    pub const SCROLL_MOMENTUM: LvObjFlag = LvObjFlag(1 << 6);
    pub const EVENT_BUBBLE: LvObjFlag = LvObjFlag(1 << 14);
    pub const IGNORE_LAYOUT: LvObjFlag = LvObjFlag(1 << 17);
    pub const FLOATING: LvObjFlag = LvObjFlag(1 << 18);
    pub const OVERFLOW_VISIBLE: LvObjFlag = LvObjFlag(1 << 20);
}

impl BitOr for LvObjFlag {
    type Output = LvObjFlag;
    fn bitor(self, rhs: LvObjFlag) -> LvObjFlag {
        LvObjFlag(self.0 | rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{LvObjFlag, LvState};

    #[test]
    fn state_checked_is_bit_0() {
        assert_eq!(LvState::CHECKED.0, 0x0001);
    }
    #[test]
    fn state_focused_is_bit_1() {
        assert_eq!(LvState::FOCUSED.0, 0x0002);
    }
    #[test]
    fn state_disabled_is_bit_7() {
        assert_eq!(LvState::DISABLED.0, 0x0080);
    }
    #[test]
    fn state_default_is_zero() {
        assert_eq!(LvState::DEFAULT.0, 0x0000);
    }
    #[test]
    fn state_bitor_combines_bits() {
        assert_eq!((LvState::CHECKED | LvState::FOCUSED).0, 0x0003);
    }
    #[test]
    fn flag_hidden_is_bit_0() {
        assert_eq!(LvObjFlag::HIDDEN.0, 1u32 << 0);
    }
    #[test]
    fn flag_clickable_is_bit_1() {
        assert_eq!(LvObjFlag::CLICKABLE.0, 1u32 << 1);
    }
    #[test]
    fn flag_floating_is_bit_18() {
        assert_eq!(LvObjFlag::FLOATING.0, 1u32 << 18);
    }
    #[test]
    fn flag_bitor_combines_bits() {
        assert_eq!((LvObjFlag::HIDDEN | LvObjFlag::CLICKABLE).0, 0b11u32);
    }
}
