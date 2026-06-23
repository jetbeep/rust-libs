use crate::c_bindings;

use super::border::BorderSide;
use super::color::Color;
use super::corner_radius::CornerRadius;
use super::event::{Event, LvEventCode};
use super::flex::{FlexAlign, FlexFlow};
use super::image::{ImageRetentionSlot, ImageSrc, set_retained_src_for_obj};
use super::size::Size;
use super::state::{LvObjFlag, LvState};
use super::static_style::StaticStyle;
use super::style::Style;
use super::{Font, LvAlign};

pub struct LvObj {
    obj: *mut c_bindings::lv_obj_t,
}

impl LvObj {
    pub(crate) fn from_raw(obj: *mut c_bindings::lv_obj_t) -> Self {
        Self { obj }
    }

    pub(crate) fn raw(&self) -> *mut c_bindings::lv_obj_t {
        self.obj
    }
}

pub trait Widget: Sized {
    fn lv_obj(&self) -> &LvObj;

    fn align(&self, align: LvAlign, x_ofs: i32, y_ofs: i32) -> &Self {
        // SAFETY: `LvObj` instances are only constructed from non-null LVGL object pointers.
        unsafe {
            c_bindings::lv_obj_align(self.lv_obj().raw(), align as u32, x_ofs, y_ofs);
        }

        self
    }

    fn text_font(&self, font: &Font) -> &Self {
        // SAFETY: `LvObj` is non-null at construction; font pointer is a static linker symbol.
        unsafe {
            c_bindings::lv_obj_set_style_text_font(self.lv_obj().raw(), font.as_ptr(), 0);
        }
        self
    }

    fn add_style(&self, style: &Style) -> &Self {
        unsafe {
            c_bindings::lv_obj_add_style(self.lv_obj().raw(), style.as_raw(), 0);
        }
        self
    }

    fn add_static_style(&self, style: &'static StaticStyle) -> &Self {
        unsafe {
            c_bindings::lv_obj_add_style(self.lv_obj().raw(), style.as_lv_style_ptr(), 0);
        }
        self
    }

    fn add_static_styles(&self, styles: &[&'static StaticStyle]) -> &Self {
        for style in styles {
            self.add_static_style(style);
        }
        self
    }

    fn on_event(&self, cb: fn(Event), code: LvEventCode) -> &Self {
        // One shared trampoline for all safe callbacks.
        // SAFETY: user_data stores the fn(Event) pointer cast to *mut c_void;
        // trampoline reads it back with transmute, which is valid because fn pointers
        // and data pointers are the same size on all supported targets.
        unsafe extern "C" fn trampoline(e: *mut c_bindings::lv_event_t) {
            unsafe {
                let user_data = c_bindings::lv_event_get_user_data(e);
                let cb: fn(Event) = core::mem::transmute(user_data);
                cb(Event::from_raw(e));
            }
        }
        unsafe {
            c_bindings::lv_obj_add_event_cb(
                self.lv_obj().raw(),
                Some(trampoline),
                code as u32,
                cb as usize as *mut core::ffi::c_void,
            );
        }
        self
    }

    fn on_click(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::Clicked)
    }

    /// Returns the raw LVGL object pointer as a `usize`.
    ///
    /// Use this to store widget handles across FFI boundaries or in contexts
    /// (e.g. `AtomicUsize`) where the typed wrapper cannot be held.
    /// The value is only meaningful while the LVGL object is alive.
    ///
    /// **Safety note:** the returned pointer must only be dereferenced or
    /// passed into LVGL APIs on the LVGL/UI thread.  Using it from any other
    /// thread violates LVGL's single-threaded access contract and is undefined
    /// behaviour.
    fn raw_ptr(&self) -> usize {
        self.lv_obj().raw() as usize
    }

    // --- State ---

    fn add_state(&self, state: LvState) -> &Self {
        unsafe { c_bindings::lv_obj_add_state(self.lv_obj().raw(), state.0) };
        self
    }

    fn remove_state(&self, state: LvState) -> &Self {
        unsafe { c_bindings::lv_obj_remove_state(self.lv_obj().raw(), state.0) };
        self
    }

    #[must_use]
    fn has_state(&self, state: LvState) -> bool {
        unsafe { c_bindings::lv_obj_has_state(self.lv_obj().raw(), state.0) }
    }

    /// Returns the object's current width in pixels.
    ///
    /// Only meaningful after the object has been laid out, so call it once the
    /// containing screen has been rendered (e.g. on a user interaction), not
    /// immediately after construction.
    #[must_use]
    fn get_width(&self) -> i32 {
        unsafe { c_bindings::lv_obj_get_width(self.lv_obj().raw()) }
    }

    // --- Flags ---

    fn add_flag(&self, flag: LvObjFlag) -> &Self {
        unsafe { c_bindings::lv_obj_add_flag(self.lv_obj().raw(), flag.0) };
        self
    }

    fn remove_flag(&self, flag: LvObjFlag) -> &Self {
        unsafe { c_bindings::lv_obj_remove_flag(self.lv_obj().raw(), flag.0) };
        self
    }

    #[must_use]
    fn has_flag(&self, flag: LvObjFlag) -> bool {
        unsafe { c_bindings::lv_obj_has_flag(self.lv_obj().raw(), flag.0) }
    }

    // --- Convenience sugar ---

    fn set_hidden(&self, hidden: bool) -> &Self {
        if hidden {
            self.add_flag(LvObjFlag::HIDDEN)
        } else {
            self.remove_flag(LvObjFlag::HIDDEN)
        }
    }

    fn set_disabled(&self, disabled: bool) -> &Self {
        if disabled {
            self.add_state(LvState::DISABLED)
        } else {
            self.remove_state(LvState::DISABLED)
        }
    }

    fn set_checked(&self, checked: bool) -> &Self {
        if checked {
            self.add_state(LvState::CHECKED)
        } else {
            self.remove_state(LvState::CHECKED)
        }
    }

    fn set_checkable(&self, checkable: bool) -> &Self {
        if checkable {
            self.add_flag(LvObjFlag::CHECKABLE)
        } else {
            self.remove_flag(LvObjFlag::CHECKABLE)
        }
    }

    fn set_clickable(&self, clickable: bool) -> &Self {
        if clickable {
            self.add_flag(LvObjFlag::CLICKABLE)
        } else {
            self.remove_flag(LvObjFlag::CLICKABLE)
        }
    }

    fn set_scrollable(&self, scrollable: bool) -> &Self {
        if scrollable {
            self.add_flag(LvObjFlag::SCROLLABLE)
        } else {
            self.remove_flag(LvObjFlag::SCROLLABLE)
        }
    }

    #[must_use]
    fn is_hidden(&self) -> bool {
        self.has_flag(LvObjFlag::HIDDEN)
    }

    #[must_use]
    fn is_disabled(&self) -> bool {
        self.has_state(LvState::DISABLED)
    }

    #[must_use]
    fn is_checked(&self) -> bool {
        self.has_state(LvState::CHECKED)
    }

    // --- Flex (Task 3.1) ---

    fn flex_row(&self) -> &Self {
        self.set_flex_flow(FlexFlow::Row)
    }
    fn flex_col(&self) -> &Self {
        self.set_flex_flow(FlexFlow::Column)
    }
    fn flex_row_wrap(&self) -> &Self {
        self.set_flex_flow(FlexFlow::RowWrap)
    }

    fn set_flex_flow(&self, flow: FlexFlow) -> &Self {
        // SAFETY: integer enum value, no pointer concerns.
        unsafe { c_bindings::lv_obj_set_flex_flow(self.lv_obj().raw(), flow as u32) };
        self
    }

    fn flex_align(&self, main: FlexAlign, cross: FlexAlign, track: FlexAlign) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_flex_align(
                self.lv_obj().raw(),
                main as u32,
                cross as u32,
                track as u32,
            );
        }
        self
    }

    fn flex_grow(&self, grow: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_flex_grow(self.lv_obj().raw(), grow) };
        self
    }

    fn gap(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_row(self.lv_obj().raw(), px, 0);
            c_bindings::lv_obj_set_style_pad_column(self.lv_obj().raw(), px, 0);
        }
        self
    }

    fn pad_all(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_top(self.lv_obj().raw(), px, 0);
            c_bindings::lv_obj_set_style_pad_bottom(self.lv_obj().raw(), px, 0);
            c_bindings::lv_obj_set_style_pad_left(self.lv_obj().raw(), px, 0);
            c_bindings::lv_obj_set_style_pad_right(self.lv_obj().raw(), px, 0);
        }
        self
    }

    fn pad_left(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_pad_left(self.lv_obj().raw(), px, 0) };
        self
    }

    fn pad_right(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_pad_right(self.lv_obj().raw(), px, 0) };
        self
    }

    fn pad_top(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_pad_top(self.lv_obj().raw(), px, 0) };
        self
    }

    fn pad_bottom(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_pad_bottom(self.lv_obj().raw(), px, 0) };
        self
    }

    fn margin_top(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_margin_top(self.lv_obj().raw(), px, 0) };
        self
    }

    fn margin_left(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_margin_left(self.lv_obj().raw(), px, 0) };
        self
    }

    fn margin_bottom(&self, px: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_margin_bottom(self.lv_obj().raw(), px, 0) };
        self
    }

    // --- Sizing (Task 3.2) ---

    fn width(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_width(self.lv_obj().raw(), s.to_lv()) };
        self
    }

    fn height(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_height(self.lv_obj().raw(), s.to_lv()) };
        self
    }

    fn size(&self, w: Size, h: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_size(self.lv_obj().raw(), w.to_lv(), h.to_lv()) };
        self
    }

    fn min_width(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_min_width(self.lv_obj().raw(), s.to_lv(), 0) };
        self
    }

    fn max_width(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_max_width(self.lv_obj().raw(), s.to_lv(), 0) };
        self
    }

    fn min_height(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_min_height(self.lv_obj().raw(), s.to_lv(), 0) };
        self
    }

    fn max_height(&self, s: Size) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_max_height(self.lv_obj().raw(), s.to_lv(), 0) };
        self
    }

    // --- Styling: background (Task 4.2) ---

    fn bg_color(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_bg_color(self.lv_obj().raw(), c.to_lv(), 0) };
        self
    }

    fn bg_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_bg_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    // --- Styling: text (Task 4.2) ---

    fn text_color(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_text_color(self.lv_obj().raw(), c.to_lv(), 0) };
        self
    }

    fn text_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_text_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    // --- Styling: shape (Task 4.2) ---

    /// Set the corner radius for all four corners.
    ///
    /// Accepts any value that converts into [`CornerRadius`], including a plain
    /// `i32` (pixel value) for backward compatibility.
    ///
    /// # Note
    ///
    /// LVGL 9.3 applies a **single uniform radius** to all corners via
    /// `lv_obj_set_style_radius`.  Per-corner control is not supported by this
    /// LVGL version.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// widget.radius(CornerRadius::Full);   // pill / circle shape
    /// widget.radius(CornerRadius::Px(8));  // 8 px rounding
    /// widget.radius(8);                    // same — i32 auto-converts
    /// widget.radius(CornerRadius::None);   // no rounding
    /// ```
    fn radius(&self, r: impl Into<CornerRadius>) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_radius(self.lv_obj().raw(), r.into().into_lv_value(), 0)
        };
        self
    }

    fn opacity(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    // --- Border (Task 4.3) ---

    fn border_color(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_color(self.lv_obj().raw(), c.to_lv(), 0) };
        self
    }

    fn border_width(&self, w: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_width(self.lv_obj().raw(), w, 0) };
        self
    }

    fn border_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    fn border_side(&self, side: BorderSide) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_side(self.lv_obj().raw(), side.0, 0) };
        self
    }

    /// Sets the border color for the pressed state (LV_STATE_PRESSED = 0x0020).
    fn border_color_pressed(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_color(self.lv_obj().raw(), c.to_lv(), 0x0020) };
        self
    }

    /// Sets the border width for the pressed state (LV_STATE_PRESSED = 0x0020).
    fn border_width_pressed(&self, w: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_border_width(self.lv_obj().raw(), w, 0x0020) };
        self
    }

    /// Sets the background color for the pressed state (LV_STATE_PRESSED = 0x0020).
    fn bg_color_pressed(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_bg_color(self.lv_obj().raw(), c.to_lv(), 0x0020) };
        self
    }

    // --- Outline (Task 4.3) ---

    fn outline_color(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_outline_color(self.lv_obj().raw(), c.to_lv(), 0) };
        self
    }

    fn outline_width(&self, w: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_outline_width(self.lv_obj().raw(), w, 0) };
        self
    }

    fn outline_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_outline_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    fn outline_pad(&self, pad: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_outline_pad(self.lv_obj().raw(), pad, 0) };
        self
    }

    // --- Shadow (Task 4.3) ---

    fn shadow_color(&self, c: Color) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_shadow_color(self.lv_obj().raw(), c.to_lv(), 0) };
        self
    }

    fn shadow_width(&self, w: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_shadow_width(self.lv_obj().raw(), w, 0) };
        self
    }

    fn shadow_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_shadow_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    fn shadow_offset(&self, x: i32, y: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_shadow_offset_x(self.lv_obj().raw(), x, 0);
            c_bindings::lv_obj_set_style_shadow_offset_y(self.lv_obj().raw(), y, 0);
        }
        self
    }

    fn shadow_spread(&self, spread: i32) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_shadow_spread(self.lv_obj().raw(), spread, 0) };
        self
    }

    // --- Background image ---

    /// Sets and retains an image to use as the widget's background.
    fn bg_image(&self, src: ImageSrc) -> &Self {
        let obj = self.lv_obj().raw();
        set_retained_src_for_obj(obj, ImageRetentionSlot::Background, src, |src_ptr| unsafe {
            c_bindings::lv_obj_set_style_bg_image_src(obj, src_ptr, 0);
        });
        self
    }

    /// Tiles the background image when `true` (repeats to fill the widget area).
    fn bg_image_tiled(&self, tiled: bool) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_bg_image_tiled(self.lv_obj().raw(), tiled, 0) };
        self
    }

    /// Sets the opacity of the background image (0 = transparent, 255 = opaque).
    fn bg_image_opa(&self, opa: u8) -> &Self {
        unsafe { c_bindings::lv_obj_set_style_bg_image_opa(self.lv_obj().raw(), opa, 0) };
        self
    }

    /// Tints the background image with `color`.
    fn bg_image_recolor(&self, color: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_image_recolor(self.lv_obj().raw(), color.to_lv(), 0);
        }
        self
    }

    /// Controls how strongly the recolor tint is applied (0 = none, 255 = solid color).
    fn bg_image_recolor_opa(&self, opa: u8) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_image_recolor_opa(self.lv_obj().raw(), opa, 0);
        }
        self
    }

    // --- Scrollbar styling (LV_PART_SCROLLBAR) ---

    fn scrollbar_color(&self, c: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_color(
                self.lv_obj().raw(), c.to_lv(), c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_opa(&self, opa: u8) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_opa(
                self.lv_obj().raw(), opa, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_width(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_width(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_pad_left(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_left(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_pad_top(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_top(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_pad_bottom(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_bottom(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_min_height(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_min_height(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_max_height(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_max_height(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_border_color(&self, c: Color) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_border_color(
                self.lv_obj().raw(), c.to_lv(), c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_border_width(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_border_width(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_border_opa(&self, opa: u8) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_border_opa(
                self.lv_obj().raw(), opa, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_pad_right(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_pad_right(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_radius(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_radius(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn scrollbar_length(&self, px: i32) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_length(
                self.lv_obj().raw(), px, c_bindings::LV_PART_SCROLLBAR,
            )
        };
        self
    }

    fn delete(self) {
        // SAFETY: `self` is consumed, preventing use after `lv_obj_delete` invalidates the handle.
        unsafe {
            c_bindings::lv_obj_delete(self.lv_obj().raw());
        }
    }
}

impl Widget for LvObj {
    fn lv_obj(&self) -> &LvObj {
        self
    }
}
#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::CornerRadius;
    use crate::lvgl::LvAlign;
    use crate::lvgl::event::Event;
    use crate::lvgl::obj::Obj;
    use crate::lvgl::screen::Screen;
    use crate::lvgl::state::{LvObjFlag, LvState};
    use crate::lvgl::widget::Widget;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn align_records_correct_enum_value_and_offsets() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.align(LvAlign::Center, 10, -5);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::Align { align, x, y, .. } if *align == 9 && *x == 10 && *y == -5
            )),
            "expected Align{{align:9, x:10, y:-5}}, got: {:?}",
            calls
        );
    }
    #[test]
    fn add_state_has_state_roundtrip() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.add_state(LvState::CHECKED);
        assert!(obj.has_state(LvState::CHECKED));
    }
    #[test]
    fn has_state_false_before_add() {
        let p = parent();
        let obj = Obj::new(&p);
        assert!(!obj.has_state(LvState::FOCUSED));
    }
    #[test]
    fn remove_state_clears_state() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.add_state(LvState::CHECKED);
        obj.remove_state(LvState::CHECKED);
        assert!(!obj.has_state(LvState::CHECKED));
    }
    #[test]
    fn add_flag_has_flag_roundtrip() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.add_flag(LvObjFlag::HIDDEN);
        assert!(obj.has_flag(LvObjFlag::HIDDEN));
    }
    #[test]
    fn remove_flag_clears_flag() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.add_flag(LvObjFlag::HIDDEN);
        obj.remove_flag(LvObjFlag::HIDDEN);
        assert!(!obj.has_flag(LvObjFlag::HIDDEN));
    }
    #[test]
    fn on_click_registers_clicked_event_code() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        fn my_cb(_: Event) {}
        obj.on_click(my_cb);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::AddEventCb { code, .. } if *code == 10
            )),
            "expected AddEventCb{{code:10}}, got: {:?}",
            calls
        );
    }
    #[test]
    fn set_hidden_true_adds_hidden_flag() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.set_hidden(true);
        assert!(obj.has_flag(LvObjFlag::HIDDEN));
    }
    #[test]
    fn set_hidden_false_removes_hidden_flag() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.add_flag(LvObjFlag::HIDDEN);
        obj.set_hidden(false);
        assert!(!obj.has_flag(LvObjFlag::HIDDEN));
    }
    #[test]
    fn set_checked_true_adds_checked_state() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.set_checked(true);
        assert!(obj.has_state(LvState::CHECKED));
    }
    #[test]
    fn set_disabled_true_adds_disabled_state() {
        let p = parent();
        let obj = Obj::new(&p);
        obj.set_disabled(true);
        assert!(obj.has_state(LvState::DISABLED));
    }

    #[test]
    fn radius_px_sends_pixel_value() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.radius(CornerRadius::Px(12));
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::SetStyleRadius { value, .. } if *value == 12
            )),
            "expected SetStyleRadius{{value:12}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn radius_full_sends_radius_circle_value() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.radius(CornerRadius::Full);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::SetStyleRadius { value, .. } if *value == 0x7FFF
            )),
            "expected SetStyleRadius{{value:0x7FFF}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn radius_none_sends_zero() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.radius(CornerRadius::None);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::SetStyleRadius { value, .. } if *value == 0
            )),
            "expected SetStyleRadius{{value:0}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn radius_raw_i32_sends_pixel_value() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.radius(8_i32);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::SetStyleRadius { value, .. } if *value == 8
            )),
            "expected SetStyleRadius{{value:8}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn bg_image_records_spy() {
        use crate::lvgl::image::ImageSrc;
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        obj.bg_image(src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageSrc { .. })),
            "expected SetStyleBgImageSrc in spy: {:?}",
            calls
        );
    }

    #[test]
    fn bg_image_tiled_records_spy() {
        use crate::lvgl::image::ImageSrc;
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        obj.bg_image(src).bg_image_tiled(true);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageTiled { tiled: true, .. })),
            "expected SetStyleBgImageTiled{{true}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn bg_image_opa_records_spy() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.bg_image_opa(200);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageOpa { opa: 200, .. })),
            "expected SetStyleBgImageOpa{{200}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn bg_image_recolor_records_spy() {
        use crate::lvgl::color::Color;
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.bg_image_recolor(Color::hex(0xFF0000));
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageRecolor { .. })),
            "expected SetStyleBgImageRecolor in spy: {:?}",
            calls
        );
    }

    #[test]
    fn bg_image_recolor_opa_records_spy() {
        let p = parent();
        let obj = Obj::new(&p);
        spy_drain();
        obj.bg_image_recolor_opa(128);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageRecolorOpa { opa: 128, .. })),
            "expected SetStyleBgImageRecolorOpa{{128}} in spy: {:?}",
            calls
        );
    }
}
