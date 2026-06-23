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

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LvEventCode {
    All = 0,
    Pressed = 1,
    ShortClicked = 4,
    LongPressed = 8,
    Clicked = 10,
    Released = 11,
    Focused = 19,
    Defocused = 20,
    ValueChanged = 35,
    Ready = 36,
    Cancel = 37,
    ScreenLoadStart = 46,
    ScreenLoaded = 47,
    ScreenUnloaded = 48,
}

impl LvEventCode {
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
            35 => Self::ValueChanged,
            36 => Self::Ready,
            37 => Self::Cancel,
            46 => Self::ScreenLoadStart,
            47 => Self::ScreenLoaded,
            48 => Self::ScreenUnloaded,
            _ => Self::All,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LvEventCode;

    #[test]
    fn all_is_zero() {
        assert_eq!(LvEventCode::All as u32, 0);
    }
    #[test]
    fn clicked_is_ten() {
        assert_eq!(LvEventCode::Clicked as u32, 10);
    }
    #[test]
    fn value_changed_is_35() {
        assert_eq!(LvEventCode::ValueChanged as u32, 35);
    }
    #[test]
    fn screen_loaded_is_47() {
        assert_eq!(LvEventCode::ScreenLoaded as u32, 47);
    }
}
