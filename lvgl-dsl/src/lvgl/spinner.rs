use crate::c_bindings;

use super::color::Color;
use super::widget::{LvObj, Widget};

/// LVGL spinner widget (`lv_spinner`).
///
/// Wraps an `lv_spinner_create`-allocated object and inherits all layout,
/// style, and state methods from the [`Widget`] trait.
pub struct Spinner {
    obj: LvObj,
}

impl Widget for Spinner {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Spinner {
    /// Creates a new spinner widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Spinner {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_spinner_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_spinner_create returned null");
        }
        Spinner {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Sets spinner animation duration and arc length.
    ///
    /// `spin_ms` is the time for one full spin. `arc_length_deg` is the visible
    /// arc length in degrees, matching LVGL's `lv_spinner_set_anim_params`.
    pub fn set_anim_params(&self, spin_ms: u32, arc_length_deg: u32) -> &Self {
        unsafe {
            c_bindings::lv_spinner_set_anim_params(self.lv_obj().raw(), spin_ms, arc_length_deg);
        }
        self
    }

    fn set_arc_color_part(&self, color: Color, selector: u32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_color(self.lv_obj().raw(), color.to_lv(), selector);
        }
        self
    }

    fn set_arc_width_part(&self, width: i32, selector: u32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_width(self.lv_obj().raw(), width, selector);
        }
        self
    }

    fn set_arc_opa_part(&self, opa: u8, selector: u32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_arc_opa(self.lv_obj().raw(), opa, selector);
        }
        self
    }

    /// Sets the spinner track (background arc) color.
    pub fn track_color(&self, color: Color) -> &Self {
        self.set_arc_color_part(color, c_bindings::LV_PART_MAIN)
    }

    /// Sets the spinner indicator (rotating arc) color.
    pub fn indicator_color(&self, color: Color) -> &Self {
        self.set_arc_color_part(color, c_bindings::LV_PART_INDICATOR)
    }

    /// Sets the spinner track (background arc) stroke width in pixels.
    pub fn track_width(&self, width: i32) -> &Self {
        self.set_arc_width_part(width, c_bindings::LV_PART_MAIN)
    }

    /// Sets the spinner indicator (rotating arc) stroke width in pixels.
    pub fn indicator_width(&self, width: i32) -> &Self {
        self.set_arc_width_part(width, c_bindings::LV_PART_INDICATOR)
    }

    /// Sets the spinner track opacity (0-255).
    pub fn track_opa(&self, opa: u8) -> &Self {
        self.set_arc_opa_part(opa, c_bindings::LV_PART_MAIN)
    }

    /// Sets the spinner indicator opacity (0-255).
    pub fn indicator_opa(&self, opa: u8) -> &Self {
        self.set_arc_opa_part(opa, c_bindings::LV_PART_INDICATOR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::reset_obj_pool;
    use crate::lvgl::screen::Screen;

    #[test]
    fn new_does_not_panic() {
        reset_obj_pool();
        let p = Screen::active();
        let _ = Spinner::new(&p);
    }

    #[test]
    fn set_anim_params_records_spy() {
        use crate::c_bindings::{spy_drain, LvCall};

        reset_obj_pool();
        let p = Screen::active();
        let spinner = Spinner::new(&p);
        spy_drain();

        spinner.set_anim_params(900, 90);

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SpinnerSetAnimParams {
                    spin_ms: 900,
                    arc_length_deg: 90,
                    ..
                }
            )),
            "expected SpinnerSetAnimParams, got: {:?}",
            calls
        );
    }

    #[test]
    fn track_and_indicator_color_use_correct_parts() {
        use crate::c_bindings::{spy_drain, LvCall, LV_PART_INDICATOR, LV_PART_MAIN};

        reset_obj_pool();
        let p = Screen::active();
        let spinner = Spinner::new(&p);
        spy_drain();

        spinner
            .track_color(Color::hex(0xE9F0F0))
            .indicator_color(Color::hex(0xE41C1C));

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcColor { selector, .. } if *selector == LV_PART_MAIN
            )),
            "expected StyleArcColor on LV_PART_MAIN, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcColor { selector, .. } if *selector == LV_PART_INDICATOR
            )),
            "expected StyleArcColor on LV_PART_INDICATOR, got: {:?}",
            calls
        );
    }

    #[test]
    fn track_and_indicator_width_use_correct_parts() {
        use crate::c_bindings::{spy_drain, LvCall, LV_PART_INDICATOR, LV_PART_MAIN};

        reset_obj_pool();
        let p = Screen::active();
        let spinner = Spinner::new(&p);
        spy_drain();

        spinner.track_width(4).indicator_width(6);

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcWidth { width: 4, selector, .. } if *selector == LV_PART_MAIN
            )),
            "expected StyleArcWidth(4) on LV_PART_MAIN, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcWidth { width: 6, selector, .. } if *selector == LV_PART_INDICATOR
            )),
            "expected StyleArcWidth(6) on LV_PART_INDICATOR, got: {:?}",
            calls
        );
    }

    #[test]
    fn track_and_indicator_opa_use_correct_parts() {
        use crate::c_bindings::{spy_drain, LvCall, LV_PART_INDICATOR, LV_PART_MAIN};

        reset_obj_pool();
        let p = Screen::active();
        let spinner = Spinner::new(&p);
        spy_drain();

        spinner.track_opa(64).indicator_opa(255);

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcOpa { opa: 64, selector, .. } if *selector == LV_PART_MAIN
            )),
            "expected StyleArcOpa(64) on LV_PART_MAIN, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::StyleArcOpa { opa: 255, selector, .. } if *selector == LV_PART_INDICATOR
            )),
            "expected StyleArcOpa(255) on LV_PART_INDICATOR, got: {:?}",
            calls
        );
    }
}
