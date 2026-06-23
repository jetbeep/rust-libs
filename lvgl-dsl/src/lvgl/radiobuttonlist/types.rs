use super::super::{Color, CornerRadius, Font};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RadioButtonEvent<'a> {
    pub index: usize,
    pub label: &'a str,
}

#[derive(Copy, Clone)]
pub struct RadioButtonListStyle {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub border_opa: Option<u8>,
    pub radius: Option<CornerRadius>,
    pub text_color: Option<Color>,
    pub text_opa: Option<u8>,
    pub text_font: Option<Font>,
}

impl Default for RadioButtonListStyle {
    fn default() -> Self {
        Self {
            bg_color: None,
            bg_opa: None,
            border_color: None,
            border_width: None,
            border_opa: None,
            radius: None,
            text_color: None,
            text_opa: None,
            text_font: None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RadioIndicatorStyle {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub border_opa: Option<u8>,
    pub radius: Option<CornerRadius>,
    /// Fill color of the inner dot rendered inside the indicator ring. When
    /// combined with `dot_opa = Some(255)` this produces the classic
    /// "ring + filled center" radio look used by the Figma design.
    pub dot_color: Option<Color>,
    /// Opacity of the inner dot. Use `Some(0)` to hide (default for unselected
    /// rows) and `Some(255)` to show the dot for the selected row.
    pub dot_opa: Option<u8>,
}

impl Default for RadioIndicatorStyle {
    fn default() -> Self {
        Self {
            bg_color: None,
            bg_opa: Some(0),
            border_color: None,
            border_width: Some(1),
            border_opa: None,
            radius: Some(CornerRadius::Full),
            dot_color: None,
            dot_opa: Some(0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RadioButtonListConfig {
    pub row_height: i32,
    pub gap: i32,
    pub pad_h: i32,
    pub pad_v: i32,
    pub indicator_size: i32,
    pub indicator_label_gap: i32,
}

impl Default for RadioButtonListConfig {
    fn default() -> Self {
        Self {
            row_height: 44,
            gap: 8,
            pad_h: 12,
            pad_v: 10,
            indicator_size: 18,
            indicator_label_gap: 12,
        }
    }
}

pub(crate) fn assert_valid_options(labels: &[&str]) {
    assert!(!labels.is_empty(), "RadioButtonList requires at least one option");
}

pub(crate) fn assert_valid_config(cfg: RadioButtonListConfig) {
    assert!(cfg.row_height > 0, "RadioButtonList row height must be positive, got {}", cfg.row_height);
    assert!(cfg.indicator_size > 0, "RadioButtonList indicator size must be positive, got {}", cfg.indicator_size);
    assert!(cfg.gap >= 0, "RadioButtonList gap must be non-negative, got {}", cfg.gap);
    assert!(cfg.pad_h >= 0, "RadioButtonList horizontal padding must be non-negative, got {}", cfg.pad_h);
    assert!(cfg.pad_v >= 0, "RadioButtonList vertical padding must be non-negative, got {}", cfg.pad_v);
    assert!(cfg.indicator_label_gap >= 0, "RadioButtonList indicator-label gap must be non-negative, got {}", cfg.indicator_label_gap);
}
