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

    /// Converts the widget's internal indexed (I1) canvas buffer into a
    /// directly-drawable RGB565 buffer of the same size.
    ///
    /// # Why
    /// `lv_qrcode` renders into a 1-bpp indexed (I1) canvas. The software
    /// renderer cannot draw indexed images directly — it decodes them into a
    /// full ARGB8888 image-cache entry (`size * size * 4` bytes) that is
    /// subject to LRU eviction. Under a tight image cache that decode can fail
    /// with `cache_evict_one_internal_no_lock: No victim found` (every entry is
    /// locked by the frame being drawn), leaving the QR blank.
    ///
    /// A variable RGB565 image is drawn straight from its buffer with **no**
    /// image-cache entry (the `use_directly` path in `lv_bin_decoder`), so the
    /// QR can never be evicted regardless of cache size. The new buffer
    /// (`size * size * 2` bytes) is owned by the canvas and freed by the
    /// `lv_qrcode` destructor when the widget is deleted.
    ///
    /// Call once, right after a successful [`update`](QrCode::update), passing
    /// the same `dark`/`light` colors. No-op when the internal buffers are
    /// unavailable (e.g. the host test mock) or when allocating the RGB565
    /// buffer fails (the widget then keeps its original I1 buffer).
    ///
    /// # Safety
    /// `size` must equal the value passed to the most recent
    /// [`set_size`](QrCode::set_size) + [`update`](QrCode::update) on this
    /// widget. It bounds the raw pointer walk over the source (I1) canvas
    /// buffer, which is exactly `size * size`; a larger `size` reads/writes
    /// out of bounds (undefined behaviour).
    pub unsafe fn to_rgb565_direct(&self, size: i32, dark: Color, light: Color) {
        if size <= 0 {
            return;
        }
        let obj = self.lv_obj().raw();
        // SAFETY: `obj` is a valid `lv_qrcode` (a canvas subclass) after
        // `update()`; `lv_canvas_get_draw_buf` returns its current draw buffer
        // (or null under the mock / before rendering).
        let old = unsafe { c_bindings::lv_canvas_get_draw_buf(obj) };
        if old.is_null() {
            return;
        }
        // SAFETY: pure allocation call; `0` selects LVGL's automatic stride.
        let new = unsafe {
            c_bindings::lv_draw_buf_create(
                size as u32,
                size as u32,
                c_bindings::LV_COLOR_FORMAT_RGB565,
                0,
            )
        };
        if new.is_null() {
            // Out of memory: keep the I1 buffer so the QR still renders (via
            // the image cache) rather than disappearing entirely.
            return;
        }

        let dark565 = rgb565(dark);
        let light565 = rgb565(light);
        for y in 0..size {
            // SAFETY: both buffers are `size` rows tall; `goto_xy(_, 0, y)`
            // returns the start of row `y` (past the I1 palette on the source).
            let src_row = unsafe { c_bindings::lv_draw_buf_goto_xy(old, 0, y as u32) } as *const u8;
            let dst_row = unsafe { c_bindings::lv_draw_buf_goto_xy(new, 0, y as u32) } as *mut u8;
            if src_row.is_null() || dst_row.is_null() {
                // SAFETY: `new` is a live draw buffer that never got attached.
                unsafe { c_bindings::lv_draw_buf_destroy(new) };
                return;
            }
            for x in 0..size {
                // I1 pixel: bit `7 - (x & 7)` of byte `x / 8`; 1 == dark module
                // (palette index 1), matching `lv_canvas_set_px`'s I1 layout.
                // SAFETY: `x < size`, so `x / 8` is within the row's I1 bytes.
                let byte = unsafe { *src_row.add((x >> 3) as usize) };
                let bit = (byte >> (7 - (x & 7))) & 1;
                let color = if bit == 1 { dark565 } else { light565 };
                // Write the pixel as a native-endian `u16`, matching LVGL's
                // own RGB565 canvas layout (`lv_color16_t`, written natively by
                // `lv_canvas_set_px`). `write_unaligned` makes no alignment
                // assumption on the row pointer.
                // SAFETY: `x < size`, so `x * 2` stays inside the RGB565 row.
                unsafe {
                    (dst_row.add((x as usize) * 2) as *mut u16).write_unaligned(color);
                }
            }
        }

        // SAFETY: `obj` is a valid canvas; `new` is a fully-written RGB565
        // buffer. The canvas takes ownership of `new`.
        unsafe { c_bindings::lv_canvas_set_draw_buf(obj, new) };
        // SAFETY: `old` is now detached from the canvas and must be freed.
        unsafe { c_bindings::lv_draw_buf_destroy(old) };
    }
}

/// Packs an sRGB [`Color`] into a native-endian RGB565 pixel value, matching
/// LVGL's `lv_color16_t` in-RAM layout.
fn rgb565(color: Color) -> u16 {
    let c = color.to_lv();
    ((c.red as u16 >> 3) << 11) | ((c.green as u16 >> 2) << 5) | (c.blue as u16 >> 3)
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
