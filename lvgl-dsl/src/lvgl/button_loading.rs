//! Button loading state scaffolding.
//!
//! # Safety and Invariants: Raw Pointer Cleanup
//!
//! The fields in [`ButtonLoadingState`] hold raw LVGL pointers that require careful management:
//!
//! - **`normal_children`**: Child object pointers and their original hidden flags are
//!   observed only; LVGL owns their lifecycle. Later restore/cleanup logic must not
//!   attempt to delete these pointers.
//!
//! - **`loading_container`**: Container object created during loading. Later restore or drop
//!   logic must delete it with `lv_obj_delete()` when unwinding loading state.
//!
//! - **`min_timer`**: Timer created to enforce minimum duration. Later restore or drop logic
//!   must delete it with `lv_timer_delete()` when unwinding loading state.
//!
//! - **`timer_ctx`**: Boxed timer context allocated for callback handling. Later restore or
//!   drop logic must free the boxed context (via `Box::from_raw()`) when unwinding loading state.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ffi::c_void;

use crate::c_bindings::{self, lv_event_t, lv_obj_t, lv_timer_t};

use super::align::LvAlign;
use super::anim::Anim;
use super::flex::{FlexAlign, FlexFlow};
use super::image::{Image, ImageSrc};
use super::label::Label;
use super::size::Size;
use super::spinner::Spinner;
use super::state::{LvObjFlag, LvState};
use super::static_style::{
    LV_STYLE_BORDER_WIDTH, LV_STYLE_PAD_BOTTOM, LV_STYLE_PAD_LEFT, LV_STYLE_PAD_RIGHT,
    LV_STYLE_PAD_TOP,
};
use super::widget::{LvObj, Widget};

pub type ButtonLoadingCustomContent = fn(&LvObj);
pub type ButtonLoadingContainerStyle = fn(&LvObj);
pub type ButtonLoadingLabelStyle = fn(&Label);
pub type ButtonLoadingSpinnerStyle = fn(&Spinner);

#[derive(Clone)]
pub enum ButtonLoadingIndicator {
    Spinner {
        size_px: i32,
        spin_ms: u32,
        arc_length_deg: u32,
    },
    Image {
        src: ImageSrc,
        size_px: i32,
        rotation_ms: u32,
    },
    None,
}

#[derive(Clone)]
pub struct ButtonLoadingConfig {
    min_duration_ms: u32,
    text: Option<String>,
    indicator: ButtonLoadingIndicator,
    gap_px: i32,
    custom_content: Option<ButtonLoadingCustomContent>,
    container_style: Option<ButtonLoadingContainerStyle>,
    label_style: Option<ButtonLoadingLabelStyle>,
    spinner_style: Option<ButtonLoadingSpinnerStyle>,
}

impl Default for ButtonLoadingConfig {
    fn default() -> Self {
        Self {
            min_duration_ms: 300,
            text: None,
            indicator: ButtonLoadingIndicator::Spinner {
                size_px: 24,
                spin_ms: 900,
                arc_length_deg: 90,
            },
            gap_px: 8,
            custom_content: None,
            container_style: None,
            label_style: None,
            spinner_style: None,
        }
    }
}

impl ButtonLoadingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_duration_ms(mut self, value: u32) -> Self {
        self.min_duration_ms = value;
        self
    }

    pub fn text(mut self, value: &str) -> Self {
        self.text = Some(value.to_string());
        self
    }

    pub fn clear_text(mut self) -> Self {
        self.text = None;
        self
    }

    pub fn indicator(mut self, value: ButtonLoadingIndicator) -> Self {
        self.indicator = value;
        self
    }

    pub fn gap_px(mut self, value: i32) -> Self {
        self.gap_px = value;
        self
    }

    pub fn custom_content(mut self, builder: ButtonLoadingCustomContent) -> Self {
        self.custom_content = Some(builder);
        self
    }

    /// Hook invoked after the loading container is created with its default
    /// flex layout. Use it to override the container's bg color, radius,
    /// border, padding, etc. The container is sized to fully cover the button.
    pub fn container_style(mut self, style: ButtonLoadingContainerStyle) -> Self {
        self.container_style = Some(style);
        self
    }

    /// Hook invoked after the loading label is created. Use it to set the
    /// label's text color, font, etc.
    pub fn label_style(mut self, style: ButtonLoadingLabelStyle) -> Self {
        self.label_style = Some(style);
        self
    }

    /// Hook invoked after the spinner indicator is created. Use it to set
    /// arc colors, stroke width, etc. Has no effect for non-spinner indicators.
    pub fn spinner_style(mut self, style: ButtonLoadingSpinnerStyle) -> Self {
        self.spinner_style = Some(style);
        self
    }

    pub fn min_duration_ms_value(&self) -> u32 {
        self.min_duration_ms
    }

    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn indicator_value(&self) -> &ButtonLoadingIndicator {
        &self.indicator
    }

    pub fn gap_px_value(&self) -> i32 {
        self.gap_px
    }

    pub fn custom_content_value(&self) -> Option<ButtonLoadingCustomContent> {
        self.custom_content
    }

    pub fn container_style_value(&self) -> Option<ButtonLoadingContainerStyle> {
        self.container_style
    }

    pub fn label_style_value(&self) -> Option<ButtonLoadingLabelStyle> {
        self.label_style
    }

    pub fn spinner_style_value(&self) -> Option<ButtonLoadingSpinnerStyle> {
        self.spinner_style
    }
}

pub(crate) struct SavedNormalChild {
    pub(crate) ptr: *mut lv_obj_t,
    pub(crate) was_hidden: bool,
    pub(crate) delete_ctx: *mut ButtonLoadingDeleteCtx,
    pub(crate) deleted: bool,
}

pub(crate) struct ButtonLoadingState {
    pub(crate) config: ButtonLoadingConfig,
    pub(crate) active: bool,
    pub(crate) deleted: bool,
    pub(crate) min_elapsed: bool,
    pub(crate) finish_pending: bool,
    pub(crate) normal_children: Vec<SavedNormalChild>,
    pub(crate) loading_container: *mut lv_obj_t,
    pub(crate) min_timer: *mut lv_timer_t,
    pub(crate) timer_ctx: *mut ButtonLoadingTimerCtx,
    pub(crate) button_delete_ctx: *mut ButtonLoadingDeleteCtx,
    pub(crate) container_delete_ctx: *mut ButtonLoadingDeleteCtx,
    pub(crate) was_disabled: bool,
    pub(crate) session_id: u64,
}

impl ButtonLoadingState {
    pub(crate) fn new() -> Self {
        Self {
            config: ButtonLoadingConfig::default(),
            active: false,
            deleted: false,
            min_elapsed: true,
            finish_pending: false,
            normal_children: Vec::new(),
            loading_container: core::ptr::null_mut(),
            min_timer: core::ptr::null_mut(),
            timer_ctx: core::ptr::null_mut(),
            button_delete_ctx: core::ptr::null_mut(),
            container_delete_ctx: core::ptr::null_mut(),
            was_disabled: false,
            session_id: 0,
        }
    }
}

#[allow(dead_code)]
pub(crate) struct ButtonLoadingTimerCtx {
    pub(crate) button_obj: *mut lv_obj_t,
    pub(crate) state: Rc<RefCell<ButtonLoadingState>>,
}

#[allow(dead_code)]
pub(crate) struct ButtonLoadingDeleteCtx {
    pub(crate) state: Rc<RefCell<ButtonLoadingState>>,
    pub(crate) target: ButtonLoadingDeleteTarget,
    pub(crate) obj: *mut lv_obj_t,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum ButtonLoadingDeleteTarget {
    Button,
    Container,
    NormalChild,
}

#[allow(dead_code)]
pub struct LoadingHandle {
    button_obj: *mut lv_obj_t,
    state: Rc<RefCell<ButtonLoadingState>>,
    session_id: u64,
    finished: bool,
}

impl LoadingHandle {
    pub(crate) fn new(
        button_obj: *mut lv_obj_t,
        state: Rc<RefCell<ButtonLoadingState>>,
        session_id: Option<u64>,
    ) -> Self {
        Self {
            button_obj,
            state,
            session_id: session_id.unwrap_or(0),
            finished: session_id.is_none(),
        }
    }

    pub fn finish(mut self) {
        if !self.finished {
            finish_session(self.button_obj, &self.state, self.session_id);
            self.finished = true;
        }
    }
}

impl Drop for LoadingHandle {
    fn drop(&mut self) {
        if !self.finished {
            finish_session(self.button_obj, &self.state, self.session_id);
            self.finished = true;
        }
    }
}

pub(crate) fn is_loading(state: &Rc<RefCell<ButtonLoadingState>>) -> bool {
    state.borrow().active
}

pub(crate) fn start(
    button_obj: *mut lv_obj_t,
    state: &Rc<RefCell<ButtonLoadingState>>,
) -> Option<u64> {
    if {
        let s = state.borrow();
        s.active || s.deleted
    } {
        return None;
    }

    let was_disabled = unsafe { c_bindings::lv_obj_has_state(button_obj, LvState::DISABLED.0) };

    let session_id = {
        let mut s = state.borrow_mut();
        s.session_id = s.session_id.wrapping_add(1);
        s.active = true;
        s.min_elapsed = false;
        s.finish_pending = false;
        s.normal_children = snapshot_children(button_obj);
        s.was_disabled = was_disabled;
        s.session_id
    };

    unsafe {
        c_bindings::lv_obj_add_state(button_obj, LvState::DISABLED.0);
        // Zero the button's own padding and border so the loading container
        // (sized at lv_pct(100), relative to the button's content area)
        // covers the full button bounding box. Originals are restored in
        // `restore()` via `lv_obj_remove_local_style_prop`.
        //
        // CAVEAT: `restore()` removes any local padding/border style on the
        // button, so a caller that sets per-instance pad via
        // `Widget::pad_top()`/etc. on a loading-enabled button will lose
        // those overrides after the loading session finishes (the button
        // will revert to whatever its static styles define). All current
        // loading-enabled buttons in this workspace style themselves via
        // `add_static_style`, so this is not a regression today; revisit
        // with a snapshot-based approach (requires LVGL static-inline
        // getter wrappers) if a caller needs per-instance overrides.
        c_bindings::lv_obj_set_style_pad_top(button_obj, 0, c_bindings::LV_PART_MAIN);
        c_bindings::lv_obj_set_style_pad_bottom(button_obj, 0, c_bindings::LV_PART_MAIN);
        c_bindings::lv_obj_set_style_pad_left(button_obj, 0, c_bindings::LV_PART_MAIN);
        c_bindings::lv_obj_set_style_pad_right(button_obj, 0, c_bindings::LV_PART_MAIN);
        c_bindings::lv_obj_set_style_border_width(button_obj, 0, c_bindings::LV_PART_MAIN);
    }

    let child_ptrs = state
        .borrow()
        .normal_children
        .iter()
        .map(|child| child.ptr)
        .collect::<Vec<_>>();
    for child in child_ptrs {
        unsafe { c_bindings::lv_obj_add_flag(child, LvObjFlag::HIDDEN.0) };
        let delete_ctx =
            install_delete_callback(child, state, ButtonLoadingDeleteTarget::NormalChild);
        if let Some(saved_child) = state
            .borrow_mut()
            .normal_children
            .iter_mut()
            .find(|saved_child| saved_child.ptr == child)
        {
            saved_child.delete_ctx = delete_ctx;
        }
    }

    let cfg = state.borrow().config.clone();
    let button_delete_ctx =
        install_delete_callback(button_obj, state, ButtonLoadingDeleteTarget::Button);
    state.borrow_mut().button_delete_ctx = button_delete_ctx;
    let container = create_loading_container(button_obj, &cfg);
    let container_delete_ctx =
        install_delete_callback(container, state, ButtonLoadingDeleteTarget::Container);
    {
        let mut s = state.borrow_mut();
        s.loading_container = container;
        s.container_delete_ctx = container_delete_ctx;
    }
    create_loading_content(container, &cfg);
    start_min_timer(button_obj, state);
    Some(session_id)
}

fn install_delete_callback(
    obj: *mut lv_obj_t,
    state: &Rc<RefCell<ButtonLoadingState>>,
    target: ButtonLoadingDeleteTarget,
) -> *mut ButtonLoadingDeleteCtx {
    let raw_ctx = Box::into_raw(Box::new(ButtonLoadingDeleteCtx {
        state: state.clone(),
        target,
        obj,
    }));
    unsafe {
        c_bindings::lv_obj_add_event_cb(
            obj,
            Some(on_loading_object_deleted),
            c_bindings::LV_EVENT_DELETE,
            raw_ctx.cast::<c_void>(),
        );
    }
    raw_ctx
}

fn snapshot_children(button_obj: *mut lv_obj_t) -> Vec<SavedNormalChild> {
    let count = unsafe { c_bindings::lv_obj_get_child_count(button_obj) };
    let mut children = Vec::new();
    for idx in 0..count {
        let child = unsafe { c_bindings::lv_obj_get_child(button_obj, idx as i32) };
        if !child.is_null() {
            let was_hidden = unsafe { c_bindings::lv_obj_has_flag(child, LvObjFlag::HIDDEN.0) };
            children.push(SavedNormalChild {
                ptr: child,
                was_hidden,
                delete_ctx: core::ptr::null_mut(),
                deleted: false,
            });
        }
    }
    children
}

fn create_loading_container(button_obj: *mut lv_obj_t, cfg: &ButtonLoadingConfig) -> *mut lv_obj_t {
    let container = unsafe { c_bindings::lv_obj_create(button_obj) };
    if container.is_null() {
        panic!("lv_obj_create returned null for button loading container");
    }

    let container_widget = LvObj::from_raw(container);
    container_widget
        .size(Size::Pct(100), Size::Pct(100))
        .pad_all(0)
        .border_width(0)
        .remove_flag(LvObjFlag::SCROLLABLE)
        .set_flex_flow(FlexFlow::Row)
        .flex_align(FlexAlign::Center, FlexAlign::Center, FlexAlign::Center)
        .gap(cfg.gap_px_value())
        .align(LvAlign::Center, 0, 0);

    if let Some(style) = cfg.container_style_value() {
        style(&container_widget);
    }

    container
}

fn create_loading_content(container: *mut lv_obj_t, cfg: &ButtonLoadingConfig) {
    let container_widget = LvObj::from_raw(container);

    match cfg.indicator_value() {
        ButtonLoadingIndicator::Spinner {
            size_px,
            spin_ms,
            arc_length_deg,
        } => {
            let spinner = Spinner::new(&container_widget);
            spinner
                .size(Size::Px(*size_px), Size::Px(*size_px))
                .set_anim_params(*spin_ms, *arc_length_deg);
            if let Some(style) = cfg.spinner_style_value() {
                style(&spinner);
            }
        }
        ButtonLoadingIndicator::Image {
            src,
            size_px,
            rotation_ms,
        } => {
            let image_widget = Image::new(&container_widget);
            let image_obj = image_widget.lv_obj().raw();
            image_widget
                .set_src(src.clone())
                .size(Size::Px(*size_px), Size::Px(*size_px));
            if *rotation_ms > 0 {
                start_image_rotation(image_obj, *rotation_ms);
            }
        }
        ButtonLoadingIndicator::None => {}
    }

    if let Some(text) = cfg.text_value() {
        let label = Label::new(&container_widget);
        label.text(text);
        if let Some(style) = cfg.label_style_value() {
            style(&label);
        }
    }

    if let Some(builder) = cfg.custom_content_value() {
        builder(&container_widget);
    }
}

unsafe extern "C" fn rotate_image_exec(var: *mut c_void, angle: i32) {
    unsafe {
        c_bindings::lv_obj_set_style_transform_rotation(var.cast::<lv_obj_t>(), angle, 0);
    }
}

fn start_image_rotation(obj: *mut lv_obj_t, rotation_ms: u32) {
    Anim::new(obj.cast::<c_void>())
        .values(0, 3600)
        .duration_ms(rotation_ms)
        .repeat_count(c_bindings::LV_ANIM_REPEAT_INFINITE)
        .exec_extern(rotate_image_exec)
        .start_detached();
}

fn start_min_timer(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    let min_duration_ms = state.borrow().config.min_duration_ms_value();
    if min_duration_ms == 0 {
        state.borrow_mut().min_elapsed = true;
        return;
    }

    let raw_ctx = Box::into_raw(Box::new(ButtonLoadingTimerCtx {
        button_obj,
        state: state.clone(),
    }));

    let timer = unsafe {
        c_bindings::lv_timer_create(
            Some(on_min_duration_elapsed),
            min_duration_ms,
            raw_ctx.cast::<c_void>(),
        )
    };
    if timer.is_null() {
        unsafe {
            drop(Box::from_raw(raw_ctx));
        }
        restore_now(button_obj, state);
        panic!("lv_timer_create returned null for button loading minimum-duration timer");
    }

    unsafe {
        c_bindings::lv_timer_set_repeat_count(timer, 1);
    }

    let mut s = state.borrow_mut();
    s.min_timer = timer;
    s.timer_ctx = raw_ctx;
}

unsafe extern "C" fn on_min_duration_elapsed(timer: *mut lv_timer_t) {
    let raw_ctx =
        unsafe { c_bindings::lv_timer_get_user_data(timer) }.cast::<ButtonLoadingTimerCtx>();
    if raw_ctx.is_null() {
        return;
    }

    let (button_obj, state) = unsafe {
        let ctx = &*raw_ctx;
        (ctx.button_obj, ctx.state.clone())
    };

    let (should_restore, timer_ctx_to_free) = {
        let mut s = state.borrow_mut();
        if s.deleted || !s.active {
            return;
        }
        s.min_elapsed = true;
        s.min_timer = core::ptr::null_mut();
        let timer_ctx = s.timer_ctx;
        s.timer_ctx = core::ptr::null_mut();
        (s.finish_pending, timer_ctx)
    };

    if should_restore {
        restore(button_obj, &state);
    }

    if !timer_ctx_to_free.is_null() {
        unsafe {
            drop(Box::from_raw(timer_ctx_to_free));
        }
    }
}

unsafe extern "C" fn on_loading_object_deleted(e: *mut lv_event_t) {
    let raw_ctx = unsafe { c_bindings::lv_event_get_user_data(e) }.cast::<ButtonLoadingDeleteCtx>();
    if raw_ctx.is_null() {
        return;
    }

    let (state, target, obj) = unsafe {
        let ctx = &*raw_ctx;
        (ctx.state.clone(), ctx.target, ctx.obj)
    };

    match target {
        ButtonLoadingDeleteTarget::Button => cleanup_button_deleted(&state),
        ButtonLoadingDeleteTarget::Container => cleanup_container_deleted(&state),
        ButtonLoadingDeleteTarget::NormalChild => cleanup_normal_child_deleted(&state, obj),
    }
}

fn cleanup_button_deleted(state: &Rc<RefCell<ButtonLoadingState>>) {
    let (
        normal_children,
        loading_container,
        min_timer,
        timer_ctx,
        button_delete_ctx,
        container_delete_ctx,
    ) = {
        let mut s = state.borrow_mut();
        s.deleted = true;
        s.active = false;
        s.min_elapsed = true;
        s.finish_pending = false;
        let normal_children = core::mem::take(&mut s.normal_children);
        let loading_container = s.loading_container;
        s.loading_container = core::ptr::null_mut();
        let min_timer = s.min_timer;
        let timer_ctx = s.timer_ctx;
        let button_delete_ctx = s.button_delete_ctx;
        let container_delete_ctx = s.container_delete_ctx;
        s.min_timer = core::ptr::null_mut();
        s.timer_ctx = core::ptr::null_mut();
        s.button_delete_ctx = core::ptr::null_mut();
        s.container_delete_ctx = core::ptr::null_mut();
        s.was_disabled = false;
        (
            normal_children,
            loading_container,
            min_timer,
            timer_ctx,
            button_delete_ctx,
            container_delete_ctx,
        )
    };

    unsafe {
        for child in normal_children {
            if !child.delete_ctx.is_null() {
                if !child.deleted {
                    c_bindings::lv_obj_remove_event_cb_with_user_data(
                        child.ptr,
                        Some(on_loading_object_deleted),
                        child.delete_ctx.cast::<c_void>(),
                    );
                }
                drop(Box::from_raw(child.delete_ctx));
            }
        }
        if !loading_container.is_null() && !container_delete_ctx.is_null() {
            c_bindings::lv_obj_remove_event_cb_with_user_data(
                loading_container,
                Some(on_loading_object_deleted),
                container_delete_ctx.cast::<c_void>(),
            );
        }
        if !min_timer.is_null() {
            c_bindings::lv_timer_delete(min_timer);
        }
        if !timer_ctx.is_null() {
            drop(Box::from_raw(timer_ctx));
        }
        if !button_delete_ctx.is_null() {
            drop(Box::from_raw(button_delete_ctx));
        }
        if !container_delete_ctx.is_null() {
            drop(Box::from_raw(container_delete_ctx));
        }
    }
}

fn cleanup_container_deleted(state: &Rc<RefCell<ButtonLoadingState>>) {
    let container_delete_ctx = {
        let mut s = state.borrow_mut();
        s.loading_container = core::ptr::null_mut();
        let container_delete_ctx = s.container_delete_ctx;
        s.container_delete_ctx = core::ptr::null_mut();
        container_delete_ctx
    };

    unsafe {
        if !container_delete_ctx.is_null() {
            drop(Box::from_raw(container_delete_ctx));
        }
    }
}

fn cleanup_normal_child_deleted(state: &Rc<RefCell<ButtonLoadingState>>, child_obj: *mut lv_obj_t) {
    let delete_ctx = {
        let mut s = state.borrow_mut();
        let Some(child) = s
            .normal_children
            .iter_mut()
            .find(|child| child.ptr == child_obj)
        else {
            return;
        };
        child.deleted = true;
        let delete_ctx = child.delete_ctx;
        child.delete_ctx = core::ptr::null_mut();
        delete_ctx
    };

    unsafe {
        if !delete_ctx.is_null() {
            drop(Box::from_raw(delete_ctx));
        }
    }
}

pub(crate) fn finish(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    let should_restore = {
        let mut s = state.borrow_mut();
        if !s.active {
            return;
        }
        if s.min_elapsed {
            true
        } else {
            s.finish_pending = true;
            false
        }
    };

    if should_restore {
        restore(button_obj, state);
    }
}

fn finish_session(
    button_obj: *mut lv_obj_t,
    state: &Rc<RefCell<ButtonLoadingState>>,
    session_id: u64,
) {
    if state_matches_session(state, session_id) {
        finish(button_obj, state);
    }
}

pub(crate) fn restore_now(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    {
        let mut s = state.borrow_mut();
        if !s.active {
            return;
        }
        s.min_elapsed = true;
    }
    restore(button_obj, state);
}

fn state_matches_session(state: &Rc<RefCell<ButtonLoadingState>>, session_id: u64) -> bool {
    let s = state.borrow();
    s.active && s.session_id == session_id
}

fn restore(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    let (
        container,
        normal_children,
        min_timer,
        timer_ctx,
        button_delete_ctx,
        container_delete_ctx,
        remove_disabled,
    ) = {
        let mut s = state.borrow_mut();
        if !s.active {
            return;
        }

        let container = s.loading_container;
        let normal_children = core::mem::take(&mut s.normal_children);
        let min_timer = s.min_timer;
        let timer_ctx = s.timer_ctx;
        let button_delete_ctx = s.button_delete_ctx;
        let container_delete_ctx = s.container_delete_ctx;
        let remove_disabled = !s.was_disabled;

        s.active = false;
        s.min_elapsed = true;
        s.finish_pending = false;
        s.loading_container = core::ptr::null_mut();
        s.min_timer = core::ptr::null_mut();
        s.timer_ctx = core::ptr::null_mut();
        s.button_delete_ctx = core::ptr::null_mut();
        s.container_delete_ctx = core::ptr::null_mut();
        s.was_disabled = false;

        (
            container,
            normal_children,
            min_timer,
            timer_ctx,
            button_delete_ctx,
            container_delete_ctx,
            remove_disabled,
        )
    };

    unsafe {
        if !button_delete_ctx.is_null() {
            c_bindings::lv_obj_remove_event_cb_with_user_data(
                button_obj,
                Some(on_loading_object_deleted),
                button_delete_ctx.cast::<c_void>(),
            );
        }
        if !container.is_null() && !container_delete_ctx.is_null() {
            c_bindings::lv_obj_remove_event_cb_with_user_data(
                container,
                Some(on_loading_object_deleted),
                container_delete_ctx.cast::<c_void>(),
            );
        }
        if !container.is_null() {
            c_bindings::lv_obj_delete(container);
        }
        for child in normal_children {
            if !child.deleted && !child.delete_ctx.is_null() {
                c_bindings::lv_obj_remove_event_cb_with_user_data(
                    child.ptr,
                    Some(on_loading_object_deleted),
                    child.delete_ctx.cast::<c_void>(),
                );
            }
            if !child.deleted && !child.was_hidden {
                c_bindings::lv_obj_remove_flag(child.ptr, LvObjFlag::HIDDEN.0);
            }
            if !child.delete_ctx.is_null() {
                drop(Box::from_raw(child.delete_ctx));
            }
        }
        if remove_disabled {
            c_bindings::lv_obj_remove_state(button_obj, LvState::DISABLED.0);
        }
        // Restore the button's original padding and border by removing the
        // local style overrides set in `start()`. Falls back to whatever
        // static styles the caller had attached. See the matching CAVEAT
        // doc-comment in `start()` for the behavioral implications.
        c_bindings::lv_obj_remove_local_style_prop(
            button_obj,
            LV_STYLE_PAD_TOP,
            c_bindings::LV_PART_MAIN,
        );
        c_bindings::lv_obj_remove_local_style_prop(
            button_obj,
            LV_STYLE_PAD_BOTTOM,
            c_bindings::LV_PART_MAIN,
        );
        c_bindings::lv_obj_remove_local_style_prop(
            button_obj,
            LV_STYLE_PAD_LEFT,
            c_bindings::LV_PART_MAIN,
        );
        c_bindings::lv_obj_remove_local_style_prop(
            button_obj,
            LV_STYLE_PAD_RIGHT,
            c_bindings::LV_PART_MAIN,
        );
        c_bindings::lv_obj_remove_local_style_prop(
            button_obj,
            LV_STYLE_BORDER_WIDTH,
            c_bindings::LV_PART_MAIN,
        );
        if !min_timer.is_null() {
            c_bindings::lv_timer_delete(min_timer);
        }
        if !timer_ctx.is_null() {
            drop(Box::from_raw(timer_ctx));
        }
        if !button_delete_ctx.is_null() {
            drop(Box::from_raw(button_delete_ctx));
        }
        if !container_delete_ctx.is_null() {
            drop(Box::from_raw(container_delete_ctx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lvgl::image::ImageSrc;

    #[test]
    fn default_config_uses_spinner_and_300ms_min_duration() {
        let cfg = ButtonLoadingConfig::default();

        assert_eq!(cfg.min_duration_ms_value(), 300);
        assert_eq!(cfg.gap_px_value(), 8);
        assert!(cfg.text_value().is_none());
        assert!(matches!(
            cfg.indicator_value(),
            ButtonLoadingIndicator::Spinner {
                size_px: 24,
                spin_ms: 900,
                arc_length_deg: 90,
            }
        ));
        assert!(cfg.custom_content_value().is_none());
        assert!(cfg.container_style_value().is_none());
        assert!(cfg.label_style_value().is_none());
        assert!(cfg.spinner_style_value().is_none());
    }

    #[test]
    fn builder_sets_text_indicator_gap_and_custom_content() {
        fn custom(_: &crate::lvgl::LvObj) {}

        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        let cfg = ButtonLoadingConfig::new()
            .text("Loading")
            .min_duration_ms(450)
            .gap_px(14)
            .indicator(ButtonLoadingIndicator::Image {
                src,
                size_px: 32,
                rotation_ms: 800,
            })
            .custom_content(custom);

        assert_eq!(cfg.text_value(), Some("Loading"));
        assert_eq!(cfg.min_duration_ms_value(), 450);
        assert_eq!(cfg.gap_px_value(), 14);
        assert!(cfg.custom_content_value().is_some());
        assert!(matches!(
            cfg.indicator_value(),
            ButtonLoadingIndicator::Image {
                size_px: 32,
                rotation_ms: 800,
                ..
            }
        ));
    }

    #[test]
    fn builder_sets_container_label_and_spinner_style_hooks() {
        fn container(_: &crate::lvgl::LvObj) {}
        fn label(_: &crate::lvgl::Label) {}
        fn spinner(_: &crate::lvgl::Spinner) {}

        let cfg = ButtonLoadingConfig::new()
            .container_style(container)
            .label_style(label)
            .spinner_style(spinner);

        let stored_container = cfg.container_style_value().expect("container_style set");
        let stored_label = cfg.label_style_value().expect("label_style set");
        let stored_spinner = cfg.spinner_style_value().expect("spinner_style set");

        assert_eq!(stored_container as *const () as usize, container as *const () as usize);
        assert_eq!(stored_label as *const () as usize, label as *const () as usize);
        assert_eq!(stored_spinner as *const () as usize, spinner as *const () as usize);
    }
}
