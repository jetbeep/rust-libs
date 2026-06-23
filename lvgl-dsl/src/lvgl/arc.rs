//! LVGL arc widget (`lv_arc`).
//!
//! Part-aware styling helpers are provided for `LV_PART_MAIN` (the
//! background track), `LV_PART_INDICATOR` (the filled portion of the arc)
//! and `LV_PART_KNOB` (the draggable thumb).

use crate::c_bindings;

use super::color::Color;
use super::event::{Event, LvEventCode};
use super::widget::{LvObj, Widget};

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArcMode {
    Normal      = 0, // c_bindings::LV_ARC_MODE_NORMAL
    Symmetrical = 1, // c_bindings::LV_ARC_MODE_SYMMETRICAL
    Reverse     = 2, // c_bindings::LV_ARC_MODE_REVERSE
}

pub struct Arc {
    obj: LvObj,
}

impl Widget for Arc {
    fn lv_obj(&self) -> &LvObj { &self.obj }
}

impl Arc {
    pub fn new(parent: &impl Widget) -> Arc {
        let obj = unsafe { c_bindings::lv_arc_create(parent.lv_obj().raw()) };
        if obj.is_null() { panic!("lv_arc_create returned null"); }
        Arc { obj: LvObj::from_raw(obj) }
    }

    // --- Value & range ---
    pub fn set_range(&self, min: i32, max: i32) -> &Self {
        unsafe { c_bindings::lv_arc_set_range(self.obj.raw(), min, max) };
        self
    }
    pub fn set_value(&self, value: i32) -> &Self {
        unsafe { c_bindings::lv_arc_set_value(self.obj.raw(), value) };
        self
    }
    pub fn value(&self) -> i32 {
        unsafe { c_bindings::lv_arc_get_value(self.obj.raw()) }
    }

    // --- Geometry ---
    pub fn set_bg_angles(&self, start_deg: u16, end_deg: u16) -> &Self {
        unsafe { c_bindings::lv_arc_set_bg_angles(self.obj.raw(), start_deg, end_deg) };
        self
    }
    pub fn set_angles(&self, start_deg: u16, end_deg: u16) -> &Self {
        unsafe { c_bindings::lv_arc_set_angles(self.obj.raw(), start_deg, end_deg) };
        self
    }
    pub fn set_rotation(&self, deg: u16) -> &Self {
        unsafe { c_bindings::lv_arc_set_rotation(self.obj.raw(), deg) };
        self
    }
    pub fn set_mode(&self, mode: ArcMode) -> &Self {
        unsafe { c_bindings::lv_arc_set_mode(self.obj.raw(), mode as u32) };
        self
    }
    pub fn set_change_rate(&self, rate: u32) -> &Self {
        unsafe { c_bindings::lv_arc_set_change_rate(self.obj.raw(), rate) };
        self
    }

    /// Removes the default LVGL arc styling so all visual properties must be
    /// set explicitly. Useful when the arc is purely a non-interactive progress
    /// indicator with custom colors and no knob.
    pub fn remove_default_style(&self) -> &Self {
        unsafe { c_bindings::lv_obj_remove_style_all(self.obj.raw()) };
        self
    }

    // --- Part-aware styling ---

    /// Color of the background track (`LV_PART_MAIN`).
    pub fn track_color(&self, c: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_color(
                self.obj.raw(), c.to_lv(), c_bindings::LV_PART_MAIN,
            );
        }
        self
    }
    pub fn track_width(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_width(self.obj.raw(), px, c_bindings::LV_PART_MAIN);
        }
        self
    }
    pub fn track_opa(&self, opa: u8) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_opa(self.obj.raw(), opa, c_bindings::LV_PART_MAIN);
        }
        self
    }
    pub fn track_rounded(&self, rounded: bool) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_rounded(
                self.obj.raw(), rounded, c_bindings::LV_PART_MAIN,
            );
        }
        self
    }

    /// Color of the indicator (filled portion, `LV_PART_INDICATOR`).
    pub fn indicator_color(&self, c: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_color(
                self.obj.raw(), c.to_lv(), c_bindings::LV_PART_INDICATOR,
            );
        }
        self
    }
    pub fn indicator_width(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_width(
                self.obj.raw(), px, c_bindings::LV_PART_INDICATOR,
            );
        }
        self
    }
    pub fn indicator_opa(&self, opa: u8) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_opa(
                self.obj.raw(), opa, c_bindings::LV_PART_INDICATOR,
            );
        }
        self
    }
    pub fn indicator_rounded(&self, rounded: bool) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_rounded(
                self.obj.raw(), rounded, c_bindings::LV_PART_INDICATOR,
            );
        }
        self
    }

    /// Knob color (`LV_PART_KNOB`).
    pub fn knob_color(&self, c: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_color(
                self.obj.raw(), c.to_lv(), c_bindings::LV_PART_KNOB,
            );
        }
        self
    }

    /// Hide the knob entirely (sets `bg_opa = 0` on the knob part).
    pub fn remove_knob(&self) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_opa(
                self.obj.raw(), 0, c_bindings::LV_PART_KNOB,
            );
        }
        self
    }

    // --- Events ---

    pub fn on_value_changed(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::ValueChanged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{reset_obj_pool, spy_drain, LvCall};
    use crate::lvgl::screen::Screen;

    #[test]
    fn new_does_not_panic() {
        reset_obj_pool();
        let p = Screen::active();
        let _ = Arc::new(&p);
    }

    #[test]
    fn set_range_records_spy() {
        reset_obj_pool();
        let p = Screen::active();
        let a = Arc::new(&p);
        spy_drain();
        a.set_range(0, 1000);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ArcSetRange { min: 0, max: 1000, .. })),
            "expected ArcSetRange, got: {:?}", calls
        );
    }

    #[test]
    fn set_value_round_trips() {
        reset_obj_pool();
        let p = Screen::active();
        let a = Arc::new(&p);
        a.set_value(42);
        assert_eq!(a.value(), 42);
    }

    #[test]
    fn set_bg_angles_and_rotation_record_spies() {
        reset_obj_pool();
        let p = Screen::active();
        let a = Arc::new(&p);
        spy_drain();
        a.set_bg_angles(0, 360).set_rotation(270);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ArcSetBgAngles { start: 0, end: 360, .. })),
            "expected ArcSetBgAngles, got: {:?}", calls
        );
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ArcSetRotation { rotation: 270, .. })),
            "expected ArcSetRotation, got: {:?}", calls
        );
    }

    #[test]
    fn part_aware_styling_records_selectors() {
        reset_obj_pool();
        let p = Screen::active();
        let a = Arc::new(&p);
        spy_drain();
        a.indicator_color(Color::hex(0xE41C1C))
            .indicator_width(12)
            .indicator_rounded(true)
            .track_color(Color::hex(0xE9F0F0))
            .track_width(12)
            .remove_knob();
        let calls = spy_drain();
        // Indicator-part styling
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::StyleArcWidth { width: 12, selector, .. }
                    if *selector == c_bindings::LV_PART_INDICATOR)),
            "expected indicator-part arc width, got: {:?}", calls
        );
        // Main-part styling
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::StyleArcWidth { width: 12, selector, .. }
                    if *selector == c_bindings::LV_PART_MAIN)),
            "expected main-part arc width, got: {:?}", calls
        );
    }

    #[test]
    fn set_mode_records_mode() {
        reset_obj_pool();
        let p = Screen::active();
        let a = Arc::new(&p);
        spy_drain();
        a.set_mode(ArcMode::Reverse);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ArcSetMode { mode: 2, .. })),
            "expected ArcSetMode 2 (Reverse), got: {:?}", calls
        );
    }
}
