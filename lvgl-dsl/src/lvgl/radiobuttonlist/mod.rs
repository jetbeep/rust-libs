mod style;
mod trampolines;
mod tree;
mod types;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use super::widget::{LvObj, Widget};

pub use types::{
    RadioButtonEvent, RadioButtonListConfig, RadioButtonListStyle, RadioIndicatorStyle,
};

type ChangeCallback = Box<dyn for<'a> FnMut(RadioButtonEvent<'a>)>;

struct RadioButtonListInner {
    labels: Vec<String>,
    tree: tree::Tree,
    enabled: Vec<bool>,
    selected: Option<usize>,
    cfg: RadioButtonListConfig,
    row_style: RadioButtonListStyle,
    selected_row_style: RadioButtonListStyle,
    label_style: RadioButtonListStyle,
    dim_label_style: RadioButtonListStyle,
    indicator_style: RadioIndicatorStyle,
    selected_indicator_style: RadioIndicatorStyle,
    disabled_row_style: RadioButtonListStyle,
    disabled_label_style: RadioButtonListStyle,
    callback: Option<ChangeCallback>,
}

pub struct RadioButtonList {
    obj: LvObj,
    inner: Rc<RefCell<RadioButtonListInner>>,
    // Box required: LVGL row callbacks store stable RowCtx user-data pointers.
    #[allow(clippy::vec_box)]
    row_ctxs: Vec<Box<trampolines::RowCtx>>,
}

impl Widget for RadioButtonList {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl RadioButtonList {
    pub fn new(parent: &impl Widget, labels: &[&str]) -> Box<Self> {
        Self::with_config(parent, labels, RadioButtonListConfig::default())
    }

    pub fn with_config(
        parent: &impl Widget,
        labels: &[&str],
        cfg: RadioButtonListConfig,
    ) -> Box<Self> {
        Self::with_config_and_dim_labels(parent, labels, &[], cfg)
    }

    /// Like `with_config` but each row additionally shows a secondary dim label
    /// (e.g. dimension text) right after the primary label with no gap between
    /// them. `dim_labels` must be empty **or** the same length as `labels`; a
    /// `dim_labels[i]` that is an empty string hides that row's dim label.
    pub fn with_config_and_dim_labels(
        parent: &impl Widget,
        labels: &[&str],
        dim_labels: &[&str],
        cfg: RadioButtonListConfig,
    ) -> Box<Self> {
        types::assert_valid_options(labels);
        types::assert_valid_dim_labels(labels, dim_labels);
        types::assert_valid_config(cfg);

        let owned_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let owned_dim_labels: Vec<String> = dim_labels.iter().map(|s| s.to_string()).collect();
        let tree =
            unsafe { tree::build(parent.lv_obj().raw(), &owned_labels, &owned_dim_labels, cfg) };
        let root = tree.root;
        let enabled = alloc::vec![true; owned_labels.len()];
        let inner = Rc::new(RefCell::new(RadioButtonListInner {
            labels: owned_labels,
            tree,
            enabled,
            selected: None,
            cfg,
            row_style: RadioButtonListStyle::default(),
            selected_row_style: RadioButtonListStyle::default(),
            label_style: RadioButtonListStyle::default(),
            dim_label_style: RadioButtonListStyle::default(),
            indicator_style: RadioIndicatorStyle::default(),
            selected_indicator_style: RadioIndicatorStyle {
                dot_opa: Some(255),
                ..RadioIndicatorStyle::default()
            },
            disabled_row_style: RadioButtonListStyle::default(),
            disabled_label_style: RadioButtonListStyle::default(),
            callback: None,
        }));

        let mut list = Box::new(Self {
            obj: LvObj::from_raw(root),
            inner: inner.clone(),
            row_ctxs: Vec::new(),
        });

        // SAFETY: RowCtx stores an Rc clone of the separately allocated inner
        // state, so moving the public wrapper cannot invalidate row callbacks.
        for (index, widgets) in inner.borrow().tree.rows.iter().enumerate() {
            let mut ctx = Box::new(trampolines::RowCtx {
                inner: inner.clone(),
                index,
            });
            unsafe { trampolines::register_row(widgets.row, ctx.as_mut() as *mut _) };
            list.row_ctxs.push(ctx);
        }

        list.refresh_all_rows();
        list
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.borrow().labels.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().labels.is_empty()
    }

    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.inner.borrow().selected
    }

    pub fn set_selected(&mut self, selected: Option<usize>) -> &mut Self {
        Self::set_selected_inner(&mut self.inner.borrow_mut(), selected);
        self
    }

    pub fn row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn selected_row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().selected_row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn dim_label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().dim_label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn disabled_row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().disabled_row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn disabled_label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.inner.borrow_mut().disabled_label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
        self.inner.borrow_mut().indicator_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn selected_indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
        self.inner.borrow_mut().selected_indicator_style = style;
        self.refresh_all_rows();
        self
    }

    fn assert_index_inner(inner: &RadioButtonListInner, index: usize, purpose: &str) {
        assert!(
            index < inner.labels.len(),
            "RadioButtonList {} index out of range: {} >= {}",
            purpose,
            index,
            inner.labels.len()
        );
    }

    fn refresh_row_inner(inner: &RadioButtonListInner, index: usize) {
        let widgets = &inner.tree.rows[index];
        style::apply_visuals(
            widgets,
            inner.selected == Some(index),
            inner.enabled[index],
            inner.row_style,
            inner.selected_row_style,
            inner.label_style,
            inner.dim_label_style,
            inner.indicator_style,
            inner.selected_indicator_style,
            inner.disabled_row_style,
            inner.disabled_label_style,
        );
    }

    fn refresh_all_rows(&self) {
        let inner = self.inner.borrow();
        for index in 0..inner.labels.len() {
            Self::refresh_row_inner(&inner, index);
        }
    }

    fn set_selected_inner(inner: &mut RadioButtonListInner, selected: Option<usize>) {
        if let Some(index) = selected {
            Self::assert_index_inner(inner, index, "selection");
        }
        let old = inner.selected;
        inner.selected = selected;
        match (old, selected) {
            (Some(old_index), Some(selected_index)) if old_index != selected_index => {
                Self::refresh_row_inner(inner, old_index);
                Self::refresh_row_inner(inner, selected_index);
            }
            (Some(index), Some(_)) | (Some(index), None) | (None, Some(index)) => {
                Self::refresh_row_inner(inner, index);
            }
            (None, None) => {}
        }
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        Self::assert_index_inner(&inner, index, "enabled");
        inner.enabled[index] = enabled;
        Self::refresh_row_inner(&inner, index);
        drop(inner);
        self
    }

    #[must_use]
    pub fn is_enabled(&self, index: usize) -> bool {
        let inner = self.inner.borrow();
        Self::assert_index_inner(&inner, index, "enabled");
        inner.enabled[index]
    }

    pub fn on_changed<F>(&mut self, f: F) -> &mut Self
    where
        F: for<'a> FnMut(RadioButtonEvent<'a>) + 'static,
    {
        self.inner.borrow_mut().callback = Some(Box::new(f));
        self
    }

    pub fn row_height(&mut self, row_height: i32) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        let mut cfg = inner.cfg;
        cfg.row_height = row_height;
        types::assert_valid_config(cfg);
        inner.cfg = cfg;
        for widgets in &inner.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_size(
                    widgets.row,
                    crate::c_bindings::lv_pct(100),
                    row_height,
                );
            }
        }
        drop(inner);
        self
    }

    pub fn gap(&mut self, gap: i32) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        let mut cfg = inner.cfg;
        cfg.gap = gap;
        types::assert_valid_config(cfg);
        inner.cfg = cfg;
        unsafe {
            crate::c_bindings::lv_obj_set_style_pad_row(inner.tree.root, gap, 0);
        }
        drop(inner);
        self
    }

    pub fn row_padding(&mut self, horizontal: i32, vertical: i32) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        let mut cfg = inner.cfg;
        cfg.pad_h = horizontal;
        cfg.pad_v = vertical;
        types::assert_valid_config(cfg);
        inner.cfg = cfg;
        for widgets in &inner.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_style_pad_left(widgets.row, horizontal, 0);
                crate::c_bindings::lv_obj_set_style_pad_right(widgets.row, horizontal, 0);
                crate::c_bindings::lv_obj_set_style_pad_top(widgets.row, vertical, 0);
                crate::c_bindings::lv_obj_set_style_pad_bottom(widgets.row, vertical, 0);
            }
        }
        drop(inner);
        self
    }

    pub fn indicator_size(&mut self, indicator_size: i32) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        let mut cfg = inner.cfg;
        cfg.indicator_size = indicator_size;
        types::assert_valid_config(cfg);
        inner.cfg = cfg;
        let dot_size = (indicator_size / 2).max(1);
        for widgets in &inner.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_size(
                    widgets.indicator,
                    indicator_size,
                    indicator_size,
                );
                crate::c_bindings::lv_obj_set_size(widgets.inner_dot, dot_size, dot_size);
                crate::c_bindings::lv_obj_align(
                    widgets.inner_dot,
                    super::LvAlign::Center as u32,
                    0,
                    0,
                );
            }
        }
        drop(inner);
        self
    }

    pub fn indicator_label_gap(&mut self, gap: i32) -> &mut Self {
        let mut inner = self.inner.borrow_mut();
        let mut cfg = inner.cfg;
        cfg.indicator_label_gap = gap;
        types::assert_valid_config(cfg);
        inner.cfg = cfg;
        for widgets in &inner.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_style_pad_column(widgets.row, gap, 0);
            }
        }
        drop(inner);
        self
    }

    fn handle_row_clicked(inner: &Rc<RefCell<RadioButtonListInner>>, index: usize) {
        let (label, mut callback) = {
            let mut state = inner.borrow_mut();
            Self::assert_index_inner(&state, index, "selection");
            if !state.enabled[index] {
                return;
            }
            Self::set_selected_inner(&mut state, Some(index));
            let label = state.labels[index].clone();
            let callback = state.callback.take();
            (label, callback)
        };

        if let Some(mut f) = callback.take() {
            f(RadioButtonEvent {
                index,
                label: &label,
            });
            if let Ok(mut state) = inner.try_borrow_mut() {
                if state.callback.is_none() {
                    state.callback = Some(f);
                }
            }
        }
    }

    #[cfg(test)]
    pub fn debug_row_raw_for_test(&self, index: usize) -> usize {
        let inner = self.inner.borrow();
        Self::assert_index_inner(&inner, index, "debug row");
        inner.tree.rows[index].row as usize
    }
}

impl Drop for RadioButtonList {
    fn drop(&mut self) {
        let inner = self.inner.borrow();
        // If the owning view already deleted an ancestor (e.g.
        // `LockerAssignedView::destroy_ui` calling `root.delete()`), LVGL has
        // recursively freed this subtree — root and every row alike. Touching
        // those freed objects here (removing row event callbacks or deleting
        // the root again) is a use-after-free / double-free. Guard on the root:
        // since the rows are descendants of the root, an invalid root means the
        // whole subtree is already gone, so there is nothing left to clean up.
        // SAFETY: `lv_obj_is_valid` never dereferences the pointer; it only
        // checks LVGL's live-object registry.
        if !unsafe { crate::c_bindings::lv_obj_is_valid(inner.tree.root) } {
            return;
        }
        for (widgets, ctx) in inner.tree.rows.iter().zip(self.row_ctxs.iter_mut()) {
            unsafe { trampolines::unregister_row(widgets.row, ctx.as_mut() as *mut _) };
        }
        // Delete the LVGL root container so all rows/labels/indicators are
        // removed from the parent. Without this, dropping the wrapper would
        // leave the widgets on screen and stack on top of a freshly-built
        // list (e.g. when refreshing labels after a language change).
        // SAFETY: `inner.tree.root` was returned by `tree::build` and the
        // validity check above confirms it has not been freed — LVGL unlinks
        // the widget from its parent on delete, so any subsequent parent
        // deletion will not double-free this subtree.
        unsafe { crate::c_bindings::lv_obj_delete(inner.tree.root) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, spy_drain};
    use crate::lvgl::screen::Screen;
    use crate::lvgl::{Color, CornerRadius, FlexAlign, Font};

    fn parent() -> Screen {
        crate::c_bindings::reset_all_thread_local_spy_state();
        Screen::active()
    }

    #[test]
    fn new_builds_root_row_indicator_and_label_for_each_option() {
        let p = parent();
        let list = RadioButtonList::new(&p, &["First", "Second"]);
        assert_eq!(list.len(), 2);

        let calls = spy_drain();
        let obj_creates = calls
            .iter()
            .filter(|c| matches!(c, LvCall::ObjCreate { .. }))
            .count();
        let label_creates = calls
            .iter()
            .filter(|c| matches!(c, LvCall::LabelCreate { .. }))
            .count();
        assert_eq!(
            obj_creates, 9,
            "root + 2 rows + 2 indicators + 2 inner dots + 2 label containers, got {calls:?}"
        );
        assert_eq!(
            label_creates, 4,
            "primary + dim label per option, got {calls:?}"
        );
        assert!(
            calls.iter().any(
                |c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"First\0")
            ),
            "{calls:?}"
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"Second\0")), "{calls:?}");
    }

    #[test]
    fn default_layout_sets_column_root_and_fixed_row_geometry() {
        let p = parent();
        let _list = RadioButtonList::new(&p, &["One"]);

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetFlexFlow { flow: 1, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 44, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 18, h: 18, .. })),
            "{calls:?}"
        );
    }

    #[test]
    fn default_layout_vertically_centers_row_contents() {
        let p = parent();
        let _list = RadioButtonList::new(&p, &["One"]);

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ObjSetFlexAlign { main, cross, track, .. }
                    if *main == FlexAlign::Start as u32
                        && *cross == FlexAlign::Center as u32
                        && *track == FlexAlign::Center as u32
            )),
            "{calls:?}"
        );
    }

    #[test]
    fn label_style_setter_applies_font_to_labels() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One"]);
        spy_drain();

        list.label_style(RadioButtonListStyle {
            text_font: Some(Font::montserrat_20()),
            ..RadioButtonListStyle::default()
        });

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleTextFont { .. })),
            "{calls:?}"
        );
    }

    #[test]
    #[should_panic(expected = "RadioButtonList requires at least one option")]
    fn empty_options_panic() {
        let p = parent();
        let _ = RadioButtonList::new(&p, &[]);
    }

    #[test]
    #[should_panic(
        expected = "RadioButtonList dim_labels must be empty or match labels length (1 != 2)"
    )]
    fn mismatched_dim_labels_panic() {
        let p = parent();
        let _ = RadioButtonList::with_config_and_dim_labels(
            &p,
            &["One", "Two"],
            &["only-one"],
            RadioButtonListConfig::default(),
        );
    }

    #[test]
    #[should_panic(expected = "RadioButtonList horizontal padding must be non-negative, got -1")]
    fn negative_horizontal_padding_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(
            &p,
            &["One"],
            RadioButtonListConfig {
                pad_h: -1,
                ..RadioButtonListConfig::default()
            },
        );
    }

    #[test]
    #[should_panic(expected = "RadioButtonList vertical padding must be non-negative, got -1")]
    fn negative_vertical_padding_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(
            &p,
            &["One"],
            RadioButtonListConfig {
                pad_v: -1,
                ..RadioButtonListConfig::default()
            },
        );
    }

    #[test]
    fn set_selected_updates_state_and_checked_indicator_visuals() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        spy_drain();

        list.selected_indicator_style(RadioIndicatorStyle {
            bg_color: Some(Color::hex(0xFF6600)),
            bg_opa: Some(255),
            border_color: Some(Color::hex(0xFF6600)),
            border_width: Some(2),
            border_opa: Some(255),
            radius: Some(CornerRadius::Full),
            dot_color: None,
            dot_opa: None,
        });
        list.set_selected(Some(1));

        assert_eq!(list.selected(), Some(1));
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 2, .. })),
            "{calls:?}"
        );
    }

    #[test]
    fn set_selected_none_clears_selection() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);

        list.set_selected(Some(0));
        list.set_selected(None);

        assert_eq!(list.selected(), None);
    }

    #[test]
    #[should_panic(expected = "RadioButtonList selection index out of range: 2 >= 2")]
    fn set_selected_out_of_range_panics() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        list.set_selected(Some(2));
    }

    #[test]
    fn row_style_setter_applies_to_rows() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One"]);
        spy_drain();

        list.row_style(RadioButtonListStyle {
            bg_color: Some(Color::hex(0xFFFFFF)),
            bg_opa: Some(255),
            border_color: Some(Color::hex(0x203844)),
            border_width: Some(1),
            border_opa: Some(255),
            radius: Some(CornerRadius::Px(8)),
            text_color: None,
            text_opa: None,
            text_font: None,
        });

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleRadius { value: 8, .. })),
            "{calls:?}"
        );
    }

    #[test]
    fn set_enabled_false_marks_row_disabled_and_preserves_selection() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        list.set_selected(Some(1));
        spy_drain();

        list.set_enabled(1, false);

        assert_eq!(list.selected(), Some(1));
        assert!(!list.is_enabled(1));
        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::AddState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "{calls:?}");
    }

    #[test]
    #[should_panic(expected = "RadioButtonList enabled index out of range: 2 >= 2")]
    fn set_enabled_out_of_range_panics() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        list.set_enabled(2, false);
    }

    #[test]
    fn clicking_enabled_row_selects_then_calls_callback() {
        use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};
        use core::sync::atomic::{AtomicUsize, Ordering};

        static INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);
        INDEX.store(usize::MAX, Ordering::SeqCst);
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        list.on_changed(|event| {
            assert_eq!(event.index, 1);
            assert_eq!(event.label, "Two");
            INDEX.store(event.index, Ordering::SeqCst);
        });

        let row = list.debug_row_raw_for_test(1);
        spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

        assert_eq!(list.selected(), Some(1));
        assert_eq!(INDEX.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn row_callback_survives_moving_wrapper_out_of_box() {
        use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};

        let p = parent();
        let list = *RadioButtonList::new(&p, &["One", "Two"]);
        let row = list.debug_row_raw_for_test(1);

        spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

        assert_eq!(list.selected(), Some(1));
    }
    #[test]
    fn row_callback_may_drop_owner_during_dispatch() {
        use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};
        use alloc::rc::Rc;
        use core::cell::RefCell;

        let p = parent();
        let holder: Rc<RefCell<Option<Box<RadioButtonList>>>> = Rc::new(RefCell::new(None));
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        let row = list.debug_row_raw_for_test(1);
        let holder_for_cb = holder.clone();
        list.on_changed(move |_event| {
            holder_for_cb.borrow_mut().take();
        });
        holder.borrow_mut().replace(list);

        spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

        assert!(holder.borrow().is_none());
    }

    #[test]
    fn row_trampoline_swallows_panicking_callback() {
        use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};

        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        let row = list.debug_row_raw_for_test(1);
        list.on_changed(|_event| panic!("row callback boom"));

        spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

        assert_eq!(list.selected(), Some(1));
    }

    #[test]
    fn clicking_disabled_row_does_not_select_or_call_callback() {
        use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};
        use core::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);
        CALLS.store(0, Ordering::SeqCst);
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        list.set_enabled(1, false);
        list.on_changed(|_event| {
            CALLS.fetch_add(1, Ordering::SeqCst);
        });

        let row = list.debug_row_raw_for_test(1);
        spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

        assert_eq!(list.selected(), None);
        assert_eq!(CALLS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn with_config_applies_custom_row_height_gap_padding_and_indicator_size() {
        let p = parent();
        let _list = RadioButtonList::with_config(
            &p,
            &["One"],
            RadioButtonListConfig {
                row_height: 72,
                gap: 9,
                pad_h: 21,
                pad_v: 22,
                indicator_size: 24,
                indicator_label_gap: 15,
            },
        );

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 72, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetSize { w: 24, h: 24, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStylePadRow { value: 9, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStylePadLeft { value: 21, .. })),
            "{calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStylePadTop { value: 22, .. })),
            "{calls:?}"
        );
    }

    #[test]
    #[should_panic(expected = "RadioButtonList row height must be positive, got 0")]
    fn zero_row_height_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(
            &p,
            &["One"],
            RadioButtonListConfig {
                row_height: 0,
                ..RadioButtonListConfig::default()
            },
        );
    }

    #[test]
    #[should_panic(expected = "RadioButtonList indicator size must be positive, got 0")]
    fn zero_indicator_size_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(
            &p,
            &["One"],
            RadioButtonListConfig {
                indicator_size: 0,
                ..RadioButtonListConfig::default()
            },
        );
    }

    #[test]
    fn chaining_core_setters_returns_mut_self() {
        let p = parent();
        let mut list = RadioButtonList::new(&p, &["One", "Two"]);
        let ptr: *const RadioButtonList = &*list;
        let ret = list
            .row_height(60)
            .gap(4)
            .row_padding(14, 15)
            .indicator_size(20)
            .indicator_label_gap(10)
            .set_selected(Some(0));
        assert!(core::ptr::eq(ret as *const RadioButtonList, ptr));
    }

    #[test]
    fn drop_unregisters_row_callbacks_before_deleting_root() {
        let p = parent();
        let list = RadioButtonList::new(&p, &["First", "Second", "Third"]);
        let row_count = list.len();
        let (root, row_ptrs) = {
            let inner = list.inner.borrow();
            (
                inner.tree.root as usize,
                inner
                    .tree
                    .rows
                    .iter()
                    .map(|w| w.row as usize)
                    .collect::<Vec<_>>(),
            )
        };
        // Drain creation calls so the spy only records Drop's side effects.
        spy_drain();

        drop(list);

        let calls = spy_drain();

        let root_delete_pos = calls
            .iter()
            .position(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == root))
            .unwrap_or_else(|| panic!("expected ObjDelete for root {root:#x}, got: {calls:?}"));

        let callback_removals: Vec<(usize, usize)> = calls
            .iter()
            .enumerate()
            .filter_map(|(idx, call)| match call {
                LvCall::RemoveEventCbWithUserData { obj, .. } => Some((idx, *obj)),
                _ => None,
            })
            .collect();

        assert_eq!(
            callback_removals.len(),
            row_count,
            "expected one callback removal per row, got: {calls:?}",
        );
        for (_, obj) in &callback_removals {
            assert!(
                row_ptrs.contains(obj),
                "callback removal for unknown obj {obj:#x}; expected one of {row_ptrs:?}: {calls:?}",
            );
        }
        assert!(
            callback_removals
                .iter()
                .all(|(idx, _)| *idx < root_delete_pos),
            "row callback removal must happen before deleting root: {calls:?}",
        );
    }

    #[test]
    fn drop_after_parent_deleted_does_not_double_free() {
        let p = parent();
        let list = RadioButtonList::new(&p, &["First", "Second"]);
        let root = list.inner.borrow().tree.root as usize;

        // Simulate the owning view tearing down the whole LVGL subtree first
        // (e.g. LockerAssignedView::destroy_ui calling root.delete()), which
        // recursively frees the radio list's root and rows before the Rust
        // wrapper is dropped.
        unsafe { crate::c_bindings::lv_obj_delete(p.lv_obj().raw()) };
        spy_drain();

        drop(list);

        let calls = spy_drain();
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjDelete { obj } if *obj == root)),
            "must not delete an already-freed root (double free): {calls:?}",
        );
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::RemoveEventCbWithUserData { .. })),
            "must not touch already-freed rows: {calls:?}",
        );
    }
}
