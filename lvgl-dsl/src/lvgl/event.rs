use crate::c_bindings;

use super::widget::LvObj;

pub struct Event(*mut c_bindings::lv_event_t);

impl Event {
    pub(crate) fn from_raw(ptr: *mut c_bindings::lv_event_t) -> Self {
        Event(ptr)
    }

    pub fn code(&self) -> LvEventCode {
        // SAFETY: LVGL provides a valid event pointer at the FFI boundary.
        let raw = unsafe { c_bindings::lv_event_get_code(self.0) };
        LvEventCode::from_u32(raw as u32)
    }

    pub fn target(&self) -> LvObj {
        // SAFETY: LVGL guarantees a non-null target for user-triggered events.
        // lv_event_get_target returns *mut c_void; cast to lv_obj_t before use.
        let obj = unsafe { c_bindings::lv_event_get_target(self.0) as *mut c_bindings::lv_obj_t };
        LvObj::from_raw(obj)
    }

    /// Returns the raw LVGL target-object pointer as a `usize`.
    ///
    /// Use this inside static `fn(Event)` callbacks to pass the target widget
    /// across an FFI boundary where a typed wrapper cannot be held.
    /// The value is only meaningful while the LVGL object is alive.
    ///
    /// **Safety note:** the returned pointer must only be dereferenced or
    /// passed into LVGL APIs on the LVGL/UI thread, and only while the target
    /// object is alive.  Storing and using it after object deletion or across
    /// thread boundaries violates LVGL's single-threaded access contract.
    pub fn target_raw_ptr(&self) -> usize {
        // SAFETY: LVGL provides a valid event pointer at the FFI boundary.
        unsafe { c_bindings::lv_event_get_target(self.0) as usize }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LvEventCode {
    All,
    Pressed,
    ShortClicked,
    LongPressed,
    Clicked,
    Released,
    Focused,
    Defocused,
    DrawTaskAdded,
    ValueChanged,
    Ready,
    Cancel,
    ScreenLoadStart,
    ScreenLoaded,
    ScreenUnloaded,
    SizeChanged,
    Unknown(u32),
}

impl LvEventCode {
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::All => 0,
            Self::Pressed => 1,
            Self::ShortClicked => 4,
            Self::LongPressed => 8,
            Self::Clicked => 10,
            Self::Released => 11,
            Self::Focused => 19,
            Self::Defocused => 20,
            Self::DrawTaskAdded => 34,
            Self::ValueChanged => 35,
            // LVGL v9.3.0 src/misc/lv_event.h:79-80. Desktop sim is currently
            // 9.6-dev; these values intentionally follow the 9.3 device pin.
            Self::Ready => 38,
            Self::Cancel => 39,
            Self::ScreenLoadStart => 46,
            Self::ScreenLoaded => 47,
            Self::ScreenUnloaded => 48,
            Self::SizeChanged => 49,
            Self::Unknown(v) => v,
        }
    }

    fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::All,
            1 => Self::Pressed,
            4 => Self::ShortClicked,
            8 => Self::LongPressed,
            10 => Self::Clicked,
            11 => Self::Released,
            19 => Self::Focused,
            20 => Self::Defocused,
            34 => Self::DrawTaskAdded,
            35 => Self::ValueChanged,
            38 => Self::Ready,
            39 => Self::Cancel,
            46 => Self::ScreenLoadStart,
            47 => Self::ScreenLoaded,
            48 => Self::ScreenUnloaded,
            49 => Self::SizeChanged,
            _ => Self::Unknown(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LvEventCode;

    #[test]
    fn event_codes_match_lvgl_9_3_header_values() {
        // LVGL v9.3.0 src/misc/lv_event.h:35-48,56-57,76,79-91.
        assert_eq!(LvEventCode::All.as_u32(), 0);
        assert_eq!(LvEventCode::Pressed.as_u32(), 1);
        assert_eq!(LvEventCode::ShortClicked.as_u32(), 4);
        assert_eq!(LvEventCode::LongPressed.as_u32(), 8);
        assert_eq!(LvEventCode::Clicked.as_u32(), 10);
        assert_eq!(LvEventCode::Released.as_u32(), 11);
        assert_eq!(LvEventCode::Focused.as_u32(), 19);
        assert_eq!(LvEventCode::Defocused.as_u32(), 20);
        assert_eq!(LvEventCode::DrawTaskAdded.as_u32(), 34);
        assert_eq!(LvEventCode::ValueChanged.as_u32(), 35);
        assert_eq!(LvEventCode::Ready.as_u32(), 38);
        assert_eq!(LvEventCode::Cancel.as_u32(), 39);
        assert_eq!(LvEventCode::ScreenLoadStart.as_u32(), 46);
        assert_eq!(LvEventCode::ScreenLoaded.as_u32(), 47);
        assert_eq!(LvEventCode::ScreenUnloaded.as_u32(), 48);
        assert_eq!(LvEventCode::SizeChanged.as_u32(), 49);
    }

    #[test]
    fn ready_and_cancel_parse_from_lvgl_9_3_values() {
        // LVGL v9.3.0 src/misc/lv_event.h:79-80.
        assert_eq!(LvEventCode::from_u32(38), LvEventCode::Ready);
        assert_eq!(LvEventCode::from_u32(39), LvEventCode::Cancel);
    }

    #[test]
    fn screen_events_parse_from_lvgl_9_3_values() {
        // LVGL v9.3.0 src/misc/lv_event.h:88-91.
        assert_eq!(LvEventCode::from_u32(46), LvEventCode::ScreenLoadStart);
        assert_eq!(LvEventCode::from_u32(47), LvEventCode::ScreenLoaded);
        assert_eq!(LvEventCode::from_u32(48), LvEventCode::ScreenUnloaded);
        assert_eq!(LvEventCode::from_u32(49), LvEventCode::SizeChanged);
    }

    #[test]
    fn unknown_event_codes_are_preserved() {
        assert_eq!(LvEventCode::from_u32(1234), LvEventCode::Unknown(1234));
    }

    #[test]
    fn unknown_event_codes_round_trip_to_raw_value() {
        assert_eq!(LvEventCode::Unknown(1234).as_u32(), 1234);
    }
}
