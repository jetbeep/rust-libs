mod types;
mod tree;
mod style;
mod trampolines;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use super::widget::{LvObj, Widget};

pub use types::{RadioButtonEvent, RadioButtonListConfig, RadioButtonListStyle, RadioIndicatorStyle};

type ChangeCallback = Box<dyn for<'a> FnMut(RadioButtonEvent<'a>)>;

// Fields are populated in Task 2 and consumed by later RadioButtonList tasks.
pub struct RadioButtonList {
    obj: LvObj,
    labels: Vec<String>,
    tree: tree::Tree,
    // Box required: LVGL row callbacks store stable RowCtx user-data pointers.
    #[allow(clippy::vec_box)]
    row_ctxs: Vec<Box<trampolines::RowCtx>>,
    enabled: Vec<bool>,
    selected: Option<usize>,
    cfg: RadioButtonListConfig,
    row_style: RadioButtonListStyle,
    selected_row_style: RadioButtonListStyle,
    label_style: RadioButtonListStyle,
    /// Style for the secondary/dim label (e.g. dimensions text). Applied on
    /// top of the base label style — only non-`None` fields override.
    dim_label_style: RadioButtonListStyle,
    indicator_style: RadioIndicatorStyle,
    selected_indicator_style: RadioIndicatorStyle,
    /// Style applied to the row container when the row is disabled.
    disabled_row_style: RadioButtonListStyle,
    /// Style applied to both the primary and dim labels when the row is disabled.
    disabled_label_style: RadioButtonListStyle,
    // Used by Task 4 row-click callback dispatch.
    #[allow(dead_code)]
    callback: RefCell<Option<ChangeCallback>>,
}

impl Widget for RadioButtonList {
    fn lv_obj(&self) -> &LvObj { &self.obj }
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
        types::assert_valid_config(cfg);

        let owned_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let owned_dim_labels: Vec<String> = dim_labels.iter().map(|s| s.to_string()).collect();
        let tree = unsafe { tree::build(parent.lv_obj().raw(), &owned_labels, &owned_dim_labels, cfg) };
        let root = tree.root;
        let enabled = alloc::vec![true; owned_labels.len()];

        let mut list = Box::new(Self {
            obj: LvObj::from_raw(root),
            labels: owned_labels,
            tree,
            row_ctxs: Vec::new(),
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
            callback: RefCell::new(None),
        });

        // SAFETY: RowCtx stores this raw pointer for LVGL event callbacks.
        // Returning `Box<Self>` keeps the RadioButtonList allocation stable, and
        // Drop unregisters row callbacks before the row contexts are freed.
        let list_ptr: *mut RadioButtonList = list.as_mut();
        for (index, widgets) in list.tree.rows.iter().enumerate() {
            let mut ctx = Box::new(trampolines::RowCtx { list: list_ptr, index });
            unsafe { trampolines::register_row(widgets.row, ctx.as_mut() as *mut _) };
            list.row_ctxs.push(ctx);
        }

        list.refresh_all_rows();
        list
    }

    #[must_use]
    pub fn len(&self) -> usize { self.labels.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.labels.is_empty() }

    #[must_use]
    pub fn selected(&self) -> Option<usize> { self.selected }

    pub fn set_selected(&mut self, selected: Option<usize>) -> &mut Self {
        if let Some(index) = selected {
            self.assert_index(index, "selection");
        }
        let old = self.selected;
        self.selected = selected;
        match (old, selected) {
            (Some(old_index), Some(selected_index)) if old_index != selected_index => {
                self.refresh_row(old_index);
                self.refresh_row(selected_index);
            }
            (Some(index), Some(_)) | (Some(index), None) | (None, Some(index)) => {
                self.refresh_row(index);
            }
            (None, None) => {}
        }
        self
    }

    pub fn row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn selected_row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.selected_row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn dim_label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.dim_label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn disabled_row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.disabled_row_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn disabled_label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
        self.disabled_label_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
        self.indicator_style = style;
        self.refresh_all_rows();
        self
    }

    pub fn selected_indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
        self.selected_indicator_style = style;
        self.refresh_all_rows();
        self
    }

    fn assert_index(&self, index: usize, purpose: &str) {
        assert!(
            index < self.labels.len(),
            "RadioButtonList {} index out of range: {} >= {}",
            purpose,
            index,
            self.labels.len()
        );
    }

    fn refresh_row(&self, index: usize) {
        let widgets = &self.tree.rows[index];
        style::apply_visuals(
            widgets,
            self.selected == Some(index),
            self.enabled[index],
            self.row_style,
            self.selected_row_style,
            self.label_style,
            self.dim_label_style,
            self.indicator_style,
            self.selected_indicator_style,
            self.disabled_row_style,
            self.disabled_label_style,
        );
    }

    fn refresh_all_rows(&self) {
        for index in 0..self.labels.len() {
            self.refresh_row(index);
        }
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) -> &mut Self {
        self.assert_index(index, "enabled");
        self.enabled[index] = enabled;
        self.refresh_row(index);
        self
    }

    #[must_use]
    pub fn is_enabled(&self, index: usize) -> bool {
        self.assert_index(index, "enabled");
        self.enabled[index]
    }

    pub fn on_changed<F>(&mut self, f: F) -> &mut Self
    where
        F: for<'a> FnMut(RadioButtonEvent<'a>) + 'static,
    {
        *self.callback.borrow_mut() = Some(Box::new(f));
        self
    }

    pub fn row_height(&mut self, row_height: i32) -> &mut Self {
        let mut cfg = self.cfg;
        cfg.row_height = row_height;
        types::assert_valid_config(cfg);
        self.cfg = cfg;
        for widgets in &self.tree.rows {
            unsafe { crate::c_bindings::lv_obj_set_size(widgets.row, crate::c_bindings::lv_pct(100), row_height); }
        }
        self
    }

    pub fn gap(&mut self, gap: i32) -> &mut Self {
        let mut cfg = self.cfg;
        cfg.gap = gap;
        types::assert_valid_config(cfg);
        self.cfg = cfg;
        unsafe { crate::c_bindings::lv_obj_set_style_pad_row(self.tree.root, gap, 0); }
        self
    }

    pub fn row_padding(&mut self, horizontal: i32, vertical: i32) -> &mut Self {
        let mut cfg = self.cfg;
        cfg.pad_h = horizontal;
        cfg.pad_v = vertical;
        types::assert_valid_config(cfg);
        self.cfg = cfg;
        for widgets in &self.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_style_pad_left(widgets.row, horizontal, 0);
                crate::c_bindings::lv_obj_set_style_pad_right(widgets.row, horizontal, 0);
                crate::c_bindings::lv_obj_set_style_pad_top(widgets.row, vertical, 0);
                crate::c_bindings::lv_obj_set_style_pad_bottom(widgets.row, vertical, 0);
            }
        }
        self
    }

    pub fn indicator_size(&mut self, indicator_size: i32) -> &mut Self {
        let mut cfg = self.cfg;
        cfg.indicator_size = indicator_size;
        types::assert_valid_config(cfg);
        self.cfg = cfg;
        let dot_size = (indicator_size / 2).max(1);
        for widgets in &self.tree.rows {
            unsafe {
                crate::c_bindings::lv_obj_set_size(widgets.indicator, indicator_size, indicator_size);
                crate::c_bindings::lv_obj_set_size(widgets.inner_dot, dot_size, dot_size);
                crate::c_bindings::lv_obj_align(
                    widgets.inner_dot,
                    super::LvAlign::Center as u32,
                    0,
                    0,
                );
            }
        }
        self
    }

    pub fn indicator_label_gap(&mut self, gap: i32) -> &mut Self {
        let mut cfg = self.cfg;
        cfg.indicator_label_gap = gap;
        types::assert_valid_config(cfg);
        self.cfg = cfg;
        for widgets in &self.tree.rows {
            unsafe { crate::c_bindings::lv_obj_set_style_pad_column(widgets.row, gap, 0); }
        }
        self
    }

    pub(crate) fn handle_row_clicked(&mut self, index: usize) {
        self.assert_index(index, "selection");
        if !self.enabled[index] {
            return;
        }
        self.set_selected(Some(index));
        let label = self.labels[index].as_str();
        let event = RadioButtonEvent { index, label };
        let cb = self.callback.get_mut();
        if let Some(f) = cb.as_mut() {
            f(event);
        }
    }

    #[cfg(test)]
    pub fn debug_row_raw_for_test(&self, index: usize) -> usize {
        self.assert_index(index, "debug row");
        self.tree.rows[index].row as usize
    }
}

impl Drop for RadioButtonList {
    fn drop(&mut self) {
        for (widgets, ctx) in self.tree.rows.iter().zip(self.row_ctxs.iter_mut()) {
            unsafe { trampolines::unregister_row(widgets.row, ctx.as_mut() as *mut _) };
        }
        // Delete the LVGL root container so all rows/labels/indicators are
        // removed from the parent. Without this, dropping the wrapper would
        // leave the widgets on screen and stack on top of a freshly-built
        // list (e.g. when refreshing labels after a language change).
        // SAFETY: `self.tree.root` was returned by `tree::build` and has not
        // been freed yet — LVGL unlinks the widget from its parent on delete,
        // so any subsequent parent deletion will not double-free this subtree.
        unsafe { crate::c_bindings::lv_obj_delete(self.tree.root) };
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
        let obj_creates = calls.iter().filter(|c| matches!(c, LvCall::ObjCreate { .. })).count();
        let label_creates = calls.iter().filter(|c| matches!(c, LvCall::LabelCreate { .. })).count();
        assert_eq!(obj_creates, 9, "root + 2 rows + 2 indicators + 2 inner dots + 2 label containers, got {calls:?}");
        assert_eq!(label_creates, 4, "primary + dim label per option, got {calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"First\0")), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"Second\0")), "{calls:?}");
    }

    #[test]
    fn default_layout_sets_column_root_and_fixed_row_geometry() {
        let p = parent();
        let _list = RadioButtonList::new(&p, &["One"]);

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetFlexFlow { flow: 1, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 44, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 18, h: 18, .. })), "{calls:?}");
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
            calls.iter().any(|c| matches!(c, LvCall::SetStyleTextFont { .. })),
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
    #[should_panic(expected = "RadioButtonList horizontal padding must be non-negative, got -1")]
    fn negative_horizontal_padding_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
            pad_h: -1,
            ..RadioButtonListConfig::default()
        });
    }

    #[test]
    #[should_panic(expected = "RadioButtonList vertical padding must be non-negative, got -1")]
    fn negative_vertical_padding_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
            pad_v: -1,
            ..RadioButtonListConfig::default()
        });
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
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 2, .. })), "{calls:?}");
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
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleRadius { value: 8, .. })), "{calls:?}");
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
        use crate::c_bindings::{spy_emit_event, LV_EVENT_CLICKED};
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
    fn clicking_disabled_row_does_not_select_or_call_callback() {
        use crate::c_bindings::{spy_emit_event, LV_EVENT_CLICKED};
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
        let _list = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
            row_height: 72,
            gap: 9,
            pad_h: 21,
            pad_v: 22,
            indicator_size: 24,
            indicator_label_gap: 15,
        });

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 72, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 24, h: 24, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadRow { value: 9, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadLeft { value: 21, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadTop { value: 22, .. })), "{calls:?}");
    }

    #[test]
    #[should_panic(expected = "RadioButtonList row height must be positive, got 0")]
    fn zero_row_height_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
            row_height: 0,
            ..RadioButtonListConfig::default()
        });
    }

    #[test]
    #[should_panic(expected = "RadioButtonList indicator size must be positive, got 0")]
    fn zero_indicator_size_panics() {
        let p = parent();
        let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
            indicator_size: 0,
            ..RadioButtonListConfig::default()
        });
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
        let root = list.tree.root as usize;
        let row_ptrs: Vec<usize> =
            list.tree.rows.iter().map(|w| w.row as usize).collect();
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
            callback_removals.iter().all(|(idx, _)| *idx < root_delete_pos),
            "row callback removal must happen before deleting root: {calls:?}",
        );
    }
}
