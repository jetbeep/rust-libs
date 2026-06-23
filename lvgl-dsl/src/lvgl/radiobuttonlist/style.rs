use crate::c_bindings;
use super::tree::RowWidgets;
use super::types::{RadioButtonListStyle, RadioIndicatorStyle};

pub(crate) fn apply_row_style(row: *mut c_bindings::lv_obj_t, style: RadioButtonListStyle) {
    unsafe {
        if let Some(c) = style.bg_color { c_bindings::lv_obj_set_style_bg_color(row, c.to_lv(), 0); }
        if let Some(opa) = style.bg_opa { c_bindings::lv_obj_set_style_bg_opa(row, opa, 0); }
        if let Some(c) = style.border_color { c_bindings::lv_obj_set_style_border_color(row, c.to_lv(), 0); }
        if let Some(w) = style.border_width { c_bindings::lv_obj_set_style_border_width(row, w, 0); }
        if let Some(opa) = style.border_opa { c_bindings::lv_obj_set_style_border_opa(row, opa, 0); }
        if let Some(r) = style.radius { c_bindings::lv_obj_set_style_radius(row, r.into_lv_value(), 0); }
    }
}

pub(crate) fn apply_label_style(label: *mut c_bindings::lv_obj_t, style: RadioButtonListStyle) {
    unsafe {
        if let Some(c) = style.text_color { c_bindings::lv_obj_set_style_text_color(label, c.to_lv(), 0); }
        if let Some(opa) = style.text_opa { c_bindings::lv_obj_set_style_text_opa(label, opa, 0); }
        if let Some(font) = style.text_font { c_bindings::lv_obj_set_style_text_font(label, font.as_ptr(), 0); }
    }
}

pub(crate) fn apply_indicator_style(
    indicator: *mut c_bindings::lv_obj_t,
    inner_dot: *mut c_bindings::lv_obj_t,
    style: RadioIndicatorStyle,
) {
    unsafe {
        if let Some(c) = style.bg_color { c_bindings::lv_obj_set_style_bg_color(indicator, c.to_lv(), 0); }
        if let Some(opa) = style.bg_opa { c_bindings::lv_obj_set_style_bg_opa(indicator, opa, 0); }
        if let Some(c) = style.border_color { c_bindings::lv_obj_set_style_border_color(indicator, c.to_lv(), 0); }
        if let Some(w) = style.border_width { c_bindings::lv_obj_set_style_border_width(indicator, w, 0); }
        if let Some(opa) = style.border_opa { c_bindings::lv_obj_set_style_border_opa(indicator, opa, 0); }
        if let Some(r) = style.radius { c_bindings::lv_obj_set_style_radius(indicator, r.into_lv_value(), 0); }
        if let Some(c) = style.dot_color { c_bindings::lv_obj_set_style_bg_color(inner_dot, c.to_lv(), 0); }
        if let Some(opa) = style.dot_opa { c_bindings::lv_obj_set_style_bg_opa(inner_dot, opa, 0); }
    }
}

pub(crate) fn apply_visuals(
    widgets: &RowWidgets,
    selected: bool,
    enabled: bool,
    row_style: RadioButtonListStyle,
    selected_row_style: RadioButtonListStyle,
    label_style: RadioButtonListStyle,
    dim_label_style: RadioButtonListStyle,
    indicator_style: RadioIndicatorStyle,
    selected_indicator_style: RadioIndicatorStyle,
    disabled_row_style: RadioButtonListStyle,
    disabled_label_style: RadioButtonListStyle,
) {
    apply_row_style(widgets.row, row_style);
    apply_label_style(widgets.label, label_style);
    apply_label_style(widgets.dim_label, dim_label_style);
    apply_indicator_style(widgets.indicator, widgets.inner_dot, indicator_style);
    if selected {
        apply_row_style(widgets.row, selected_row_style);
        apply_indicator_style(widgets.indicator, widgets.inner_dot, selected_indicator_style);
    }
    if !enabled {
        apply_row_style(widgets.row, disabled_row_style);
        apply_label_style(widgets.label, disabled_label_style);
        apply_label_style(widgets.dim_label, disabled_label_style);
    }
    if enabled {
        unsafe { c_bindings::lv_obj_remove_state(widgets.row, super::super::LvState::DISABLED.0); }
    } else {
        unsafe { c_bindings::lv_obj_add_state(widgets.row, super::super::LvState::DISABLED.0); }
    }
}
