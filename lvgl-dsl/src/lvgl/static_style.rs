use core::ffi::c_void;

use crate::c_bindings;

use super::widget::Widget;

pub type StyleProp = u8;

pub const LV_STYLE_PROP_CONST: u8 = 0xFF;
pub const LV_STYLE_CONST_HAS_GROUP: u32 = 0xFFFF_FFFF;

// LVGL style-property ids (`lv_style_prop_t`). These MUST match the exact LVGL
// version this crate is compiled against. On the desktop simulator build.rs
// parses the real lv_style.h and generates these ids (cfg `gen_style_props`);
// every other build uses the hand-maintained fallback table below.
#[cfg(gen_style_props)]
include!(concat!(env!("OUT_DIR"), "/lv_style_props.rs"));

#[cfg(not(gen_style_props))]
pub use hardcoded_style_props::*;

#[cfg(not(gen_style_props))]
mod hardcoded_style_props {
    use super::StyleProp;

    pub const LV_STYLE_PROP_INV: StyleProp = 0;
    pub const LV_STYLE_ALIGN: StyleProp = 18;
pub const LV_STYLE_ANIM: StyleProp = 116;
pub const LV_STYLE_ANIM_DURATION: StyleProp = 117;
pub const LV_STYLE_ARC_COLOR: StyleProp = 91;
pub const LV_STYLE_ARC_IMAGE_SRC: StyleProp = 96;
pub const LV_STYLE_ARC_OPA: StyleProp = 83;
pub const LV_STYLE_ARC_ROUNDED: StyleProp = 111;
pub const LV_STYLE_ARC_WIDTH: StyleProp = 76;
pub const LV_STYLE_BASE_DIR: StyleProp = 129;
pub const LV_STYLE_BG_COLOR: StyleProp = 73;
pub const LV_STYLE_BG_GRAD: StyleProp = 40;
pub const LV_STYLE_BG_GRAD_COLOR: StyleProp = 44;
pub const LV_STYLE_BG_GRAD_DIR: StyleProp = 41;
pub const LV_STYLE_BG_GRAD_OPA: StyleProp = 43;
pub const LV_STYLE_BG_GRAD_STOP: StyleProp = 46;
pub const LV_STYLE_BG_IMAGE_OPA: StyleProp = 49;
pub const LV_STYLE_BG_IMAGE_RECOLOR: StyleProp = 52;
pub const LV_STYLE_BG_IMAGE_RECOLOR_OPA: StyleProp = 50;
pub const LV_STYLE_BG_IMAGE_SRC: StyleProp = 48;
pub const LV_STYLE_BG_IMAGE_TILED: StyleProp = 51;
pub const LV_STYLE_BG_MAIN_OPA: StyleProp = 42;
pub const LV_STYLE_BG_MAIN_STOP: StyleProp = 45;
pub const LV_STYLE_BG_OPA: StyleProp = 72;
pub const LV_STYLE_BITMAP_MASK_SRC: StyleProp = 121;
pub const LV_STYLE_BLEND_MODE: StyleProp = 122;
pub const LV_STYLE_BLUR_BACKDROP: StyleProp = 137;
pub const LV_STYLE_BLUR_QUALITY: StyleProp = 138;
pub const LV_STYLE_BLUR_RADIUS: StyleProp = 136;
pub const LV_STYLE_BORDER_COLOR: StyleProp = 57;
pub const LV_STYLE_BORDER_OPA: StyleProp = 58;
pub const LV_STYLE_BORDER_POST: StyleProp = 59;
pub const LV_STYLE_BORDER_SIDE: StyleProp = 60;
pub const LV_STYLE_BORDER_WIDTH: StyleProp = 56;
pub const LV_STYLE_CLIP_CORNER: StyleProp = 128;
pub const LV_STYLE_COLOR_FILTER_DSC: StyleProp = 114;
pub const LV_STYLE_COLOR_FILTER_OPA: StyleProp = 115;
pub const LV_STYLE_DROP_SHADOW_COLOR: StyleProp = 147;
pub const LV_STYLE_DROP_SHADOW_OFFSET_X: StyleProp = 145;
pub const LV_STYLE_DROP_SHADOW_OFFSET_Y: StyleProp = 146;
pub const LV_STYLE_DROP_SHADOW_OPA: StyleProp = 148;
pub const LV_STYLE_DROP_SHADOW_QUALITY: StyleProp = 149;
pub const LV_STYLE_DROP_SHADOW_RADIUS: StyleProp = 144;
pub const LV_STYLE_FLEX_CROSS_PLACE: StyleProp = 162;
pub const LV_STYLE_FLEX_FLOW: StyleProp = 160;
pub const LV_STYLE_FLEX_GROW: StyleProp = 164;
pub const LV_STYLE_FLEX_MAIN_PLACE: StyleProp = 161;
pub const LV_STYLE_FLEX_TRACK_PLACE: StyleProp = 163;
pub const LV_STYLE_GRID_CELL_COLUMN_POS: StyleProp = 170;
pub const LV_STYLE_GRID_CELL_COLUMN_SPAN: StyleProp = 171;
pub const LV_STYLE_GRID_CELL_ROW_POS: StyleProp = 173;
pub const LV_STYLE_GRID_CELL_ROW_SPAN: StyleProp = 174;
pub const LV_STYLE_GRID_CELL_X_ALIGN: StyleProp = 172;
pub const LV_STYLE_GRID_CELL_Y_ALIGN: StyleProp = 175;
pub const LV_STYLE_GRID_COLUMN_ALIGN: StyleProp = 168;
pub const LV_STYLE_GRID_COLUMN_DSC_ARRAY: StyleProp = 165;
pub const LV_STYLE_GRID_ROW_ALIGN: StyleProp = 169;
pub const LV_STYLE_GRID_ROW_DSC_ARRAY: StyleProp = 166;
pub const LV_STYLE_HEIGHT: StyleProp = 2;
pub const LV_STYLE_IMAGE_COLORKEY: StyleProp = 106;
pub const LV_STYLE_IMAGE_OPA: StyleProp = 80;
pub const LV_STYLE_IMAGE_RECOLOR: StyleProp = 89;
pub const LV_STYLE_IMAGE_RECOLOR_OPA: StyleProp = 78;
pub const LV_STYLE_LAYOUT: StyleProp = 132;
pub const LV_STYLE_LENGTH: StyleProp = 3;
pub const LV_STYLE_LINE_COLOR: StyleProp = 90;
pub const LV_STYLE_LINE_DASH_GAP: StyleProp = 104;
pub const LV_STYLE_LINE_DASH_WIDTH: StyleProp = 100;
pub const LV_STYLE_LINE_OPA: StyleProp = 82;
pub const LV_STYLE_LINE_ROUNDED: StyleProp = 105;
pub const LV_STYLE_LINE_WIDTH: StyleProp = 75;
pub const LV_STYLE_MARGIN_BOTTOM: StyleProp = 33;
pub const LV_STYLE_MARGIN_LEFT: StyleProp = 34;
pub const LV_STYLE_MARGIN_RIGHT: StyleProp = 35;
pub const LV_STYLE_MARGIN_TOP: StyleProp = 32;
pub const LV_STYLE_MAX_HEIGHT: StyleProp = 11;
pub const LV_STYLE_MAX_WIDTH: StyleProp = 9;
pub const LV_STYLE_MIN_HEIGHT: StyleProp = 10;
pub const LV_STYLE_MIN_WIDTH: StyleProp = 8;
pub const LV_STYLE_OPA: StyleProp = 112;
pub const LV_STYLE_OPA_LAYERED: StyleProp = 113;
pub const LV_STYLE_OUTLINE_COLOR: StyleProp = 65;
pub const LV_STYLE_OUTLINE_OPA: StyleProp = 66;
pub const LV_STYLE_OUTLINE_PAD: StyleProp = 67;
pub const LV_STYLE_OUTLINE_WIDTH: StyleProp = 64;
pub const LV_STYLE_PAD_BOTTOM: StyleProp = 25;
pub const LV_STYLE_PAD_COLUMN: StyleProp = 30;
pub const LV_STYLE_PAD_LEFT: StyleProp = 26;
pub const LV_STYLE_PAD_RADIAL: StyleProp = 28;
pub const LV_STYLE_PAD_RIGHT: StyleProp = 27;
pub const LV_STYLE_PAD_ROW: StyleProp = 29;
pub const LV_STYLE_PAD_TOP: StyleProp = 24;
pub const LV_STYLE_RADIAL_OFFSET: StyleProp = 14;
pub const LV_STYLE_RADIUS: StyleProp = 120;
pub const LV_STYLE_RECOLOR: StyleProp = 130;
pub const LV_STYLE_RECOLOR_OPA: StyleProp = 131;
pub const LV_STYLE_ROTARY_SENSITIVITY: StyleProp = 123;
pub const LV_STYLE_SHADOW_COLOR: StyleProp = 88;
pub const LV_STYLE_SHADOW_OFFSET_X: StyleProp = 97;
pub const LV_STYLE_SHADOW_OFFSET_Y: StyleProp = 98;
pub const LV_STYLE_SHADOW_OPA: StyleProp = 81;
pub const LV_STYLE_SHADOW_SPREAD: StyleProp = 99;
pub const LV_STYLE_SHADOW_WIDTH: StyleProp = 74;
pub const LV_STYLE_TEXT_ALIGN: StyleProp = 101;
pub const LV_STYLE_TEXT_COLOR: StyleProp = 92;
pub const LV_STYLE_TEXT_DECOR: StyleProp = 110;
pub const LV_STYLE_TEXT_FONT: StyleProp = 77;
pub const LV_STYLE_TEXT_LETTER_SPACE: StyleProp = 102;
pub const LV_STYLE_TEXT_LINE_SPACE: StyleProp = 103;
pub const LV_STYLE_TEXT_OPA: StyleProp = 84;
pub const LV_STYLE_TEXT_OUTLINE_STROKE_COLOR: StyleProp = 109;
pub const LV_STYLE_TEXT_OUTLINE_STROKE_OPA: StyleProp = 108;
pub const LV_STYLE_TEXT_OUTLINE_STROKE_WIDTH: StyleProp = 107;
pub const LV_STYLE_TRANSFORM_HEIGHT: StyleProp = 5;
pub const LV_STYLE_TRANSFORM_PIVOT_X: StyleProp = 154;
pub const LV_STYLE_TRANSFORM_PIVOT_Y: StyleProp = 155;
pub const LV_STYLE_TRANSFORM_ROTATION: StyleProp = 156;
pub const LV_STYLE_TRANSFORM_SCALE_X: StyleProp = 152;
pub const LV_STYLE_TRANSFORM_SCALE_Y: StyleProp = 153;
pub const LV_STYLE_TRANSFORM_SKEW_X: StyleProp = 157;
pub const LV_STYLE_TRANSFORM_SKEW_Y: StyleProp = 158;
pub const LV_STYLE_TRANSFORM_WIDTH: StyleProp = 4;
pub const LV_STYLE_TRANSITION: StyleProp = 118;
pub const LV_STYLE_TRANSLATE_RADIAL: StyleProp = 124;
pub const LV_STYLE_TRANSLATE_X: StyleProp = 12;
pub const LV_STYLE_TRANSLATE_Y: StyleProp = 13;
pub const LV_STYLE_WIDTH: StyleProp = 1;
pub const LV_STYLE_X: StyleProp = 16;
pub const LV_STYLE_Y: StyleProp = 17;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union StaticStyleValue {
    pub num: i32,
    pub ptr: *const c_void,
    pub color: c_bindings::lv_color_t,
}

impl StaticStyleValue {
    pub const fn num(v: i32) -> Self {
        Self { num: v }
    }

    pub const fn ptr(v: *const c_void) -> Self {
        Self { ptr: v }
    }

    pub const fn color(v: c_bindings::lv_color_t) -> Self {
        Self { color: v }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StaticStyleProp {
    pub prop: StyleProp,
    pub value: StaticStyleValue,
}

impl StaticStyleProp {
    pub const fn num(prop: StyleProp, value: i32) -> Self {
        Self {
            prop,
            value: StaticStyleValue::num(value),
        }
    }

    pub const fn ptr(prop: StyleProp, value: *const c_void) -> Self {
        Self {
            prop,
            value: StaticStyleValue::ptr(value),
        }
    }

    pub const fn color(prop: StyleProp, value: c_bindings::lv_color_t) -> Self {
        Self {
            prop,
            value: StaticStyleValue::color(value),
        }
    }

    // Rust analogue of LV_STYLE_CONST_PROPS_END.
    pub const fn end() -> Self {
        Self {
            prop: LV_STYLE_PROP_INV,
            value: StaticStyleValue::num(0),
        }
    }
}

// Layout-compatible mirror of lv_style_t used for static styles.
// This follows LV_USE_ASSERT_STYLE == 0 layout.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StaticStyle {
    pub values_and_props: *mut c_void,
    pub has_group: u32,
    pub prop_cnt: u8,
}

impl StaticStyle {
    // Rust analogue of LV_STYLE_CONST_INIT(style_name, prop_array).
    pub const fn init(props: &'static [StaticStyleProp]) -> Self {
        Self {
            values_and_props: props.as_ptr() as *mut c_void,
            has_group: LV_STYLE_CONST_HAS_GROUP,
            prop_cnt: LV_STYLE_PROP_CONST,
        }
    }

    pub const fn as_lv_style_ptr(&self) -> *const c_bindings::lv_style_t {
        (self as *const StaticStyle).cast::<c_bindings::lv_style_t>()
    }

    /// Assign this static style to `widget` with selector 0 (default state).
    ///
    /// Mirrors `Style::apply` naming for API consistency.
    pub fn apply(&self, widget: &impl Widget) {
        self.apply_with_selector(widget, 0);
    }

    /// Assign this static style to `widget` with an explicit selector.
    pub fn apply_with_selector(&self, widget: &impl Widget, selector: u32) {
        unsafe {
            c_bindings::lv_obj_add_style(widget.lv_obj().raw(), self.as_lv_style_ptr(), selector);
        }
    }
}

pub fn add_to_widget(style: &'static StaticStyle, widget: &impl Widget, selector: u32) {
    style.apply_with_selector(widget, selector);
}

#[macro_export]
macro_rules! lv_style_const_num {
    ($prop:expr, $val:expr) => {
        $crate::StaticStyleProp::num($prop as u8, $val as i32)
    };
}

#[macro_export]
macro_rules! lv_style_const_ptr {
    ($prop:expr, $ptr:expr) => {
        $crate::StaticStyleProp::ptr($prop as u8, $ptr as *const core::ffi::c_void)
    };
}

#[macro_export]
macro_rules! lv_style_const_color {
    ($prop:expr, $color:expr) => {
        $crate::StaticStyleProp::color($prop as u8, $color)
    };
}

#[macro_export]
macro_rules! LV_COLOR_RGB {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::lv_color_t {
            red: $r as u8,
            green: $g as u8,
            blue: $b as u8,
        }
    };
}

#[macro_export]
macro_rules! LV_COLOR_HEX {
    ($hex:expr) => {
        $crate::LV_COLOR_RGB!(
            ((($hex) >> 16) & 0xFF) as u8,
            ((($hex) >> 8) & 0xFF) as u8,
            (($hex) & 0xFF) as u8
        )
    };
}

// Optional shorthand alias if you prefer a shorter call site.
#[macro_export]
macro_rules! LV_COLOR {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::LV_COLOR_RGB!($r, $g, $b)
    };
}

// Generate analogues for all LV_STYLE_CONST_* generated by LVGL.
macro_rules! define_lv_style_const_macros {
    ($( $macro_name:ident => ($prop_id:ident, num) ),* $(,)?) => {
        $(
            #[macro_export]
            macro_rules! $macro_name {
                ($val:expr) => {
                    $crate::StaticStyleProp::num(
                        $crate::static_style::$prop_id,
                        $val as i32,
                    )
                };
            }
        )*
    };
    ($( $macro_name:ident => ($prop_id:ident, ptr) ),* $(,)?) => {
        $(
            #[macro_export]
            macro_rules! $macro_name {
                ($val:expr) => {
                    $crate::StaticStyleProp::ptr(
                        $crate::static_style::$prop_id,
                        $val as *const core::ffi::c_void,
                    )
                };
            }
        )*
    };
    ($( $macro_name:ident => ($prop_id:ident, color) ),* $(,)?) => {
        $(
            #[macro_export]
            macro_rules! $macro_name {
                ($val:expr) => {
                    $crate::StaticStyleProp::color(
                        $crate::static_style::$prop_id,
                        $val,
                    )
                };
            }
        )*
    };
}

define_lv_style_const_macros!(
    LV_STYLE_CONST_WIDTH => (LV_STYLE_WIDTH, num),
    LV_STYLE_CONST_MIN_WIDTH => (LV_STYLE_MIN_WIDTH, num),
    LV_STYLE_CONST_MAX_WIDTH => (LV_STYLE_MAX_WIDTH, num),
    LV_STYLE_CONST_HEIGHT => (LV_STYLE_HEIGHT, num),
    LV_STYLE_CONST_MIN_HEIGHT => (LV_STYLE_MIN_HEIGHT, num),
    LV_STYLE_CONST_MAX_HEIGHT => (LV_STYLE_MAX_HEIGHT, num),
    LV_STYLE_CONST_LENGTH => (LV_STYLE_LENGTH, num),
    LV_STYLE_CONST_X => (LV_STYLE_X, num),
    LV_STYLE_CONST_Y => (LV_STYLE_Y, num),
    LV_STYLE_CONST_ALIGN => (LV_STYLE_ALIGN, num),
    LV_STYLE_CONST_TRANSFORM_WIDTH => (LV_STYLE_TRANSFORM_WIDTH, num),
    LV_STYLE_CONST_TRANSFORM_HEIGHT => (LV_STYLE_TRANSFORM_HEIGHT, num),
    LV_STYLE_CONST_TRANSLATE_X => (LV_STYLE_TRANSLATE_X, num),
    LV_STYLE_CONST_TRANSLATE_Y => (LV_STYLE_TRANSLATE_Y, num),
    LV_STYLE_CONST_TRANSLATE_RADIAL => (LV_STYLE_TRANSLATE_RADIAL, num),
    LV_STYLE_CONST_TRANSFORM_SCALE_X => (LV_STYLE_TRANSFORM_SCALE_X, num),
    LV_STYLE_CONST_TRANSFORM_SCALE_Y => (LV_STYLE_TRANSFORM_SCALE_Y, num),
    LV_STYLE_CONST_TRANSFORM_ROTATION => (LV_STYLE_TRANSFORM_ROTATION, num),
    LV_STYLE_CONST_TRANSFORM_PIVOT_X => (LV_STYLE_TRANSFORM_PIVOT_X, num),
    LV_STYLE_CONST_TRANSFORM_PIVOT_Y => (LV_STYLE_TRANSFORM_PIVOT_Y, num),
    LV_STYLE_CONST_TRANSFORM_SKEW_X => (LV_STYLE_TRANSFORM_SKEW_X, num),
    LV_STYLE_CONST_TRANSFORM_SKEW_Y => (LV_STYLE_TRANSFORM_SKEW_Y, num),
    LV_STYLE_CONST_PAD_TOP => (LV_STYLE_PAD_TOP, num),
    LV_STYLE_CONST_PAD_BOTTOM => (LV_STYLE_PAD_BOTTOM, num),
    LV_STYLE_CONST_PAD_LEFT => (LV_STYLE_PAD_LEFT, num),
    LV_STYLE_CONST_PAD_RIGHT => (LV_STYLE_PAD_RIGHT, num),
    LV_STYLE_CONST_PAD_ROW => (LV_STYLE_PAD_ROW, num),
    LV_STYLE_CONST_PAD_COLUMN => (LV_STYLE_PAD_COLUMN, num),
    LV_STYLE_CONST_PAD_RADIAL => (LV_STYLE_PAD_RADIAL, num),
    LV_STYLE_CONST_MARGIN_TOP => (LV_STYLE_MARGIN_TOP, num),
    LV_STYLE_CONST_MARGIN_BOTTOM => (LV_STYLE_MARGIN_BOTTOM, num),
    LV_STYLE_CONST_MARGIN_LEFT => (LV_STYLE_MARGIN_LEFT, num),
    LV_STYLE_CONST_MARGIN_RIGHT => (LV_STYLE_MARGIN_RIGHT, num),
    LV_STYLE_CONST_BG_OPA => (LV_STYLE_BG_OPA, num),
    LV_STYLE_CONST_BG_GRAD_DIR => (LV_STYLE_BG_GRAD_DIR, num),
    LV_STYLE_CONST_BG_MAIN_STOP => (LV_STYLE_BG_MAIN_STOP, num),
    LV_STYLE_CONST_BG_GRAD_STOP => (LV_STYLE_BG_GRAD_STOP, num),
    LV_STYLE_CONST_BG_MAIN_OPA => (LV_STYLE_BG_MAIN_OPA, num),
    LV_STYLE_CONST_BG_GRAD_OPA => (LV_STYLE_BG_GRAD_OPA, num),
    LV_STYLE_CONST_BG_IMAGE_OPA => (LV_STYLE_BG_IMAGE_OPA, num),
    LV_STYLE_CONST_BG_IMAGE_RECOLOR_OPA => (LV_STYLE_BG_IMAGE_RECOLOR_OPA, num),
    LV_STYLE_CONST_BG_IMAGE_TILED => (LV_STYLE_BG_IMAGE_TILED, num),
    LV_STYLE_CONST_BORDER_OPA => (LV_STYLE_BORDER_OPA, num),
    LV_STYLE_CONST_BORDER_WIDTH => (LV_STYLE_BORDER_WIDTH, num),
    LV_STYLE_CONST_BORDER_SIDE => (LV_STYLE_BORDER_SIDE, num),
    LV_STYLE_CONST_BORDER_POST => (LV_STYLE_BORDER_POST, num),
    LV_STYLE_CONST_OUTLINE_WIDTH => (LV_STYLE_OUTLINE_WIDTH, num),
    LV_STYLE_CONST_OUTLINE_OPA => (LV_STYLE_OUTLINE_OPA, num),
    LV_STYLE_CONST_OUTLINE_PAD => (LV_STYLE_OUTLINE_PAD, num),
    LV_STYLE_CONST_SHADOW_WIDTH => (LV_STYLE_SHADOW_WIDTH, num),
    LV_STYLE_CONST_SHADOW_OFFSET_X => (LV_STYLE_SHADOW_OFFSET_X, num),
    LV_STYLE_CONST_SHADOW_OFFSET_Y => (LV_STYLE_SHADOW_OFFSET_Y, num),
    LV_STYLE_CONST_SHADOW_SPREAD => (LV_STYLE_SHADOW_SPREAD, num),
    LV_STYLE_CONST_SHADOW_OPA => (LV_STYLE_SHADOW_OPA, num),
    LV_STYLE_CONST_IMAGE_OPA => (LV_STYLE_IMAGE_OPA, num),
    LV_STYLE_CONST_IMAGE_RECOLOR_OPA => (LV_STYLE_IMAGE_RECOLOR_OPA, num),
    LV_STYLE_CONST_LINE_WIDTH => (LV_STYLE_LINE_WIDTH, num),
    LV_STYLE_CONST_LINE_DASH_WIDTH => (LV_STYLE_LINE_DASH_WIDTH, num),
    LV_STYLE_CONST_LINE_DASH_GAP => (LV_STYLE_LINE_DASH_GAP, num),
    LV_STYLE_CONST_LINE_ROUNDED => (LV_STYLE_LINE_ROUNDED, num),
    LV_STYLE_CONST_LINE_OPA => (LV_STYLE_LINE_OPA, num),
    LV_STYLE_CONST_ARC_WIDTH => (LV_STYLE_ARC_WIDTH, num),
    LV_STYLE_CONST_ARC_ROUNDED => (LV_STYLE_ARC_ROUNDED, num),
    LV_STYLE_CONST_ARC_OPA => (LV_STYLE_ARC_OPA, num),
    LV_STYLE_CONST_TEXT_OPA => (LV_STYLE_TEXT_OPA, num),
    LV_STYLE_CONST_TEXT_LETTER_SPACE => (LV_STYLE_TEXT_LETTER_SPACE, num),
    LV_STYLE_CONST_TEXT_LINE_SPACE => (LV_STYLE_TEXT_LINE_SPACE, num),
    LV_STYLE_CONST_TEXT_DECOR => (LV_STYLE_TEXT_DECOR, num),
    LV_STYLE_CONST_TEXT_ALIGN => (LV_STYLE_TEXT_ALIGN, num),
    LV_STYLE_CONST_TEXT_OUTLINE_STROKE_WIDTH => (LV_STYLE_TEXT_OUTLINE_STROKE_WIDTH, num),
    LV_STYLE_CONST_TEXT_OUTLINE_STROKE_OPA => (LV_STYLE_TEXT_OUTLINE_STROKE_OPA, num),
    LV_STYLE_CONST_BLUR_RADIUS => (LV_STYLE_BLUR_RADIUS, num),
    LV_STYLE_CONST_BLUR_BACKDROP => (LV_STYLE_BLUR_BACKDROP, num),
    LV_STYLE_CONST_BLUR_QUALITY => (LV_STYLE_BLUR_QUALITY, num),
    LV_STYLE_CONST_DROP_SHADOW_RADIUS => (LV_STYLE_DROP_SHADOW_RADIUS, num),
    LV_STYLE_CONST_DROP_SHADOW_OFFSET_X => (LV_STYLE_DROP_SHADOW_OFFSET_X, num),
    LV_STYLE_CONST_DROP_SHADOW_OFFSET_Y => (LV_STYLE_DROP_SHADOW_OFFSET_Y, num),
    LV_STYLE_CONST_DROP_SHADOW_OPA => (LV_STYLE_DROP_SHADOW_OPA, num),
    LV_STYLE_CONST_DROP_SHADOW_QUALITY => (LV_STYLE_DROP_SHADOW_QUALITY, num),
    LV_STYLE_CONST_RADIUS => (LV_STYLE_RADIUS, num),
    LV_STYLE_CONST_RADIAL_OFFSET => (LV_STYLE_RADIAL_OFFSET, num),
    LV_STYLE_CONST_CLIP_CORNER => (LV_STYLE_CLIP_CORNER, num),
    LV_STYLE_CONST_OPA => (LV_STYLE_OPA, num),
    LV_STYLE_CONST_OPA_LAYERED => (LV_STYLE_OPA_LAYERED, num),
    LV_STYLE_CONST_COLOR_FILTER_OPA => (LV_STYLE_COLOR_FILTER_OPA, num),
    LV_STYLE_CONST_RECOLOR_OPA => (LV_STYLE_RECOLOR_OPA, num),
    LV_STYLE_CONST_ANIM_DURATION => (LV_STYLE_ANIM_DURATION, num),
    LV_STYLE_CONST_BLEND_MODE => (LV_STYLE_BLEND_MODE, num),
    LV_STYLE_CONST_LAYOUT => (LV_STYLE_LAYOUT, num),
    LV_STYLE_CONST_BASE_DIR => (LV_STYLE_BASE_DIR, num),
    LV_STYLE_CONST_ROTARY_SENSITIVITY => (LV_STYLE_ROTARY_SENSITIVITY, num),
    LV_STYLE_CONST_FLEX_FLOW => (LV_STYLE_FLEX_FLOW, num),
    LV_STYLE_CONST_FLEX_MAIN_PLACE => (LV_STYLE_FLEX_MAIN_PLACE, num),
    LV_STYLE_CONST_FLEX_CROSS_PLACE => (LV_STYLE_FLEX_CROSS_PLACE, num),
    LV_STYLE_CONST_FLEX_TRACK_PLACE => (LV_STYLE_FLEX_TRACK_PLACE, num),
    LV_STYLE_CONST_FLEX_GROW => (LV_STYLE_FLEX_GROW, num),
    LV_STYLE_CONST_GRID_COLUMN_ALIGN => (LV_STYLE_GRID_COLUMN_ALIGN, num),
    LV_STYLE_CONST_GRID_ROW_ALIGN => (LV_STYLE_GRID_ROW_ALIGN, num),
    LV_STYLE_CONST_GRID_CELL_COLUMN_POS => (LV_STYLE_GRID_CELL_COLUMN_POS, num),
    LV_STYLE_CONST_GRID_CELL_X_ALIGN => (LV_STYLE_GRID_CELL_X_ALIGN, num),
    LV_STYLE_CONST_GRID_CELL_COLUMN_SPAN => (LV_STYLE_GRID_CELL_COLUMN_SPAN, num),
    LV_STYLE_CONST_GRID_CELL_ROW_POS => (LV_STYLE_GRID_CELL_ROW_POS, num),
    LV_STYLE_CONST_GRID_CELL_Y_ALIGN => (LV_STYLE_GRID_CELL_Y_ALIGN, num),
    LV_STYLE_CONST_GRID_CELL_ROW_SPAN => (LV_STYLE_GRID_CELL_ROW_SPAN, num),
);

define_lv_style_const_macros!(
    LV_STYLE_CONST_BG_GRAD => (LV_STYLE_BG_GRAD, ptr),
    LV_STYLE_CONST_BG_IMAGE_SRC => (LV_STYLE_BG_IMAGE_SRC, ptr),
    LV_STYLE_CONST_IMAGE_COLORKEY => (LV_STYLE_IMAGE_COLORKEY, ptr),
    LV_STYLE_CONST_ARC_IMAGE_SRC => (LV_STYLE_ARC_IMAGE_SRC, ptr),
    LV_STYLE_CONST_TEXT_FONT => (LV_STYLE_TEXT_FONT, ptr),
    LV_STYLE_CONST_COLOR_FILTER_DSC => (LV_STYLE_COLOR_FILTER_DSC, ptr),
    LV_STYLE_CONST_ANIM => (LV_STYLE_ANIM, ptr),
    LV_STYLE_CONST_TRANSITION => (LV_STYLE_TRANSITION, ptr),
    LV_STYLE_CONST_BITMAP_MASK_SRC => (LV_STYLE_BITMAP_MASK_SRC, ptr),
    LV_STYLE_CONST_GRID_COLUMN_DSC_ARRAY => (LV_STYLE_GRID_COLUMN_DSC_ARRAY, ptr),
    LV_STYLE_CONST_GRID_ROW_DSC_ARRAY => (LV_STYLE_GRID_ROW_DSC_ARRAY, ptr),
);

define_lv_style_const_macros!(
    LV_STYLE_CONST_BG_COLOR => (LV_STYLE_BG_COLOR, color),
    LV_STYLE_CONST_BG_GRAD_COLOR => (LV_STYLE_BG_GRAD_COLOR, color),
    LV_STYLE_CONST_BG_IMAGE_RECOLOR => (LV_STYLE_BG_IMAGE_RECOLOR, color),
    LV_STYLE_CONST_BORDER_COLOR => (LV_STYLE_BORDER_COLOR, color),
    LV_STYLE_CONST_OUTLINE_COLOR => (LV_STYLE_OUTLINE_COLOR, color),
    LV_STYLE_CONST_SHADOW_COLOR => (LV_STYLE_SHADOW_COLOR, color),
    LV_STYLE_CONST_IMAGE_RECOLOR => (LV_STYLE_IMAGE_RECOLOR, color),
    LV_STYLE_CONST_LINE_COLOR => (LV_STYLE_LINE_COLOR, color),
    LV_STYLE_CONST_ARC_COLOR => (LV_STYLE_ARC_COLOR, color),
    LV_STYLE_CONST_TEXT_COLOR => (LV_STYLE_TEXT_COLOR, color),
    LV_STYLE_CONST_TEXT_OUTLINE_STROKE_COLOR => (LV_STYLE_TEXT_OUTLINE_STROKE_COLOR, color),
    LV_STYLE_CONST_DROP_SHADOW_COLOR => (LV_STYLE_DROP_SHADOW_COLOR, color),
    LV_STYLE_CONST_RECOLOR => (LV_STYLE_RECOLOR, color),
);

#[cfg(test)]
mod tests {
    use super::*;

    const BTN_STYLE_PROPS: [StaticStyleProp; 6] = [
        LV_STYLE_CONST_WIDTH!(120),
        LV_STYLE_CONST_HEIGHT!(50),
        LV_STYLE_CONST_BG_COLOR!(LV_COLOR_RGB!(0x11, 0x55, 0x88)),
        LV_STYLE_CONST_BG_OPA!(255),
        LV_STYLE_CONST_RADIUS!(8),
        StaticStyleProp::end(),
    ];

    const BTN_STYLE: StaticStyle = StaticStyle::init(&BTN_STYLE_PROPS);

    #[test]
    fn static_style_marker_values_match_lvgl_const_style_contract() {
        assert_eq!(BTN_STYLE.has_group, LV_STYLE_CONST_HAS_GROUP);
        assert_eq!(BTN_STYLE.prop_cnt, LV_STYLE_PROP_CONST);
    }
}

// -----------------------------------------------------------------------------
// Rust-idiomatic macro aliases
// -----------------------------------------------------------------------------
#[macro_export]
macro_rules! static_style {
    ($name:ident, $($prop:expr),+ $(,)?) => {
        pub const $name: $crate::StaticStyle = {
            const PROPS: &[$crate::StaticStyleProp] = &[
                $($prop,)+
                $crate::StaticStyleProp::end(),
            ];
            $crate::StaticStyle::init(PROPS)
        };
    };
}

#[macro_export]
macro_rules! static_color_rgb {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::LV_COLOR_RGB!($r, $g, $b)
    };
}

#[macro_export]
macro_rules! static_color_hex {
    ($hex:expr) => {
        $crate::LV_COLOR_HEX!($hex)
    };
}

#[macro_export]
macro_rules! static_style_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_min_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MIN_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_max_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MAX_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_height {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_HEIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_min_height {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MIN_HEIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_max_height {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MAX_HEIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_length {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LENGTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_height {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_HEIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_translate_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSLATE_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_translate_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSLATE_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_translate_radial {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSLATE_RADIAL!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_scale_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_SCALE_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_scale_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_SCALE_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_rotation {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_ROTATION!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_pivot_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_PIVOT_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_pivot_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_PIVOT_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_skew_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_SKEW_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_transform_skew_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSFORM_SKEW_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_top {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_TOP!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_bottom {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_BOTTOM!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_left {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_LEFT!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_right {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_RIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_row {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_ROW!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_column {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_COLUMN!($val)
    };
}

#[macro_export]
macro_rules! static_style_pad_radial {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_PAD_RADIAL!($val)
    };
}

#[macro_export]
macro_rules! static_style_margin_top {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MARGIN_TOP!($val)
    };
}

#[macro_export]
macro_rules! static_style_margin_bottom {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MARGIN_BOTTOM!($val)
    };
}

#[macro_export]
macro_rules! static_style_margin_left {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MARGIN_LEFT!($val)
    };
}

#[macro_export]
macro_rules! static_style_margin_right {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_MARGIN_RIGHT!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_grad_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_GRAD_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_grad_dir {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_GRAD_DIR!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_main_stop {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_MAIN_STOP!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_grad_stop {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_GRAD_STOP!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_main_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_MAIN_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_grad_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_GRAD_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_grad {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_GRAD!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_image_src {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_IMAGE_SRC!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_image_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_IMAGE_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_image_recolor {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_IMAGE_RECOLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_image_recolor_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_IMAGE_RECOLOR_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_bg_image_tiled {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BG_IMAGE_TILED!($val)
    };
}

#[macro_export]
macro_rules! static_style_border_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BORDER_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_border_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BORDER_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_border_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BORDER_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_border_side {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BORDER_SIDE!($val)
    };
}

#[macro_export]
macro_rules! static_style_border_post {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BORDER_POST!($val)
    };
}

#[macro_export]
macro_rules! static_style_outline_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OUTLINE_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_outline_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OUTLINE_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_outline_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OUTLINE_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_outline_pad {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OUTLINE_PAD!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_offset_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_OFFSET_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_offset_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_OFFSET_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_spread {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_SPREAD!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_shadow_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_SHADOW_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_image_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_IMAGE_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_image_recolor {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_IMAGE_RECOLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_image_recolor_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_IMAGE_RECOLOR_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_image_colorkey {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_IMAGE_COLORKEY!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_dash_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_DASH_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_dash_gap {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_DASH_GAP!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_rounded {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_ROUNDED!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_line_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LINE_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_arc_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ARC_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_arc_rounded {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ARC_ROUNDED!($val)
    };
}

#[macro_export]
macro_rules! static_style_arc_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ARC_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_arc_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ARC_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_arc_image_src {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ARC_IMAGE_SRC!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_font {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_FONT!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_letter_space {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_LETTER_SPACE!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_line_space {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_LINE_SPACE!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_decor {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_DECOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_outline_stroke_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_OUTLINE_STROKE_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_outline_stroke_width {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_OUTLINE_STROKE_WIDTH!($val)
    };
}

#[macro_export]
macro_rules! static_style_text_outline_stroke_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TEXT_OUTLINE_STROKE_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_blur_radius {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BLUR_RADIUS!($val)
    };
}

#[macro_export]
macro_rules! static_style_blur_backdrop {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BLUR_BACKDROP!($val)
    };
}

#[macro_export]
macro_rules! static_style_blur_quality {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BLUR_QUALITY!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_radius {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_RADIUS!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_offset_x {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_OFFSET_X!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_offset_y {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_OFFSET_Y!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_color {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_COLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_drop_shadow_quality {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_DROP_SHADOW_QUALITY!($val)
    };
}

#[macro_export]
macro_rules! static_style_radius {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_RADIUS!($val)
    };
}

#[macro_export]
macro_rules! static_style_radial_offset {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_RADIAL_OFFSET!($val)
    };
}

#[macro_export]
macro_rules! static_style_clip_corner {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_CLIP_CORNER!($val)
    };
}

#[macro_export]
macro_rules! static_style_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_opa_layered {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_OPA_LAYERED!($val)
    };
}

#[macro_export]
macro_rules! static_style_color_filter_dsc {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_COLOR_FILTER_DSC!($val)
    };
}

#[macro_export]
macro_rules! static_style_color_filter_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_COLOR_FILTER_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_recolor {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_RECOLOR!($val)
    };
}

#[macro_export]
macro_rules! static_style_recolor_opa {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_RECOLOR_OPA!($val)
    };
}

#[macro_export]
macro_rules! static_style_anim {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ANIM!($val)
    };
}

#[macro_export]
macro_rules! static_style_anim_duration {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ANIM_DURATION!($val)
    };
}

#[macro_export]
macro_rules! static_style_transition {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_TRANSITION!($val)
    };
}

#[macro_export]
macro_rules! static_style_blend_mode {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BLEND_MODE!($val)
    };
}

#[macro_export]
macro_rules! static_style_layout {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_LAYOUT!($val)
    };
}

#[macro_export]
macro_rules! static_style_base_dir {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BASE_DIR!($val)
    };
}

#[macro_export]
macro_rules! static_style_bitmap_mask_src {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_BITMAP_MASK_SRC!($val)
    };
}

#[macro_export]
macro_rules! static_style_rotary_sensitivity {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_ROTARY_SENSITIVITY!($val)
    };
}

#[macro_export]
macro_rules! static_style_flex_flow {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_FLEX_FLOW!($val)
    };
}

#[macro_export]
macro_rules! static_style_flex_main_place {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_FLEX_MAIN_PLACE!($val)
    };
}

#[macro_export]
macro_rules! static_style_flex_cross_place {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_FLEX_CROSS_PLACE!($val)
    };
}

#[macro_export]
macro_rules! static_style_flex_track_place {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_FLEX_TRACK_PLACE!($val)
    };
}

#[macro_export]
macro_rules! static_style_flex_grow {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_FLEX_GROW!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_column_dsc_array {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_COLUMN_DSC_ARRAY!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_column_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_COLUMN_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_row_dsc_array {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_ROW_DSC_ARRAY!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_row_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_ROW_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_column_pos {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_COLUMN_POS!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_x_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_X_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_column_span {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_COLUMN_SPAN!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_row_pos {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_ROW_POS!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_y_align {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_Y_ALIGN!($val)
    };
}

#[macro_export]
macro_rules! static_style_grid_cell_row_span {
    ($val:expr) => {
        $crate::LV_STYLE_CONST_GRID_CELL_ROW_SPAN!($val)
    };
}

// -----------------------------------------------------------------------------
// Rust-idiomatic short aliases
// -----------------------------------------------------------------------------
#[macro_export]
macro_rules! style {
    ($name:ident, $($prop:expr),+ $(,)?) => {
        pub const $name: $crate::StaticStyle = {
            const PROPS: &[$crate::StaticStyleProp] = &[
                $($prop,)+
                $crate::StaticStyleProp::end(),
            ];
            $crate::StaticStyle::init(PROPS)
        };
    };
}

#[macro_export]
macro_rules! color_rgb {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::static_color_rgb!($r, $g, $b)
    };
}

#[macro_export]
macro_rules! color_hex {
    ($hex:expr) => {
        $crate::static_color_hex!($hex)
    };
}

#[macro_export]
macro_rules! width {
    ($val:expr) => {
        $crate::static_style_width!($val)
    };
}

#[macro_export]
macro_rules! min_width {
    ($val:expr) => {
        $crate::static_style_min_width!($val)
    };
}

#[macro_export]
macro_rules! max_width {
    ($val:expr) => {
        $crate::static_style_max_width!($val)
    };
}

#[macro_export]
macro_rules! height {
    ($val:expr) => {
        $crate::static_style_height!($val)
    };
}

#[macro_export]
macro_rules! min_height {
    ($val:expr) => {
        $crate::static_style_min_height!($val)
    };
}

#[macro_export]
macro_rules! max_height {
    ($val:expr) => {
        $crate::static_style_max_height!($val)
    };
}

#[macro_export]
macro_rules! length {
    ($val:expr) => {
        $crate::static_style_length!($val)
    };
}

#[macro_export]
macro_rules! x {
    ($val:expr) => {
        $crate::static_style_x!($val)
    };
}

#[macro_export]
macro_rules! y {
    ($val:expr) => {
        $crate::static_style_y!($val)
    };
}

#[macro_export]
macro_rules! align {
    ($val:expr) => {
        $crate::static_style_align!($val)
    };
}

#[macro_export]
macro_rules! transform_width {
    ($val:expr) => {
        $crate::static_style_transform_width!($val)
    };
}

#[macro_export]
macro_rules! transform_height {
    ($val:expr) => {
        $crate::static_style_transform_height!($val)
    };
}

#[macro_export]
macro_rules! translate_x {
    ($val:expr) => {
        $crate::static_style_translate_x!($val)
    };
}

#[macro_export]
macro_rules! translate_y {
    ($val:expr) => {
        $crate::static_style_translate_y!($val)
    };
}

#[macro_export]
macro_rules! translate_radial {
    ($val:expr) => {
        $crate::static_style_translate_radial!($val)
    };
}

#[macro_export]
macro_rules! transform_scale_x {
    ($val:expr) => {
        $crate::static_style_transform_scale_x!($val)
    };
}

#[macro_export]
macro_rules! transform_scale_y {
    ($val:expr) => {
        $crate::static_style_transform_scale_y!($val)
    };
}

#[macro_export]
macro_rules! transform_rotation {
    ($val:expr) => {
        $crate::static_style_transform_rotation!($val)
    };
}

#[macro_export]
macro_rules! transform_pivot_x {
    ($val:expr) => {
        $crate::static_style_transform_pivot_x!($val)
    };
}

#[macro_export]
macro_rules! transform_pivot_y {
    ($val:expr) => {
        $crate::static_style_transform_pivot_y!($val)
    };
}

#[macro_export]
macro_rules! transform_skew_x {
    ($val:expr) => {
        $crate::static_style_transform_skew_x!($val)
    };
}

#[macro_export]
macro_rules! transform_skew_y {
    ($val:expr) => {
        $crate::static_style_transform_skew_y!($val)
    };
}

#[macro_export]
macro_rules! pad_top {
    ($val:expr) => {
        $crate::static_style_pad_top!($val)
    };
}

#[macro_export]
macro_rules! pad_bottom {
    ($val:expr) => {
        $crate::static_style_pad_bottom!($val)
    };
}

#[macro_export]
macro_rules! pad_left {
    ($val:expr) => {
        $crate::static_style_pad_left!($val)
    };
}

#[macro_export]
macro_rules! pad_right {
    ($val:expr) => {
        $crate::static_style_pad_right!($val)
    };
}

#[macro_export]
macro_rules! pad_row {
    ($val:expr) => {
        $crate::static_style_pad_row!($val)
    };
}

#[macro_export]
macro_rules! pad_column {
    ($val:expr) => {
        $crate::static_style_pad_column!($val)
    };
}

#[macro_export]
macro_rules! pad_radial {
    ($val:expr) => {
        $crate::static_style_pad_radial!($val)
    };
}

#[macro_export]
macro_rules! margin_top {
    ($val:expr) => {
        $crate::static_style_margin_top!($val)
    };
}

#[macro_export]
macro_rules! margin_bottom {
    ($val:expr) => {
        $crate::static_style_margin_bottom!($val)
    };
}

#[macro_export]
macro_rules! margin_left {
    ($val:expr) => {
        $crate::static_style_margin_left!($val)
    };
}

#[macro_export]
macro_rules! margin_right {
    ($val:expr) => {
        $crate::static_style_margin_right!($val)
    };
}

#[macro_export]
macro_rules! bg_color {
    ($val:expr) => {
        $crate::static_style_bg_color!($val)
    };
}

#[macro_export]
macro_rules! bg_opa {
    ($val:expr) => {
        $crate::static_style_bg_opa!($val)
    };
}

#[macro_export]
macro_rules! bg_grad_color {
    ($val:expr) => {
        $crate::static_style_bg_grad_color!($val)
    };
}

#[macro_export]
macro_rules! bg_grad_dir {
    ($val:expr) => {
        $crate::static_style_bg_grad_dir!($val)
    };
}

#[macro_export]
macro_rules! bg_main_stop {
    ($val:expr) => {
        $crate::static_style_bg_main_stop!($val)
    };
}

#[macro_export]
macro_rules! bg_grad_stop {
    ($val:expr) => {
        $crate::static_style_bg_grad_stop!($val)
    };
}

#[macro_export]
macro_rules! bg_main_opa {
    ($val:expr) => {
        $crate::static_style_bg_main_opa!($val)
    };
}

#[macro_export]
macro_rules! bg_grad_opa {
    ($val:expr) => {
        $crate::static_style_bg_grad_opa!($val)
    };
}

#[macro_export]
macro_rules! bg_grad {
    ($val:expr) => {
        $crate::static_style_bg_grad!($val)
    };
}

#[macro_export]
macro_rules! bg_image_src {
    ($val:expr) => {
        $crate::static_style_bg_image_src!($val)
    };
}

#[macro_export]
macro_rules! bg_image_opa {
    ($val:expr) => {
        $crate::static_style_bg_image_opa!($val)
    };
}

#[macro_export]
macro_rules! bg_image_recolor {
    ($val:expr) => {
        $crate::static_style_bg_image_recolor!($val)
    };
}

#[macro_export]
macro_rules! bg_image_recolor_opa {
    ($val:expr) => {
        $crate::static_style_bg_image_recolor_opa!($val)
    };
}

#[macro_export]
macro_rules! bg_image_tiled {
    ($val:expr) => {
        $crate::static_style_bg_image_tiled!($val)
    };
}

#[macro_export]
macro_rules! border_color {
    ($val:expr) => {
        $crate::static_style_border_color!($val)
    };
}

#[macro_export]
macro_rules! border_opa {
    ($val:expr) => {
        $crate::static_style_border_opa!($val)
    };
}

#[macro_export]
macro_rules! border_width {
    ($val:expr) => {
        $crate::static_style_border_width!($val)
    };
}

#[macro_export]
macro_rules! border_side {
    ($val:expr) => {
        $crate::static_style_border_side!($val)
    };
}

#[macro_export]
macro_rules! border_post {
    ($val:expr) => {
        $crate::static_style_border_post!($val)
    };
}

#[macro_export]
macro_rules! outline_width {
    ($val:expr) => {
        $crate::static_style_outline_width!($val)
    };
}

#[macro_export]
macro_rules! outline_color {
    ($val:expr) => {
        $crate::static_style_outline_color!($val)
    };
}

#[macro_export]
macro_rules! outline_opa {
    ($val:expr) => {
        $crate::static_style_outline_opa!($val)
    };
}

#[macro_export]
macro_rules! outline_pad {
    ($val:expr) => {
        $crate::static_style_outline_pad!($val)
    };
}

#[macro_export]
macro_rules! shadow_width {
    ($val:expr) => {
        $crate::static_style_shadow_width!($val)
    };
}

#[macro_export]
macro_rules! shadow_offset_x {
    ($val:expr) => {
        $crate::static_style_shadow_offset_x!($val)
    };
}

#[macro_export]
macro_rules! shadow_offset_y {
    ($val:expr) => {
        $crate::static_style_shadow_offset_y!($val)
    };
}

#[macro_export]
macro_rules! shadow_spread {
    ($val:expr) => {
        $crate::static_style_shadow_spread!($val)
    };
}

#[macro_export]
macro_rules! shadow_color {
    ($val:expr) => {
        $crate::static_style_shadow_color!($val)
    };
}

#[macro_export]
macro_rules! shadow_opa {
    ($val:expr) => {
        $crate::static_style_shadow_opa!($val)
    };
}

#[macro_export]
macro_rules! image_opa {
    ($val:expr) => {
        $crate::static_style_image_opa!($val)
    };
}

#[macro_export]
macro_rules! image_recolor {
    ($val:expr) => {
        $crate::static_style_image_recolor!($val)
    };
}

#[macro_export]
macro_rules! image_recolor_opa {
    ($val:expr) => {
        $crate::static_style_image_recolor_opa!($val)
    };
}

#[macro_export]
macro_rules! image_colorkey {
    ($val:expr) => {
        $crate::static_style_image_colorkey!($val)
    };
}

#[macro_export]
macro_rules! line_width {
    ($val:expr) => {
        $crate::static_style_line_width!($val)
    };
}

#[macro_export]
macro_rules! line_dash_width {
    ($val:expr) => {
        $crate::static_style_line_dash_width!($val)
    };
}

#[macro_export]
macro_rules! line_dash_gap {
    ($val:expr) => {
        $crate::static_style_line_dash_gap!($val)
    };
}

#[macro_export]
macro_rules! line_rounded {
    ($val:expr) => {
        $crate::static_style_line_rounded!($val)
    };
}

#[macro_export]
macro_rules! line_color {
    ($val:expr) => {
        $crate::static_style_line_color!($val)
    };
}

#[macro_export]
macro_rules! line_opa {
    ($val:expr) => {
        $crate::static_style_line_opa!($val)
    };
}

#[macro_export]
macro_rules! arc_width {
    ($val:expr) => {
        $crate::static_style_arc_width!($val)
    };
}

#[macro_export]
macro_rules! arc_rounded {
    ($val:expr) => {
        $crate::static_style_arc_rounded!($val)
    };
}

#[macro_export]
macro_rules! arc_color {
    ($val:expr) => {
        $crate::static_style_arc_color!($val)
    };
}

#[macro_export]
macro_rules! arc_opa {
    ($val:expr) => {
        $crate::static_style_arc_opa!($val)
    };
}

#[macro_export]
macro_rules! arc_image_src {
    ($val:expr) => {
        $crate::static_style_arc_image_src!($val)
    };
}

#[macro_export]
macro_rules! text_color {
    ($val:expr) => {
        $crate::static_style_text_color!($val)
    };
}

#[macro_export]
macro_rules! text_opa {
    ($val:expr) => {
        $crate::static_style_text_opa!($val)
    };
}

#[macro_export]
macro_rules! text_font {
    ($val:expr) => {
        $crate::static_style_text_font!($val)
    };
}

#[macro_export]
macro_rules! text_letter_space {
    ($val:expr) => {
        $crate::static_style_text_letter_space!($val)
    };
}

#[macro_export]
macro_rules! text_line_space {
    ($val:expr) => {
        $crate::static_style_text_line_space!($val)
    };
}

#[macro_export]
macro_rules! text_decor {
    ($val:expr) => {
        $crate::static_style_text_decor!($val)
    };
}

#[macro_export]
macro_rules! text_align {
    ($val:expr) => {
        $crate::static_style_text_align!($val)
    };
}

#[macro_export]
macro_rules! text_outline_stroke_color {
    ($val:expr) => {
        $crate::static_style_text_outline_stroke_color!($val)
    };
}

#[macro_export]
macro_rules! text_outline_stroke_width {
    ($val:expr) => {
        $crate::static_style_text_outline_stroke_width!($val)
    };
}

#[macro_export]
macro_rules! text_outline_stroke_opa {
    ($val:expr) => {
        $crate::static_style_text_outline_stroke_opa!($val)
    };
}

#[macro_export]
macro_rules! blur_radius {
    ($val:expr) => {
        $crate::static_style_blur_radius!($val)
    };
}

#[macro_export]
macro_rules! blur_backdrop {
    ($val:expr) => {
        $crate::static_style_blur_backdrop!($val)
    };
}

#[macro_export]
macro_rules! blur_quality {
    ($val:expr) => {
        $crate::static_style_blur_quality!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_radius {
    ($val:expr) => {
        $crate::static_style_drop_shadow_radius!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_offset_x {
    ($val:expr) => {
        $crate::static_style_drop_shadow_offset_x!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_offset_y {
    ($val:expr) => {
        $crate::static_style_drop_shadow_offset_y!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_color {
    ($val:expr) => {
        $crate::static_style_drop_shadow_color!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_opa {
    ($val:expr) => {
        $crate::static_style_drop_shadow_opa!($val)
    };
}

#[macro_export]
macro_rules! drop_shadow_quality {
    ($val:expr) => {
        $crate::static_style_drop_shadow_quality!($val)
    };
}

#[macro_export]
macro_rules! radius {
    ($val:expr) => {
        $crate::static_style_radius!($val)
    };
}

#[macro_export]
macro_rules! radial_offset {
    ($val:expr) => {
        $crate::static_style_radial_offset!($val)
    };
}

#[macro_export]
macro_rules! clip_corner {
    ($val:expr) => {
        $crate::static_style_clip_corner!($val)
    };
}

#[macro_export]
macro_rules! opa {
    ($val:expr) => {
        $crate::static_style_opa!($val)
    };
}

#[macro_export]
macro_rules! opa_layered {
    ($val:expr) => {
        $crate::static_style_opa_layered!($val)
    };
}

#[macro_export]
macro_rules! color_filter_dsc {
    ($val:expr) => {
        $crate::static_style_color_filter_dsc!($val)
    };
}

#[macro_export]
macro_rules! color_filter_opa {
    ($val:expr) => {
        $crate::static_style_color_filter_opa!($val)
    };
}

#[macro_export]
macro_rules! recolor {
    ($val:expr) => {
        $crate::static_style_recolor!($val)
    };
}

#[macro_export]
macro_rules! recolor_opa {
    ($val:expr) => {
        $crate::static_style_recolor_opa!($val)
    };
}

#[macro_export]
macro_rules! anim {
    ($val:expr) => {
        $crate::static_style_anim!($val)
    };
}

#[macro_export]
macro_rules! anim_duration {
    ($val:expr) => {
        $crate::static_style_anim_duration!($val)
    };
}

#[macro_export]
macro_rules! transition {
    ($val:expr) => {
        $crate::static_style_transition!($val)
    };
}

#[macro_export]
macro_rules! blend_mode {
    ($val:expr) => {
        $crate::static_style_blend_mode!($val)
    };
}

#[macro_export]
macro_rules! layout {
    ($val:expr) => {
        $crate::static_style_layout!($val)
    };
}

#[macro_export]
macro_rules! base_dir {
    ($val:expr) => {
        $crate::static_style_base_dir!($val)
    };
}

#[macro_export]
macro_rules! bitmap_mask_src {
    ($val:expr) => {
        $crate::static_style_bitmap_mask_src!($val)
    };
}

#[macro_export]
macro_rules! rotary_sensitivity {
    ($val:expr) => {
        $crate::static_style_rotary_sensitivity!($val)
    };
}

#[macro_export]
macro_rules! flex_flow {
    ($val:expr) => {
        $crate::static_style_flex_flow!($val)
    };
}

#[macro_export]
macro_rules! flex_main_place {
    ($val:expr) => {
        $crate::static_style_flex_main_place!($val)
    };
}

#[macro_export]
macro_rules! flex_cross_place {
    ($val:expr) => {
        $crate::static_style_flex_cross_place!($val)
    };
}

#[macro_export]
macro_rules! flex_track_place {
    ($val:expr) => {
        $crate::static_style_flex_track_place!($val)
    };
}

#[macro_export]
macro_rules! flex_grow {
    ($val:expr) => {
        $crate::static_style_flex_grow!($val)
    };
}

#[macro_export]
macro_rules! grid_column_dsc_array {
    ($val:expr) => {
        $crate::static_style_grid_column_dsc_array!($val)
    };
}

#[macro_export]
macro_rules! grid_column_align {
    ($val:expr) => {
        $crate::static_style_grid_column_align!($val)
    };
}

#[macro_export]
macro_rules! grid_row_dsc_array {
    ($val:expr) => {
        $crate::static_style_grid_row_dsc_array!($val)
    };
}

#[macro_export]
macro_rules! grid_row_align {
    ($val:expr) => {
        $crate::static_style_grid_row_align!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_column_pos {
    ($val:expr) => {
        $crate::static_style_grid_cell_column_pos!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_x_align {
    ($val:expr) => {
        $crate::static_style_grid_cell_x_align!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_column_span {
    ($val:expr) => {
        $crate::static_style_grid_cell_column_span!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_row_pos {
    ($val:expr) => {
        $crate::static_style_grid_cell_row_pos!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_y_align {
    ($val:expr) => {
        $crate::static_style_grid_cell_y_align!($val)
    };
}

#[macro_export]
macro_rules! grid_cell_row_span {
    ($val:expr) => {
        $crate::static_style_grid_cell_row_span!($val)
    };
}
