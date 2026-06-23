use alloc::rc::Rc;
use core::cell::RefCell;
use core::ffi::c_char;

use crate::c_bindings;

use super::align::LvAlign;
use super::button_loading::{ButtonLoadingConfig, ButtonLoadingState};
use super::image::{Image, ImageSrc};
use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};

/// LVGL button widget (`lv_button`).
///
/// Wraps an `lv_button_create`-allocated object and inherits all layout,
/// style, event, and state methods from the [`Widget`] trait.  Use
/// [`text`](Button::text) to add a centred label child in a single call.
pub struct Button {
    obj: LvObj,
    pub(crate) loading: Rc<RefCell<ButtonLoadingState>>,
}

impl Widget for Button {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Button {
    /// Creates a new button widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Button {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_button_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_button_create returned null");
        }
        Button {
            obj: LvObj::from_raw(obj),
            loading: Rc::new(RefCell::new(ButtonLoadingState::new())),
        }
    }

    /// Wraps a raw `*mut lv_obj_t` pointer in a `Button` without taking ownership semantics.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - `ptr` is non-null and points to a valid `lv_obj_t` of button widget kind
    ///   (i.e. created by `lv_button_create`).
    /// - The wrapper does not outlive the underlying C-owned LVGL object.
    /// - No other wrapper exists for the same pointer that would also attempt to free it
    ///   (LVGL retains ownership of the underlying object in typical usage).
    /// - The pointer is only used on the LVGL thread (LVGL is single-threaded).
    ///
    /// # Warning: Loading State Isolation
    /// Each wrapper instance created via `from_raw()` for the same raw pointer will have
    /// an **independent** loading state. Loading-related APIs should be used through a single
    /// wrapper per LVGL button instance to maintain consistent state. Creating multiple
    /// wrappers for the same button and calling loading APIs on different wrappers will
    /// result in independent state and unexpected behavior.
    pub unsafe fn from_raw(ptr: *mut crate::c_bindings::lv_obj_t) -> Self {
        Button {
            obj: LvObj::from_raw(ptr),
            loading: Rc::new(RefCell::new(ButtonLoadingState::new())),
        }
    }

    /// Convenience: creates an internal Label child, sets its text, and centers it.
    /// The Label handle is managed by LVGL's parent tree — not returned to the caller.
    /// For independent Label control, use `Label::new(&btn)` instead.
    pub fn text(&self, t: &str) -> &Self {
        let c_string = to_null_terminated(t);
        // SAFETY: label pointer is checked non-null; LVGL copies the string before `c_string` drops.
        unsafe {
            let label = c_bindings::lv_label_create(self.lv_obj().raw());
            if label.is_null() {
                panic!("lv_label_create returned null");
            }
            c_bindings::lv_label_set_text(label, c_string.as_ptr() as *const c_char);
            c_bindings::lv_obj_align(label, LvAlign::Center as u32, 0, 0);
        }
        self
    }

    /// Convenience: creates a centred child [`Image`] displaying `src`.
    ///
    /// The image handle is owned by LVGL's parent tree and is not returned.
    /// For independent image control, use `Image::new(&btn)` directly.
    ///
    /// The image source is retained until the child LVGL image object is deleted.
    pub fn icon(&self, src: ImageSrc) -> &Self {
        // Image struct is dropped here — LVGL's parent tree owns the widget.
        let _ = Image::new(self).set_src(src).align(LvAlign::Center, 0, 0);
        self
    }

    /// Configures loading state behavior for this button.
    ///
    /// Sets the loading configuration that will be used when `start_loading()` is called.
    /// Does not start loading immediately.
    pub fn loading_config(&self, cfg: ButtonLoadingConfig) -> &Self {
        self.loading.borrow_mut().config = cfg;
        self
    }

    pub fn start_loading(&self) -> super::button_loading::LoadingHandle {
        let owns_session = super::button_loading::start(self.lv_obj().raw(), &self.loading);
        super::button_loading::LoadingHandle::new(
            self.lv_obj().raw(),
            self.loading.clone(),
            owns_session,
        )
    }

    pub fn is_loading(&self) -> bool {
        super::button_loading::is_loading(&self.loading)
    }

    pub fn set_loading(&self, loading: bool) -> &Self {
        if loading {
            super::button_loading::start(self.lv_obj().raw(), &self.loading);
        } else {
            super::button_loading::finish(self.lv_obj().raw(), &self.loading);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::{reset_obj_pool, spy_drain, LvCall};
    use crate::lvgl::button::Button;
    use crate::lvgl::screen::Screen;
    use crate::lvgl::LvAlign;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_does_not_panic() {
        let p = parent();
        let _ = Button::new(&p);
    }
    #[test]
    fn text_records_label_set_text_call() {
        let p = parent();
        let btn = Button::new(&p);
        spy_drain();
        btn.text("hello");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"hello\0"
            )),
            "expected LabelSetText with b\"hello\\0\", got: {:?}",
            calls
        );
    }
    #[test]
    fn text_centers_child_label() {
        let p = parent();
        let btn = Button::new(&p);
        spy_drain();
        btn.text("ok");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::Align { align, x, y, .. }
                    if *align == LvAlign::Center as u32 && *x == 0 && *y == 0
            )),
            "expected Align{{Center, 0, 0}}, got: {:?}",
            calls
        );
    }
    #[test]
    fn text_empty_string_sends_nul_byte() {
        let p = parent();
        let btn = Button::new(&p);
        spy_drain();
        btn.text("");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"\0"
            )),
            "expected LabelSetText with b\"\\0\", got: {:?}",
            calls
        );
    }

    #[test]
    fn icon_creates_centered_image_child() {
        use crate::lvgl::image::ImageSrc;
        let p = parent();
        let btn = Button::new(&p);
        spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.icon(src);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageCreate { .. })),
            "expected ImageCreate in spy: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageSetSrc { .. })),
            "expected ImageSetSrc in spy: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::Align { align, x, y, .. }
                    if *align == crate::lvgl::LvAlign::Center as u32 && *x == 0 && *y == 0
            )),
            "expected Align{{Center, 0, 0}} in spy: {:?}",
            calls
        );
    }

    #[test]
    fn start_loading_replaces_content_with_spinner_text_and_disables_button() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator};

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Pay");
        spy_drain();

        btn.loading_config(
            ButtonLoadingConfig::new()
                .text("LOADING...")
                .indicator(ButtonLoadingIndicator::Spinner {
                    size_px: 36,
                    spin_ms: 700,
                    arc_length_deg: 120,
                })
                .gap_px(12),
        );

        let _handle = btn.start_loading();
        let calls = spy_drain();

        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjGetChildCount { ret: 1, .. })),
            "expected child snapshot, got: {:?}",
            calls
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::AddState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "expected disabled state, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::AddFlag { flag, .. } if *flag == crate::lvgl::LvObjFlag::HIDDEN.0)), "expected normal child hidden, got: {:?}", calls);
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ObjCreate { .. })),
            "expected loading container, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SpinnerSetAnimParams {
                    spin_ms: 700,
                    arc_length_deg: 120,
                    ..
                }
            )),
            "expected spinner params, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 36, h: 36, .. })),
            "expected spinner size, got: {:?}",
            calls
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"LOADING...\0")), "expected loading text, got: {:?}", calls);
        assert!(btn.is_loading());
    }

    #[test]
    fn start_loading_creates_image_indicator_and_custom_content() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator, ImageSrc, Label};
        use core::sync::atomic::{AtomicUsize, Ordering};

        static CUSTOM_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn custom(parent: &crate::lvgl::LvObj) {
            CUSTOM_CALLS.fetch_add(1, Ordering::SeqCst);
            Label::new(parent).text("custom");
        }

        let p = parent();
        let btn = Button::new(&p);
        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        btn.loading_config(
            ButtonLoadingConfig::new()
                .indicator(ButtonLoadingIndicator::Image {
                    src,
                    size_px: 28,
                    rotation_ms: 600,
                })
                .custom_content(custom),
        );
        spy_drain();

        let _handle = btn.start_loading();
        let calls = spy_drain();

        assert_eq!(CUSTOM_CALLS.load(Ordering::SeqCst), 1);
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageCreate { .. })),
            "expected ImageCreate, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageSetSrc { .. })),
            "expected ImageSetSrc, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 28, h: 28, .. })),
            "expected image size, got: {:?}",
            calls
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::AnimSetRepeatCount { count } if *count == crate::c_bindings::LV_ANIM_REPEAT_INFINITE)), "expected repeated rotation animation, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"custom\0")), "expected custom label, got: {:?}", calls);
    }

    #[test]
    fn set_loading_false_waits_for_min_duration_then_restores() {
        use crate::c_bindings::{spy_fire_timer, spy_live_timer_handles, LvCall};
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        spy_drain();

        btn.set_loading(true);
        assert!(btn.is_loading());
        let handles = spy_live_timer_handles();
        assert_eq!(handles.len(), 1, "expected one min-duration timer");
        spy_drain();

        btn.set_loading(false);
        assert!(
            btn.is_loading(),
            "finish before timer should remain pending"
        );
        let early_calls = spy_drain();
        assert!(
            !early_calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjDelete { .. })),
            "should not restore before timer: {:?}",
            early_calls
        );

        spy_fire_timer(handles[0] as *mut crate::c_bindings::lv_timer_t);
        assert!(!btn.is_loading());
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ObjDelete { .. })),
            "expected loading container delete, got: {:?}",
            calls
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::RemoveFlag { flag, .. } if *flag == crate::lvgl::LvObjFlag::HIDDEN.0)), "expected normal child unhidden, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::RemoveState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "expected disabled state removed, got: {:?}", calls);
        // The min-duration timer is one-shot (repeat_count = 1): firing it
        // consumes it via LVGL's auto-delete, so `restore` must NOT delete it
        // again (that would be a double-free).
        assert!(
            spy_live_timer_handles().is_empty(),
            "fired one-shot timer should be consumed"
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::TimerDelete { .. })),
            "consumed one-shot timer must not be deleted again, got: {:?}",
            calls
        );
    }

    #[test]
    fn loading_handle_finish_and_drop_are_idempotent() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        assert!(btn.is_loading());
        handle.finish();
        assert!(!btn.is_loading());

        let calls = spy_drain();
        let container = calls
            .iter()
            .find_map(|c| match c {
                LvCall::ObjCreate { obj, .. } => Some(*obj),
                _ => None,
            })
            .expect("loading should create a container");
        let deletes = calls
            .iter()
            .filter(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == container))
            .count();
        assert_eq!(
            deletes, 1,
            "finish should delete loading container once: {:?}",
            calls
        );

        let handle = btn.start_loading();
        assert!(btn.is_loading());
        drop(handle);
        assert!(
            !btn.is_loading(),
            "dropping unfinished handle should restore immediately"
        );
    }

    #[test]
    fn loading_handle_drop_respects_min_duration() {
        use crate::c_bindings::{spy_fire_timer, spy_live_timer_handles, LvCall};
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        let handles = spy_live_timer_handles();
        assert_eq!(handles.len(), 1, "expected one min-duration timer");
        spy_drain();

        drop(handle);
        assert!(
            btn.is_loading(),
            "dropping the handle before min duration should leave loading pending"
        );
        let early_calls = spy_drain();
        assert!(
            !early_calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjDelete { .. })),
            "drop should not restore before min duration: {:?}",
            early_calls
        );

        spy_fire_timer(handles[0] as *mut crate::c_bindings::lv_timer_t);

        assert!(!btn.is_loading());
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ObjDelete { .. })),
            "timer should restore after dropped handle requested finish: {:?}",
            calls
        );
    }

    #[test]
    fn repeated_start_and_finish_calls_are_idempotent() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        btn.set_loading(true);
        btn.set_loading(true);
        btn.set_loading(false);
        btn.set_loading(false);

        let calls = spy_drain();
        let container_objs: Vec<usize> = calls
            .iter()
            .filter_map(|c| match c {
                LvCall::ObjCreate { obj, .. } => Some(*obj),
                _ => None,
            })
            .collect();
        let deletes = calls
            .iter()
            .filter(|c| matches!(c, LvCall::ObjDelete { obj } if container_objs.contains(obj)))
            .count();
        assert_eq!(
            container_objs.len(),
            1,
            "second start should not create another container: {:?}",
            calls
        );
        assert_eq!(
            deletes, 1,
            "second finish should not delete twice: {:?}",
            calls
        );
        assert!(!btn.is_loading());
    }

    #[test]
    fn none_indicator_with_text_only_creates_label_without_spinner_or_image() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator};

        let p = parent();
        let btn = Button::new(&p);
        btn.loading_config(
            ButtonLoadingConfig::new()
                .indicator(ButtonLoadingIndicator::None)
                .text("Please wait"),
        );
        spy_drain();

        let handle = btn.start_loading();
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"Please wait\0"
            )),
            "expected text label, got: {:?}",
            calls
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::SpinnerSetAnimParams { .. })),
            "did not expect spinner, got: {:?}",
            calls
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::ImageCreate { .. })),
            "did not expect image, got: {:?}",
            calls
        );
        handle.finish();
    }

    #[test]
    fn duplicate_loading_handle_drop_does_not_restore_active_session() {
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let first = btn.start_loading();
        assert!(btn.is_loading());
        let duplicate = btn.start_loading();

        drop(duplicate);
        assert!(
            btn.is_loading(),
            "duplicate handle should be a no-op on drop"
        );

        first.finish();
        assert!(!btn.is_loading());
    }

    #[test]
    fn loading_restore_preserves_preexisting_disabled_state() {
        use crate::c_bindings::LvCall;
        use crate::lvgl::{ButtonLoadingConfig, LvState, Widget};

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.add_state(LvState::DISABLED);
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        assert!(btn.is_loading());
        handle.finish();
        assert!(!btn.is_loading());
        assert!(btn.has_state(LvState::DISABLED));

        let calls = spy_drain();
        assert!(
            !calls.iter().any(
                |c| matches!(c, LvCall::RemoveState { state, .. } if *state == LvState::DISABLED.0)
            ),
            "preexisting disabled state should not be removed: {:?}",
            calls
        );
    }

    #[test]
    fn loading_restore_preserves_preexisting_hidden_child_state() {
        use crate::lvgl::{ButtonLoadingConfig, Label, LvObjFlag, Widget};

        let p = parent();
        let btn = Button::new(&p);
        let visible_child = Label::new(&btn);
        let hidden_child = Label::new(&btn);
        hidden_child.add_flag(LvObjFlag::HIDDEN);
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        assert!(visible_child.has_flag(LvObjFlag::HIDDEN));
        assert!(hidden_child.has_flag(LvObjFlag::HIDDEN));

        handle.finish();

        assert!(!btn.is_loading());
        assert!(!visible_child.has_flag(LvObjFlag::HIDDEN));
        assert!(hidden_child.has_flag(LvObjFlag::HIDDEN));
    }

    #[test]
    fn finish_after_min_timer_elapsed_does_not_delete_consumed_timer() {
        use crate::c_bindings::{spy_fire_timer, spy_live_timer_handles, LvCall};
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        spy_drain();

        btn.set_loading(true);
        let handles = spy_live_timer_handles();
        assert_eq!(handles.len(), 1, "expected one min-duration timer");
        spy_drain();

        spy_fire_timer(handles[0] as *mut crate::c_bindings::lv_timer_t);
        assert!(
            btn.is_loading(),
            "elapsed minimum should not finish by itself"
        );
        assert!(
            spy_live_timer_handles().is_empty(),
            "one-shot timer should be consumed"
        );
        spy_drain();

        btn.set_loading(false);
        assert!(!btn.is_loading());
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ObjDelete { .. })),
            "expected loading container delete, got: {:?}",
            calls
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::TimerDelete { .. })),
            "elapsed one-shot timer must not be deleted again: {:?}",
            calls
        );
    }

    #[test]
    fn stale_loading_handle_drop_does_not_restore_new_session() {
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let stale = btn.start_loading();
        btn.set_loading(false);
        assert!(!btn.is_loading());

        let _current = btn.start_loading();
        assert!(btn.is_loading());

        drop(stale);
        assert!(
            btn.is_loading(),
            "stale handle from prior session must not restore current session"
        );
    }

    #[test]
    fn stale_loading_handle_finish_does_not_restore_new_session() {
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let stale = btn.start_loading();
        btn.set_loading(false);
        assert!(!btn.is_loading());

        let _current = btn.start_loading();
        assert!(btn.is_loading());

        stale.finish();
        assert!(
            btn.is_loading(),
            "stale handle finish must not restore current session"
        );
    }

    #[test]
    fn button_delete_while_loading_cancels_timer_and_makes_handle_noop() {
        use crate::c_bindings::{
            spy_emit_event, spy_fire_timer, spy_live_timer_handles, LvCall, LV_EVENT_DELETE,
        };
        use crate::lvgl::{ButtonLoadingConfig, Widget};

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        let handles = spy_live_timer_handles();
        assert_eq!(handles.len(), 1, "expected one min-duration timer");
        spy_drain();

        spy_emit_event(btn.lv_obj().raw(), LV_EVENT_DELETE);

        assert!(!btn.is_loading());
        assert!(
            spy_live_timer_handles().is_empty(),
            "delete cleanup should cancel the pending timer"
        );
        spy_drain();

        drop(handle);
        spy_fire_timer(handles[0] as *mut crate::c_bindings::lv_timer_t);

        let calls = spy_drain();
        assert!(
            !calls.iter().any(|c| matches!(
                c,
                LvCall::ObjDelete { .. } | LvCall::RemoveFlag { .. } | LvCall::RemoveState { .. }
            )),
            "stale handle/timer must not touch deleted LVGL objects: {:?}",
            calls
        );
    }

    #[test]
    fn button_delete_callback_unregisters_live_child_and_container_callbacks() {
        use crate::c_bindings::{spy_emit_event, LvCall, LV_EVENT_DELETE};
        use crate::lvgl::{ButtonLoadingConfig, Label, Widget};

        let p = parent();
        let btn = Button::new(&p);
        let child = Label::new(&btn);
        let child_ptr = child.lv_obj().raw() as usize;
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        spy_drain();

        let _handle = btn.start_loading();
        let calls = spy_drain();
        let container = calls
            .iter()
            .find_map(|c| match c {
                LvCall::ObjCreate { obj, .. } => Some(*obj),
                _ => None,
            })
            .expect("loading should create a container");

        spy_emit_event(btn.lv_obj().raw(), LV_EVENT_DELETE);

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::RemoveEventCbWithUserData { obj, .. } if *obj == child_ptr
            )),
            "button delete cleanup must unregister live normal-child callbacks: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::RemoveEventCbWithUserData { obj, .. } if *obj == container
            )),
            "button delete cleanup must unregister live container callback: {:?}",
            calls
        );
    }

    #[test]
    fn parent_delete_cleans_active_button_loading_state() {
        use crate::c_bindings::{lv_obj_delete, spy_live_timer_handles};
        use crate::lvgl::{ButtonLoadingConfig, Widget};

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(500)
                .text("Loading"),
        );
        let handle = btn.start_loading();
        assert!(btn.is_loading());

        unsafe {
            lv_obj_delete(p.lv_obj().raw());
        }

        assert!(!btn.is_loading());
        assert!(
            spy_live_timer_handles().is_empty(),
            "parent delete should cancel child button loading timer"
        );
        drop(handle);
    }

    #[test]
    fn externally_deleted_loading_container_is_not_deleted_again_on_restore() {
        use crate::c_bindings::{lv_obj_delete, LvCall};
        use crate::lvgl::ButtonLoadingConfig;

        let p = parent();
        let btn = Button::new(&p);
        btn.text("Submit");
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );
        spy_drain();

        let handle = btn.start_loading();
        let calls = spy_drain();
        let container = calls
            .iter()
            .find_map(|c| match c {
                LvCall::ObjCreate { obj, .. } => Some(*obj as *mut crate::c_bindings::lv_obj_t),
                _ => None,
            })
            .expect("loading should create a container");

        unsafe {
            lv_obj_delete(container);
        }
        spy_drain();

        handle.finish();

        let calls = spy_drain();
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == container as usize)),
            "restore should not delete an externally deleted loading container again: {:?}",
            calls
        );
    }

    #[test]
    fn deleted_normal_child_is_not_touched_on_restore() {
        use crate::lvgl::{ButtonLoadingConfig, Label, Widget};

        let p = parent();
        let btn = Button::new(&p);
        let child = Label::new(&btn);
        let child_ptr = child.lv_obj().raw() as usize;
        btn.loading_config(
            ButtonLoadingConfig::new()
                .min_duration_ms(0)
                .text("Loading"),
        );

        let handle = btn.start_loading();
        child.delete();
        spy_drain();

        handle.finish();

        let calls = spy_drain();
        assert!(
            !calls.iter().any(|c| matches!(
                c,
                LvCall::RemoveFlag { obj, .. } if *obj == child_ptr
            )),
            "restore must not unhide a deleted normal child: {:?}",
            calls
        );
    }

    #[test]
    fn start_loading_zeroes_button_pad_and_border_to_cover_full_button() {
        use crate::lvgl::ButtonLoadingConfig;
        use crate::lvgl::Widget as _;

        let p = parent();
        let btn = Button::new(&p);
        let btn_ptr = btn.lv_obj().raw() as usize;
        btn.loading_config(ButtonLoadingConfig::new().min_duration_ms(0));
        spy_drain();

        let _handle = btn.start_loading();
        let calls = spy_drain();

        for expected in [
            LvCall::SetStylePadTop { obj: btn_ptr, value: 0 },
            LvCall::SetStylePadBottom { obj: btn_ptr, value: 0 },
            LvCall::SetStylePadLeft { obj: btn_ptr, value: 0 },
            LvCall::SetStylePadRight { obj: btn_ptr, value: 0 },
            LvCall::SetStyleBorderWidth { obj: btn_ptr, value: 0 },
        ] {
            assert!(
                calls.iter().any(|c| *c == expected),
                "expected {:?} on the button itself; got: {:?}",
                expected,
                calls
            );
        }
    }

    #[test]
    fn finish_loading_removes_local_pad_and_border_overrides() {
        use crate::c_bindings;
        use crate::lvgl::ButtonLoadingConfig;
        use crate::lvgl::Widget as _;

        let p = parent();
        let btn = Button::new(&p);
        let btn_ptr = btn.lv_obj().raw() as usize;
        btn.loading_config(ButtonLoadingConfig::new().min_duration_ms(0));

        let handle = btn.start_loading();
        spy_drain();
        handle.finish();

        let calls = spy_drain();
        for prop in [
            crate::lvgl::static_style::LV_STYLE_PAD_TOP,
            crate::lvgl::static_style::LV_STYLE_PAD_BOTTOM,
            crate::lvgl::static_style::LV_STYLE_PAD_LEFT,
            crate::lvgl::static_style::LV_STYLE_PAD_RIGHT,
            crate::lvgl::static_style::LV_STYLE_BORDER_WIDTH,
        ] {
            let expected = LvCall::RemoveLocalStyleProp {
                obj: btn_ptr,
                prop,
                selector: c_bindings::LV_PART_MAIN,
            };
            assert!(
                calls.iter().any(|c| *c == expected),
                "expected {:?} on the button itself; got: {:?}",
                expected,
                calls
            );
        }
    }
}

#[cfg(test)]
mod pub_from_raw_tests {
    use super::*;

    #[test]
    fn pub_from_raw_round_trips_pointer() {
        let p = 0xDEAD_BEEF_usize as *mut crate::c_bindings::lv_obj_t;
        let b = unsafe { Button::from_raw(p) };
        assert_eq!(b.lv_obj().raw(), p);
        std::mem::forget(b);
    }
}
