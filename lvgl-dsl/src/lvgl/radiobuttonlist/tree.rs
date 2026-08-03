use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_char;

use super::types::RadioButtonListConfig;
use crate::c_bindings;

// Indicator and label handles are created in Task 2 and consumed by Task 3 styling.
pub(crate) struct RowWidgets {
    pub row: *mut c_bindings::lv_obj_t,
    pub indicator: *mut c_bindings::lv_obj_t,
    pub inner_dot: *mut c_bindings::lv_obj_t,
    /// Primary label (e.g. the size code "XS"). Styled via `label_style`.
    pub label: *mut c_bindings::lv_obj_t,
    /// Secondary/dim label (e.g. " (W:50 x H:41 x D:52 cm)"). Styled via
    /// `dim_label_style`. Hidden when no dim text was provided.
    pub dim_label: *mut c_bindings::lv_obj_t,
    /// Flex-row wrapper holding `label` (+ optional `dim_label`). Kept so
    /// `wrap_labels` can let it grow and bound the label width for wrapping.
    pub label_container: *mut c_bindings::lv_obj_t,
}

pub(crate) struct Tree {
    pub root: *mut c_bindings::lv_obj_t,
    pub rows: Vec<RowWidgets>,
}

pub(crate) unsafe fn build(
    parent: *mut c_bindings::lv_obj_t,
    labels: &[String],
    dim_labels: &[String],
    cfg: RadioButtonListConfig,
) -> Tree {
    // LV_SIZE_CONTENT — cannot import from size.rs (private const), so duplicate here.
    const LV_SIZE_CONTENT: i32 = 1_073_741_823; // 0x3FFF_FFFF
    let root = unsafe { c_bindings::lv_obj_create(parent) };
    if root.is_null() {
        panic!("lv_obj_create returned null for RadioButtonList root");
    }

    unsafe {
        c_bindings::lv_obj_set_flex_flow(root, super::super::FlexFlow::Column as u32);
        c_bindings::lv_obj_set_style_pad_row(root, cfg.gap, 0);
        c_bindings::lv_obj_set_style_pad_column(root, 0, 0);
    }

    let mut rows = Vec::with_capacity(labels.len());
    for (i, label_text) in labels.iter().enumerate() {
        let row = unsafe { c_bindings::lv_obj_create(root) };
        if row.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList row");
        }
        let indicator = unsafe { c_bindings::lv_obj_create(row) };
        if indicator.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList indicator");
        }
        let inner_dot = unsafe { c_bindings::lv_obj_create(indicator) };
        if inner_dot.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList inner dot");
        }

        // Transparent container holding the primary label and the optional dim
        // label side-by-side with zero column gap.
        let label_container = unsafe { c_bindings::lv_obj_create(row) };
        if label_container.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList label container");
        }
        let label = unsafe { c_bindings::lv_label_create(label_container) };
        if label.is_null() {
            panic!("lv_label_create returned null for RadioButtonList label");
        }
        let dim_label = unsafe { c_bindings::lv_label_create(label_container) };
        if dim_label.is_null() {
            panic!("lv_label_create returned null for RadioButtonList dim label");
        }

        let label_buf = super::super::util::to_null_terminated(label_text);
        let dim_text = dim_labels.get(i).map(|s| s.as_str()).unwrap_or("");
        let dim_buf = if dim_text.is_empty() {
            None
        } else {
            Some(super::super::util::to_null_terminated(dim_text))
        };
        let dot_size = (cfg.indicator_size / 2).max(1);
        unsafe {
            c_bindings::lv_obj_add_flag(row, super::super::LvObjFlag::CLICKABLE.0);
            c_bindings::lv_obj_set_flex_flow(row, super::super::FlexFlow::Row as u32);
            c_bindings::lv_obj_set_flex_align(
                row,
                super::super::FlexAlign::Start as u32,
                super::super::FlexAlign::Center as u32,
                super::super::FlexAlign::Center as u32,
            );
            c_bindings::lv_obj_set_size(row, c_bindings::lv_pct(100), cfg.row_height);
            // Rows are fixed-height pills; never let them scroll. Without this,
            // a label whose line-height slightly exceeds the row's inner height
            // makes the row scrollable and renders a per-row scrollbar.
            c_bindings::lv_obj_remove_flag(row, super::super::LvObjFlag::SCROLLABLE.0);
            c_bindings::lv_obj_set_style_pad_left(row, cfg.pad_h, 0);
            c_bindings::lv_obj_set_style_pad_right(row, cfg.pad_h, 0);
            c_bindings::lv_obj_set_style_pad_top(row, cfg.pad_v, 0);
            c_bindings::lv_obj_set_style_pad_bottom(row, cfg.pad_v, 0);
            c_bindings::lv_obj_set_style_pad_column(row, cfg.indicator_label_gap, 0);
            c_bindings::lv_obj_set_size(indicator, cfg.indicator_size, cfg.indicator_size);
            c_bindings::lv_obj_remove_flag(indicator, super::super::LvObjFlag::SCROLLABLE.0);
            c_bindings::lv_obj_remove_flag(indicator, super::super::LvObjFlag::CLICKABLE.0);
            c_bindings::lv_obj_set_style_pad_top(indicator, 0, 0);
            c_bindings::lv_obj_set_style_pad_bottom(indicator, 0, 0);
            c_bindings::lv_obj_set_style_pad_left(indicator, 0, 0);
            c_bindings::lv_obj_set_style_pad_right(indicator, 0, 0);
            c_bindings::lv_obj_set_size(inner_dot, dot_size, dot_size);
            c_bindings::lv_obj_align(inner_dot, super::super::LvAlign::Center as u32, 0, 0);
            c_bindings::lv_obj_remove_flag(inner_dot, super::super::LvObjFlag::SCROLLABLE.0);
            c_bindings::lv_obj_remove_flag(inner_dot, super::super::LvObjFlag::CLICKABLE.0);
            c_bindings::lv_obj_set_style_pad_top(inner_dot, 0, 0);
            c_bindings::lv_obj_set_style_pad_bottom(inner_dot, 0, 0);
            c_bindings::lv_obj_set_style_pad_left(inner_dot, 0, 0);
            c_bindings::lv_obj_set_style_pad_right(inner_dot, 0, 0);
            c_bindings::lv_obj_set_style_border_width(inner_dot, 0, 0);
            c_bindings::lv_obj_set_style_radius(
                inner_dot,
                super::super::CornerRadius::Full.into_lv_value(),
                0,
            );
            c_bindings::lv_obj_set_style_bg_opa(inner_dot, 0, 0);
            // Label container: invisible flex-row, content-sized, zero gap between children.
            c_bindings::lv_obj_set_size(label_container, LV_SIZE_CONTENT, LV_SIZE_CONTENT);
            c_bindings::lv_obj_set_flex_flow(label_container, super::super::FlexFlow::Row as u32);
            c_bindings::lv_obj_set_flex_align(
                label_container,
                super::super::FlexAlign::Start as u32,
                super::super::FlexAlign::Center as u32,
                super::super::FlexAlign::Center as u32,
            );
            c_bindings::lv_obj_set_style_pad_top(label_container, 0, 0);
            c_bindings::lv_obj_set_style_pad_bottom(label_container, 0, 0);
            c_bindings::lv_obj_set_style_pad_left(label_container, 0, 0);
            c_bindings::lv_obj_set_style_pad_right(label_container, 0, 0);
            c_bindings::lv_obj_set_style_pad_column(label_container, 0, 0);
            c_bindings::lv_obj_set_style_bg_opa(label_container, 0, 0);
            c_bindings::lv_obj_set_style_border_width(label_container, 0, 0);
            c_bindings::lv_obj_remove_flag(label_container, super::super::LvObjFlag::SCROLLABLE.0);
            c_bindings::lv_obj_remove_flag(label_container, super::super::LvObjFlag::CLICKABLE.0);
            c_bindings::lv_label_set_text(label, label_buf.as_ptr() as *const c_char);
            match &dim_buf {
                None => {
                    c_bindings::lv_obj_add_flag(dim_label, super::super::LvObjFlag::HIDDEN.0);
                }
                Some(buf) => {
                    c_bindings::lv_label_set_text(dim_label, buf.as_ptr() as *const c_char);
                }
            }
        }

        rows.push(RowWidgets {
            row,
            indicator,
            inner_dot,
            label,
            dim_label,
            label_container,
        });
    }

    Tree { root, rows }
}
