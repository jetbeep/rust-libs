use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::c_bindings;

use super::color::Color;
use super::corner_radius::CornerRadius;
use super::font::Font;
use super::image::ImageSrc;
use super::size::Size;
use super::widget::Widget;

/// A native LVGL style object (`lv_style_t`) with a fluent builder API.
///
/// Styles are heap-allocated so their address is stable — LVGL stores a raw
/// pointer into the allocation when you call [`apply`](Style::apply).
///
/// **Lifetime contract**: the `Style` must outlive every widget it is applied
/// to.  Store `Style` values in the same struct as your widget handles and
/// drop them only *after* deleting the LVGL widget tree (i.e. after
/// `root.delete()`).
///
/// ```rust,ignore
/// let style = Style::new()
///     .bg_color(Color::white())
///     .border_color(Color::hex(0xFF7100))
///     .border_width(2)
///     .size(Size::Px(140), Size::Px(200));
///
/// style.apply(&collect_btn);
/// style.apply(&store_btn);
/// ```
pub struct Style {
    /// Heap-allocated so the address never moves — LVGL holds a raw pointer here.
    inner: Box<c_bindings::lv_style_t>,
    image_sources: Vec<ImageSrc>,
}

impl Style {
    pub fn new() -> Self {
        // SAFETY: lv_style_t is a plain C struct; zero-init is valid as the
        // start state before lv_style_init() writes the real initial values.
        let mut inner = Box::new(unsafe { core::mem::zeroed::<c_bindings::lv_style_t>() });
        unsafe { c_bindings::lv_style_init(inner.as_mut()) };
        Self {
            inner,
            image_sources: Vec::new(),
        }
    }

    // SAFETY: inner is heap-allocated; the pointer is stable for the Style's lifetime.
    pub(super) fn as_raw(&self) -> *const c_bindings::lv_style_t {
        self.inner.as_ref()
    }

    // ── Builder methods ────────────────────────────────────────────────────────

    pub fn bg_color(mut self, c: Color) -> Self {
        unsafe { c_bindings::lv_style_set_bg_color(self.inner.as_mut(), c.to_lv()) };
        self
    }

    pub fn bg_opa(mut self, opa: u8) -> Self {
        unsafe { c_bindings::lv_style_set_bg_opa(self.inner.as_mut(), opa) };
        self
    }

    pub fn bg_image(mut self, src: ImageSrc) -> Self {
        let src_ptr = src.as_ptr();
        unsafe { c_bindings::lv_style_set_bg_image_src(self.inner.as_mut(), src_ptr) };
        self.image_sources.push(src);
        self
    }

    pub fn bg_image_opa(mut self, opa: u8) -> Self {
        unsafe { c_bindings::lv_style_set_bg_image_opa(self.inner.as_mut(), opa) };
        self
    }

    pub fn bg_image_tiled(mut self, tiled: bool) -> Self {
        unsafe { c_bindings::lv_style_set_bg_image_tiled(self.inner.as_mut(), tiled) };
        self
    }

    pub fn text_color(mut self, c: Color) -> Self {
        unsafe { c_bindings::lv_style_set_text_color(self.inner.as_mut(), c.to_lv()) };
        self
    }

    pub fn text_font(mut self, f: Font) -> Self {
        unsafe { c_bindings::lv_style_set_text_font(self.inner.as_mut(), f.as_ptr()) };
        self
    }

    pub fn border_color(mut self, c: Color) -> Self {
        unsafe { c_bindings::lv_style_set_border_color(self.inner.as_mut(), c.to_lv()) };
        self
    }

    pub fn border_width(mut self, w: i32) -> Self {
        unsafe { c_bindings::lv_style_set_border_width(self.inner.as_mut(), w) };
        self
    }

    pub fn radius(mut self, r: impl Into<CornerRadius>) -> Self {
        unsafe { c_bindings::lv_style_set_radius(self.inner.as_mut(), r.into().into_lv_value()) };
        self
    }

    pub fn opacity(mut self, opa: u8) -> Self {
        unsafe { c_bindings::lv_style_set_opa(self.inner.as_mut(), opa) };
        self
    }

    pub fn width(mut self, w: Size) -> Self {
        unsafe { c_bindings::lv_style_set_width(self.inner.as_mut(), w.to_lv()) };
        self
    }

    pub fn height(mut self, h: Size) -> Self {
        unsafe { c_bindings::lv_style_set_height(self.inner.as_mut(), h.to_lv()) };
        self
    }

    pub fn size(mut self, w: Size, h: Size) -> Self {
        unsafe {
            c_bindings::lv_style_set_width(self.inner.as_mut(), w.to_lv());
            c_bindings::lv_style_set_height(self.inner.as_mut(), h.to_lv());
        }
        self
    }

    pub fn pad_top(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_top(self.inner.as_mut(), px) };
        self
    }

    pub fn pad_bottom(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_bottom(self.inner.as_mut(), px) };
        self
    }

    pub fn pad_left(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_left(self.inner.as_mut(), px) };
        self
    }

    pub fn pad_right(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_right(self.inner.as_mut(), px) };
        self
    }

    pub fn pad_all(mut self, px: i32) -> Self {
        unsafe {
            c_bindings::lv_style_set_pad_top(self.inner.as_mut(), px);
            c_bindings::lv_style_set_pad_bottom(self.inner.as_mut(), px);
            c_bindings::lv_style_set_pad_left(self.inner.as_mut(), px);
            c_bindings::lv_style_set_pad_right(self.inner.as_mut(), px);
        }
        self
    }

    pub fn pad_row(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_row(self.inner.as_mut(), px) };
        self
    }

    pub fn pad_column(mut self, px: i32) -> Self {
        unsafe { c_bindings::lv_style_set_pad_column(self.inner.as_mut(), px) };
        self
    }

    // ── Application ───────────────────────────────────────────────────────────

    /// Assign this style to `widget` via `lv_obj_add_style` (selector 0 = default state).
    ///
    /// Multiple styles can be stacked on the same widget; later calls take precedence
    /// for properties they set.  This is a convenience mirror of [`Widget::add_style`].
    pub fn apply(&self, widget: &impl Widget) {
        widget.add_style(self);
    }
}

impl Drop for Style {
    fn drop(&mut self) {
        // Frees the LVGL-internal `values_and_props` allocation.
        // Safe only after all widgets using this style have been deleted.
        unsafe { c_bindings::lv_style_reset(self.inner.as_mut()) };
    }
}

/// Owns a collection of [`Style`] values that must outlive the widget tree.
///
/// LVGL holds raw pointers into each `lv_style_t` after `lv_obj_add_style`.
/// `StyleStore` keeps those allocations alive until [`release`](StyleStore::release)
/// is called — which must happen *after* the root widget is deleted.
///
/// ```rust,ignore
/// // build_ui — apply styles while they're locals, then move into the store
/// let btn_style = Style::new().bg_color(Color::white());
/// btn1.add_style(&btn_style);
/// btn2.add_style(&btn_style);
/// self.styles.keep(btn_style);
///
/// // destroy_ui — after root.delete()
/// self.styles.release();
/// ```
pub struct StyleStore(Vec<Style>);

impl StyleStore {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Move `style` into the store and return a reference for immediate use with `add_style`.
    pub fn keep(&mut self, style: Style) -> &Style {
        self.0.push(style);
        self.0.last().unwrap()
    }

    /// Drop all stored styles. Call this only after the LVGL widget tree has been deleted.
    pub fn release(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{spy_drain, LvCall};
    use crate::lvgl::image::ImageSrc;

    #[test]
    fn style_background_image_builders_retain_source_and_call_lvgl() {
        let src = ImageSrc::file("S:assets/main_background.png").unwrap();

        spy_drain();

        let style = Style::new()
            .bg_image(src)
            .bg_image_opa(255)
            .bg_image_tiled(false);

        assert_eq!(style.image_sources.len(), 1);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageSrc { .. })),
            "expected SetStyleBgImageSrc in spy: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageOpa { opa: 255, .. })),
            "expected SetStyleBgImageOpa{{255}} in spy: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgImageTiled { tiled: false, .. })),
            "expected SetStyleBgImageTiled{{false}} in spy: {:?}",
            calls
        );
    }
}
