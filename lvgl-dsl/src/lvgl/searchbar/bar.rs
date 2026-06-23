//! SearchBar widget tree (§2). Owns the LVGL objects but NOT the FSM —
//! the SearchBar struct in `mod.rs` composes Bar + InnerState + Slots.
use super::slots::Slots;
use crate::c_bindings::{
    LV_FLEX_FLOW_COLUMN, LV_FLEX_FLOW_ROW, lv_button_create, lv_label_create, lv_label_set_text,
    lv_obj_create, lv_obj_set_flex_flow, lv_obj_set_flex_grow, lv_obj_set_height, lv_obj_set_size,
    lv_obj_set_width, lv_obj_t, lv_pct, lv_textarea_create,
};

/// Height of the input row inside the SearchBar's column flex (px).
/// Sized to fit a 32 px textarea + small vertical padding.
pub(crate) const INPUT_ROW_HEIGHT: i32 = 48;
/// Square clear-button edge length (px). Matches `INPUT_ROW_HEIGHT - 8`
/// so the button sits comfortably inside the input row.
const CLEAR_BUTTON_EDGE: i32 = 40;

pub struct Bar {
    pub root: *mut lv_obj_t,
    pub input_container: *mut lv_obj_t,
    pub text_area: *mut lv_obj_t,
    pub clear_button: *mut lv_obj_t,
    pub clear_label: *mut lv_obj_t,
    pub result_container: *mut lv_obj_t,
    pub slots: Slots,
}

impl Bar {
    /// # Safety
    /// `parent` must be a valid LVGL object pointer (or null for screen).
    pub unsafe fn build(parent: *mut lv_obj_t, width: i32, height: i32) -> Self {
        unsafe {
            let root = lv_obj_create(parent);
            lv_obj_set_size(root, width, height);
            lv_obj_set_flex_flow(root, LV_FLEX_FLOW_COLUMN);

            // Input row: full width, fixed height. Without an explicit size
            // `lv_obj_create` produces a tiny default-sized box that the
            // column-flow root cannot stretch (LVGL flex never expands a
            // child beyond its declared size).
            let input_container = lv_obj_create(root);
            lv_obj_set_width(input_container, lv_pct(100));
            lv_obj_set_height(input_container, INPUT_ROW_HEIGHT);
            lv_obj_set_flex_flow(input_container, LV_FLEX_FLOW_ROW);

            // Textarea fills the row horizontally (flex_grow=1); the clear
            // button takes a fixed square at the right.
            let text_area = lv_textarea_create(input_container);
            lv_obj_set_height(text_area, lv_pct(100));
            lv_obj_set_flex_grow(text_area, 1);

            let clear_button = lv_button_create(input_container);
            lv_obj_set_size(clear_button, CLEAR_BUTTON_EDGE, CLEAR_BUTTON_EDGE);
            // TODO(styling): hide clear_button when textarea is empty
            // (toggle via LV_OBJ_FLAG_HIDDEN on textarea VALUE_CHANGED).
            let clear_label = lv_label_create(clear_button);
            lv_label_set_text(clear_label, b"\xC3\x97\0".as_ptr() as _); // "×" U+00D7

            // Results: full width, fills remaining height after the input
            // row via flex_grow=1.
            let result_container = lv_obj_create(root);
            lv_obj_set_width(result_container, lv_pct(100));
            lv_obj_set_flex_grow(result_container, 1);
            lv_obj_set_flex_flow(result_container, LV_FLEX_FLOW_COLUMN);

            Bar {
                root,
                input_container,
                text_area,
                clear_button,
                clear_label,
                result_container,
                slots: Slots::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, SPY, SpyFixture};
    use core::ptr;

    #[test]
    fn build_creates_full_tree() {
        let _fx = SpyFixture::new();
        let b = unsafe { Bar::build(ptr::null_mut(), 320, 240) };
        assert!(!b.root.is_null());
        assert!(!b.input_container.is_null());
        assert!(!b.text_area.is_null());
        assert!(!b.result_container.is_null());

        // Verify tree structure through parent relationships in SPY log
        let creates: Vec<(usize, usize)> = SPY.with(|s| {
            s.borrow()
                .iter()
                .filter_map(|c| match c {
                    LvCall::ObjCreate { obj, parent } => Some((*obj, *parent)),
                    _ => None,
                })
                .collect()
        });

        assert!(
            creates.len() >= 3,
            "expected ≥3 ObjCreate calls, got {}",
            creates.len()
        );
        // First create: root with null parent
        assert_eq!(creates[0].1, 0, "root should have null parent");
        // Next creates should have root as parent
        assert_eq!(
            creates[1].1, creates[0].0,
            "input_container should have root as parent"
        );
        assert_eq!(
            creates[2].1, creates[0].0,
            "result_container should have root as parent"
        );

        let creates = SPY.with(|s| {
            s.borrow()
                .iter()
                .filter(|c| {
                    matches!(
                        c,
                        LvCall::ObjCreate { .. }
                            | LvCall::TextAreaCreate { .. }
                            | LvCall::ButtonCreate { .. }
                            | LvCall::LabelCreate { .. }
                    )
                })
                .count()
        });
        assert!(creates >= 6, "expected ≥6 creates, got {creates}");
    }

    #[test]
    fn build_sets_flex_flows() {
        let _fx = SpyFixture::new();
        let _ = unsafe { Bar::build(ptr::null_mut(), 320, 240) };
        let cols = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::ObjSetFlexFlow { flow, .. } if flow == &LV_FLEX_FLOW_COLUMN))
            .count());
        let rows = SPY.with(|s| s.borrow().iter()
            .filter(|c| matches!(c, LvCall::ObjSetFlexFlow { flow, .. } if flow == &LV_FLEX_FLOW_ROW))
            .count());
        assert_eq!(cols, 2);
        assert_eq!(rows, 1);
    }
}
