use crate::c_bindings;

use super::image::{ImageRetentionSlot, ImageSrc, set_retained_src_for_obj};
use super::widget::{LvObj, Widget};

// ============================================================
//  ImageButtonState
// ============================================================

/// Visual state selector for [`ImageButton::set_src`].
///
/// Maps to the LVGL `lv_imagebutton_state_t` enum values.
/// Hardcoded values match the stable LVGL v9 ABI.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageButtonState {
    /// Normal, un-pressed state.
    Released = 0,
    /// Actively pressed.
    Pressed = 1,
    /// Widget is disabled (`LV_STATE_DISABLED`).
    Disabled = 2,
    /// Checked and un-pressed (toggle button).
    CheckedReleased = 3,
    /// Checked and pressed.
    CheckedPressed = 4,
    /// Checked and disabled.
    CheckedDisabled = 5,
}

// ============================================================
//  ImageButton widget
// ============================================================

/// LVGL image button widget (`lv_imagebutton`).
///
/// An interactive button whose appearance for each state is controlled by an
/// image source.  Use [`set_src`](ImageButton::set_src) to assign an image to
/// each [`ImageButtonState`] you care about.
///
/// This wrapper uses the **mid-section only** API (left/right sections are
/// passed as `NULL`).  For most icons and square buttons this is sufficient.
///
/// # Kconfig requirements
/// `CONFIG_LV_USE_IMAGEBUTTON=y`
///
/// # Example
/// ```ignore
/// use lvgl_dsl::lvgl::prelude::*;
///
/// // C descriptors (unsafe):
/// // extern "C" { static BTN_RELEASED: core::ffi::c_void; static BTN_PRESSED: core::ffi::c_void; }
/// let src_rel  = unsafe { ImageSrc::descriptor(&raw const BTN_RELEASED) };
/// let src_pres = unsafe { ImageSrc::descriptor(&raw const BTN_PRESSED) };
///
/// let btn = ImageButton::new(&screen)
///     .set_src(ImageButtonState::Released, src_rel)
///     .set_src(ImageButtonState::Pressed,  src_pres)
///     .size(Size::Px(80), Size::Px(80))
///     .align(LvAlign::Center, 0, 0)
///     .on_click(|_| { /* handle */ });
///
/// // File paths (safe):
/// let src_rel  = ImageSrc::file("S:btn/released.bin")?;
/// let src_pres = ImageSrc::file("S:btn/pressed.bin")?;
///
/// let btn = ImageButton::new(&screen)
///     .set_src(ImageButtonState::Released, src_rel)
///     .set_src(ImageButtonState::Pressed,  src_pres)
///     .size(Size::Px(80), Size::Px(80))
///     .align(LvAlign::Center, 0, 0)
///     .on_click(|_| { /* handle */ });
/// ```
pub struct ImageButton {
    obj: LvObj,
}

impl Widget for ImageButton {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl ImageButton {
    /// Creates a new image button widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory or
    /// `LV_USE_IMAGEBUTTON` not enabled in Kconfig).
    pub fn new(parent: &impl Widget) -> ImageButton {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_imagebutton_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_imagebutton_create returned null — check CONFIG_LV_USE_IMAGEBUTTON=y");
        }
        ImageButton {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Assigns and retains an image source for the given button state (mid-section only).
    ///
    /// Call this once per state you want to customise.  States left unset
    /// will inherit the LVGL default appearance.
    pub fn set_src(&self, state: ImageButtonState, src: ImageSrc) -> &Self {
        let obj = self.lv_obj().raw();
        set_retained_src_for_obj(
            obj,
            ImageRetentionSlot::ImageButtonState(state as u32),
            src,
            |src_ptr| unsafe {
                c_bindings::lv_imagebutton_set_src(
                    obj,
                    state as u32,
                    core::ptr::null(),
                    src_ptr,
                    core::ptr::null(),
                );
            },
        );
        self
    }
}

// ============================================================
//  Tests
// ============================================================
#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::image::ImageSrc;
    use crate::lvgl::imagebutton::{ImageButton, ImageButtonState};
    use crate::lvgl::screen::Screen;

    fn setup() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn imagebutton_create_records_spy() {
        let screen = setup();
        let _ = ImageButton::new(&screen);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonCreate { .. })),
            "expected ImageButtonCreate in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_set_src_released_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::Released, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 0, .. })),
            "expected ImageButtonSetSrc{{state=0}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_set_src_pressed_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::Pressed, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 1, .. })),
            "expected ImageButtonSetSrc{{state=1}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_chaining_returns_self() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        let result = btn
            .set_src(ImageButtonState::Released, src.clone())
            .set_src(ImageButtonState::Pressed, src);
        assert!(core::ptr::eq(result, &btn));
    }

    #[test]
    fn imagebutton_set_src_disabled_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::Disabled, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 2, .. })),
            "expected ImageButtonSetSrc{{state=2}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_set_src_checked_released_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::CheckedReleased, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 3, .. })),
            "expected ImageButtonSetSrc{{state=3}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_set_src_checked_pressed_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::CheckedPressed, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 4, .. })),
            "expected ImageButtonSetSrc{{state=4}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn imagebutton_set_src_checked_disabled_records_spy() {
        let screen = setup();
        let btn = ImageButton::new(&screen);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.set_src(ImageButtonState::CheckedDisabled, src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageButtonSetSrc { state: 5, .. })),
            "expected ImageButtonSetSrc{{state=5}} in spy: {:?}",
            calls
        );
    }
}
