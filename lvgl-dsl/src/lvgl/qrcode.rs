use crate::c_bindings;

use super::color::Color;
use super::widget::{LvObj, Widget};

/// LVGL QR code widget (`lv_qrcode`).
///
/// Wraps the LVGL canvas-based QR code widget.  The widget is sized
/// square; call [`set_size`](QrCode::set_size) after creation and then
/// [`update`](QrCode::update) to render data.
///
/// # Example
/// ```ignore
/// let qr = QrCode::new(&screen);
/// qr.set_size(200)
///     .align(LvAlign::Center, 0, 0);
/// qr.update(b"https://example.com").unwrap();
/// ```
pub struct QrCode {
    obj: LvObj,
}

impl Widget for QrCode {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl QrCode {
    /// Creates a new QR code widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory or
    /// `LV_USE_QRCODE` / `LV_USE_CANVAS` not enabled in Kconfig).
    pub fn new(parent: &impl Widget) -> QrCode {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_qrcode_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!(
                "lv_qrcode_create returned null — check CONFIG_LV_USE_QRCODE=y and CONFIG_LV_USE_CANVAS=y"
            );
        }
        QrCode {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Sets the square size (width == height) of the QR code in pixels.
    ///
    /// Must be called before [`update`](QrCode::update) — LVGL allocates the
    /// canvas buffer at this size; changing it afterwards requires a new
    /// `update()` call to re-render.
    pub fn set_size(&self, size: i32) -> &Self {
        // SAFETY: `LvObj` is non-null at construction.
        unsafe { c_bindings::lv_qrcode_set_size(self.lv_obj().raw(), size) };
        self
    }

    /// Sets the dark (foreground) color of the QR code modules.
    pub fn dark_color(&self, color: Color) -> &Self {
        // SAFETY: `LvObj` is non-null at construction.
        unsafe { c_bindings::lv_qrcode_set_dark_color(self.lv_obj().raw(), color.to_lv()) };
        self
    }

    /// Sets the light (background) color of the QR code.
    pub fn light_color(&self, color: Color) -> &Self {
        // SAFETY: `LvObj` is non-null at construction.
        unsafe { c_bindings::lv_qrcode_set_light_color(self.lv_obj().raw(), color.to_lv()) };
        self
    }

    /// Renders `data` into the QR code widget.
    ///
    /// Returns `Ok(())` on success, or `Err(())` when the data is too long
    /// to encode at the size set by [`set_size`](QrCode::set_size), or when
    /// `data.len()` exceeds `u32::MAX`.
    #[must_use = "encoding may fail if data is too long for the selected size"]
    pub fn update(&self, data: &[u8]) -> Result<(), ()> {
        let data_len = match u32::try_from(data.len()) {
            Ok(data_len) => data_len,
            Err(_) => return Err(()),
        };

        // SAFETY: `data` is a valid slice; LVGL reads exactly `data_len` bytes.
        let result = unsafe {
            c_bindings::lv_qrcode_update(
                self.lv_obj().raw(),
                data.as_ptr() as *const core::ffi::c_void,
                data_len,
            )
        };
        if result == c_bindings::LV_RESULT_OK {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::qrcode::QrCode;
    use crate::lvgl::screen::Screen;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_does_not_panic() {
        let p = parent();
        let _ = QrCode::new(&p);
    }

    #[test]
    fn set_size_records_call() {
        let p = parent();
        let qr = QrCode::new(&p);
        spy_drain();
        qr.set_size(200);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::QrCodeSetSize { size, .. } if *size == 200)),
            "expected QrCodeSetSize(200), got: {:?}",
            calls
        );
    }

    #[test]
    fn update_ok_on_valid_data() {
        let p = parent();
        let qr = QrCode::new(&p);
        qr.set_size(200);
        assert!(qr.update(b"https://example.com").is_ok());
    }

    #[test]
    fn update_records_data_length() {
        let data = b"hello";
        let p = parent();
        let qr = QrCode::new(&p);
        qr.set_size(150);
        spy_drain();
        let _ = qr.update(data);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::QrCodeUpdate { data_len, .. } if *data_len == u32::try_from(data.len()).unwrap()
            )),
            "expected QrCodeUpdate with data_len={}, got: {:?}",
            data.len(),
            calls
        );
    }
}
