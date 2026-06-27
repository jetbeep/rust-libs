use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::c_bindings;

use super::border::BorderSide;
use super::event::{Event, LvEventCode};
use super::image::{Image, ImageSrc};
use super::label::Label;
use super::obj::Obj;
use super::size::Size;
use super::textarea::TextArea;
use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatPreset {
    Groups { prefix: String, groups: Vec<usize> },
    Mask { mask: String },
}

impl FormatPreset {
    pub fn groups(prefix: &str, groups: &[usize]) -> Self {
        if groups.is_empty() {
            panic!("FormatPreset::groups requires at least one group");
        }
        if groups.iter().any(|group| *group == 0) {
            panic!("FormatPreset::groups requires every group size to be greater than zero");
        }
        Self::Groups {
            prefix: prefix.to_string(),
            groups: groups.to_vec(),
        }
    }

    pub fn mask(mask: &str) -> Self {
        if !mask.chars().any(|ch| ch == 'X') {
            panic!("FormatPreset::mask requires at least one X digit slot");
        }
        Self::Mask {
            mask: mask.to_string(),
        }
    }

    pub fn capacity(&self) -> usize {
        match self {
            Self::Groups { groups, .. } => groups.iter().copied().sum(),
            Self::Mask { mask } => mask.chars().filter(|ch| *ch == 'X').count(),
        }
    }

    pub fn max_formatted_len(&self) -> usize {
        match self {
            Self::Groups { prefix, groups } => {
                let digits: usize = groups.iter().copied().sum();
                let separators = groups.len().saturating_sub(1);
                prefix.chars().count() + digits + separators
            }
            Self::Mask { mask } => mask.chars().count(),
        }
    }

    pub fn normalize_digits(&self, input: &str) -> String {
        let source = match self {
            Self::Groups { prefix, .. } => input.strip_prefix(prefix).unwrap_or(input),
            Self::Mask { .. } => input,
        };
        source
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .take(self.capacity())
            .collect()
    }

    pub fn format_digits(&self, digits: &str) -> String {
        let raw = self.normalize_digits(digits);
        if raw.is_empty() {
            return String::new();
        }
        match self {
            Self::Groups { prefix, groups } => format_groups(prefix, groups, &raw),
            Self::Mask { mask } => format_mask(mask, &raw),
        }
    }
}

fn format_groups(prefix: &str, groups: &[usize], raw: &str) -> String {
    let mut out = String::new();
    out.push_str(prefix);
    let mut digits = raw.chars();
    for (group_index, group_size) in groups.iter().copied().enumerate() {
        let mut wrote_digit = false;
        for _ in 0..group_size {
            let Some(digit) = digits.next() else {
                break;
            };
            if !wrote_digit && group_index > 0 {
                out.push(' ');
            }
            out.push(digit);
            wrote_digit = true;
        }
        if !wrote_digit {
            break;
        }
    }
    out
}

fn format_mask(mask: &str, raw: &str) -> String {
    let mut out = String::new();
    let mut digits = raw.chars().peekable();
    for ch in mask.chars() {
        if ch == 'X' {
            let Some(digit) = digits.next() else {
                break;
            };
            out.push(digit);
        } else {
            if digits.peek().is_some() {
                out.push(ch);
            } else {
                break;
            }
        }
    }
    out
}

struct FieldContext {
    preset: FormatPreset,
    raw_digits: RefCell<String>,
    text_area_ptr: Cell<usize>,
    suppress_event: Cell<bool>,
    slot_configured: Cell<bool>,
}

pub struct PhoneFormatterField {
    root: Obj,
    left_slot: Obj,
    text_area: TextArea,
    context: Box<FieldContext>,
}

impl Widget for PhoneFormatterField {
    fn lv_obj(&self) -> &LvObj {
        self.root.lv_obj()
    }
}

impl PhoneFormatterField {
    pub fn new(parent: &impl Widget, preset: FormatPreset) -> Self {
        let max_len = preset.max_formatted_len() as u32;
        let root = Obj::new(parent);
        root.flex_row().set_scrollable(false);

        let left_slot = Obj::new(&root);
        left_slot.set_hidden(true).set_scrollable(false);

        let text_area = TextArea::new(&root);
        text_area
            .one_line(true)
            .max_length(max_len)
            .set_text("")
            .set_scrollable(false)
            .border_width(0)
            .bg_opa(0);

        let context = Box::new(FieldContext {
            preset,
            raw_digits: RefCell::new(String::new()),
            text_area_ptr: Cell::new(text_area.raw_ptr()),
            suppress_event: Cell::new(false),
            slot_configured: Cell::new(false),
        });

        let field = Self {
            root,
            left_slot,
            text_area,
            context,
        };
        field.register_value_changed();
        field
    }

    pub fn left_slot(&self, slot: LeftSlot) -> &Self {
        if !slot.has_visible_content() {
            panic!("LeftSlot preset requires text, icon, arrow, or custom content");
        }
        self.mark_left_slot_configured();
        self.show_left_slot(slot.width);
        self.left_slot.pad_left(slot.pad_x).pad_right(slot.pad_x);
        if slot.divider {
            self.left_slot
                .border_width(1)
                .border_side(BorderSide::RIGHT);
        }
        if let Some(icon) = slot.icon {
            let _ = Image::new(&self.left_slot).set_src(icon);
        }
        if let Some(text) = slot.text {
            let _ = Label::new(&self.left_slot).text(&text);
        }
        if let Some(arrow) = slot.arrow {
            let _ = Label::new(&self.left_slot).text(&arrow);
        }
        if let Some(cb) = slot.on_click {
            self.left_slot.set_clickable(true).on_click(cb);
        }
        self
    }

    /// Returns a widget handle for adding custom children to the built-in left slot.
    ///
    /// The handle aliases the field-owned LVGL object. Do not use it after the
    /// `PhoneFormatterField` that created it has been dropped.
    pub fn custom_left_slot(&self, width: Size) -> LeftSlotHandle {
        self.mark_left_slot_configured();
        self.show_left_slot(Some(width));
        LeftSlotHandle {
            obj: LvObj::from_raw(self.left_slot.lv_obj().raw()),
        }
    }

    fn mark_left_slot_configured(&self) {
        assert!(
            !self.context.slot_configured.get(),
            "left slot already configured"
        );
        self.context.slot_configured.set(true);
    }

    fn show_left_slot(&self, width: Option<Size>) {
        self.left_slot
            .set_hidden(false)
            .set_scrollable(false)
            .flex_row();
        if let Some(w) = width {
            self.left_slot.width(w);
        }
    }

    pub fn placeholder_text(&self, text: &str) -> &Self {
        self.text_area.placeholder_text(text);
        self
    }

    pub fn set_raw_digits(&self, input: &str) -> &Self {
        let raw = self.context.preset.normalize_digits(input);
        self.context.raw_digits.replace(raw.clone());
        self.write_display_text(&self.context.preset.format_digits(&raw));
        self
    }

    pub fn raw_digits(&self) -> String {
        self.context.raw_digits.borrow().clone()
    }

    pub fn formatted_text(&self) -> String {
        self.context
            .preset
            .format_digits(&self.context.raw_digits.borrow())
    }

    fn write_display_text(&self, text: &str) {
        self.context.suppress_event.set(true);
        self.text_area.set_text(text);
        self.context.suppress_event.set(false);
    }

    fn register_value_changed(&self) {
        // SAFETY: `text_area` is alive while `self` is alive, and `context` is a boxed
        // allocation whose address stays stable until the callback is unregistered in Drop.
        unsafe {
            c_bindings::lv_obj_add_event_cb(
                self.text_area.lv_obj().raw(),
                Some(on_textarea_value_changed),
                LvEventCode::ValueChanged.as_u32(),
                self.context.as_ref() as *const FieldContext as *mut core::ffi::c_void,
            );
        }
    }

    #[cfg(test)]
    fn input_raw_ptr(&self) -> usize {
        self.text_area.raw_ptr()
    }
}

impl Drop for PhoneFormatterField {
    fn drop(&mut self) {
        // SAFETY: Unregisters the callback before `context` is freed, ensuring no use-after-free.
        unsafe {
            c_bindings::lv_obj_remove_event_cb_with_user_data(
                self.text_area.lv_obj().raw(),
                Some(on_textarea_value_changed),
                self.context.as_ref() as *const FieldContext as *mut core::ffi::c_void,
            );
        }
    }
}

unsafe extern "C" fn on_textarea_value_changed(e: *mut c_bindings::lv_event_t) {
    unsafe {
        let user_data = c_bindings::lv_event_get_user_data(e);
        if user_data.is_null() {
            return;
        }
        // SAFETY: `user_data` is a pointer to `FieldContext` registered in `register_value_changed`
        // and unregistered in Drop before the box is freed. LVGL is single-threaded, so the
        // callback cannot execute during Drop or after `PhoneFormatterField` is deallocated.
        let context = &*(user_data as *const FieldContext);
        if context.suppress_event.get() {
            return;
        }
        let text_area_ptr = context.text_area_ptr.get();
        if text_area_ptr == 0 {
            return;
        }
        let input = TextArea::text_from_raw_ptr(text_area_ptr);
        let raw = context.preset.normalize_digits(&input);
        let formatted = context.preset.format_digits(&raw);
        context.raw_digits.replace(raw);
        if input != formatted {
            let c_string = to_null_terminated(&formatted);
            context.suppress_event.set(true);
            c_bindings::lv_textarea_set_text(
                text_area_ptr as *mut c_bindings::lv_obj_t,
                c_string.as_ptr() as *const core::ffi::c_char,
            );
            context.suppress_event.set(false);
        }
    }
}

pub struct LeftSlot {
    text: Option<String>,
    arrow: Option<String>,
    icon: Option<ImageSrc>,
    width: Option<Size>,
    divider: bool,
    pad_x: i32,
    on_click: Option<fn(Event)>,
}

impl LeftSlot {
    pub fn preset() -> Self {
        Self {
            text: None,
            arrow: None,
            icon: None,
            width: None,
            divider: false,
            pad_x: 10,
            on_click: None,
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn arrow(mut self, arrow: &str) -> Self {
        self.arrow = Some(arrow.to_string());
        self
    }

    pub fn icon(mut self, icon: ImageSrc) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn divider(mut self, enabled: bool) -> Self {
        self.divider = enabled;
        self
    }

    pub fn padding(mut self, px: i32) -> Self {
        self.pad_x = px;
        self
    }

    pub fn on_click(mut self, cb: fn(Event)) -> Self {
        self.on_click = Some(cb);
        self
    }

    fn has_visible_content(&self) -> bool {
        self.text.is_some() || self.arrow.is_some() || self.icon.is_some()
    }
}

pub struct LeftSlotHandle {
    obj: LvObj,
}

impl Widget for LeftSlotHandle {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatPreset, LeftSlot, PhoneFormatterField};
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::screen::Screen;
    use crate::lvgl::{CornerRadius, Size, Widget};

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_creates_root_left_slot_and_text_area() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ObjCreate { .. })),
            "expected root/slot object creation, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::TextAreaCreate { .. })),
            "expected TextAreaCreate, got: {:?}",
            calls
        );
        assert_eq!(field.raw_digits(), "");
        assert_eq!(field.formatted_text(), "");
    }

    #[test]
    fn new_configures_one_line_text_area_with_formatted_capacity() {
        let screen = parent();
        let _field = PhoneFormatterField::new(&screen, FormatPreset::mask("WECHIP - X X X X X X"));

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::TextAreaSetOneLine { en: true, .. })),
            "expected one-line text area, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::TextAreaSetMaxLength { max: 20, .. })),
            "expected max formatted length for mask, got: {:?}",
            calls
        );
    }

    #[test]
    fn set_raw_digits_updates_raw_and_formatted_text_area() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        spy_drain();

        field.set_raw_digits("86 602 93 71");

        assert_eq!(field.raw_digits(), "866029371");
        assert_eq!(field.formatted_text(), "+41 86 602 93 71");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::TextAreaSetText { text, .. } if text == b"+41 86 602 93 71\0"
            )),
            "expected formatted TextAreaSetText, got: {:?}",
            calls
        );
    }

    #[test]
    fn widget_methods_apply_to_root_object() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        spy_drain();

        field
            .size(Size::Px(340), Size::Px(56))
            .radius(CornerRadius::Px(8))
            .border_width(2);

        let root = field.raw_ptr();
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ObjSetSize { obj, w: 340, h: 56 } if *obj == root
            )),
            "expected root ObjSetSize, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleRadius { obj, value: 8 } if *obj == root
            )),
            "expected root radius, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::SetStyleBorderWidth { obj, value: 2 } if *obj == root
            )),
            "expected root border width, got: {:?}",
            calls
        );
    }

    #[test]
    fn groups_preset_formats_full_phone_number() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.format_digits("866029371"), "+41 86 602 93 71");
    }

    #[test]
    fn groups_preset_formats_partial_phone_number() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.format_digits("8"), "+41 8");
        assert_eq!(preset.format_digits("8660"), "+41 86 60");
    }

    #[test]
    fn mask_preset_formats_simple_code() {
        let preset = FormatPreset::mask("WECHIP - X X X X X X");
        assert_eq!(preset.format_digits("234567"), "WECHIP - 2 3 4 5 6 7");
        assert_eq!(preset.format_digits("234"), "WECHIP - 2 3 4");
    }

    #[test]
    fn empty_raw_digits_render_empty_text() {
        let phone = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        let code = FormatPreset::mask("WECHIP - X X X X X X");
        assert_eq!(phone.format_digits(""), "");
        assert_eq!(code.format_digits(""), "");
    }

    #[test]
    fn normalize_digits_strips_non_digits_and_truncates_to_capacity() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.normalize_digits("+41 86-602-93-7100"), "866029371");
    }

    #[test]
    fn normalize_digits_keeps_plain_digits_when_prefix_is_absent() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.normalize_digits("86-602-93-71"), "866029371");
    }

    #[test]
    #[should_panic(expected = "FormatPreset::groups requires at least one group")]
    fn groups_preset_rejects_no_groups() {
        let _ = FormatPreset::groups("+41 ", &[]);
    }

    #[test]
    #[should_panic(
        expected = "FormatPreset::groups requires every group size to be greater than zero"
    )]
    fn groups_preset_rejects_zero_group_size() {
        let _ = FormatPreset::groups("+41 ", &[2, 0, 2]);
    }

    #[test]
    #[should_panic(expected = "FormatPreset::mask requires at least one X digit slot")]
    fn mask_preset_rejects_masks_without_digit_slots() {
        let _ = FormatPreset::mask("WECHIP - ");
    }

    #[test]
    fn groups_max_formatted_len_counts_prefix_chars_not_bytes() {
        let preset = FormatPreset::groups("☎ ", &[2, 2]);
        assert_eq!(preset.max_formatted_len(), 7);
    }

    #[test]
    fn mask_preset_truncates_excess_digits_to_capacity() {
        let preset = FormatPreset::mask("XX-XX");
        assert_eq!(preset.format_digits("12345678"), "12-34");
    }

    #[test]
    fn mask_max_formatted_len_counts_chars_not_bytes() {
        let preset = FormatPreset::mask("☎X");
        assert_eq!(preset.max_formatted_len(), 2);
    }

    #[test]
    fn value_changed_normalizes_user_input_and_rewrites_formatted_text() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        let input = field.input_raw_ptr() as *mut crate::c_bindings::lv_obj_t;
        spy_drain();

        unsafe {
            crate::c_bindings::lv_textarea_set_text(input, c"+41 86 602 93 7100".as_ptr());
            crate::c_bindings::spy_emit_event(input, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
        }

        assert_eq!(field.raw_digits(), "866029371");
        assert_eq!(field.formatted_text(), "+41 86 602 93 71");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::TextAreaSetText { text, .. } if text == b"+41 86 602 93 71\0"
            )),
            "expected rewritten formatted text, got: {:?}",
            calls
        );
    }

    #[test]
    fn value_changed_formats_mask_input_progressively() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::mask("WECHIP - X X X X X X"));
        let input = field.input_raw_ptr() as *mut crate::c_bindings::lv_obj_t;
        spy_drain();

        unsafe {
            crate::c_bindings::lv_textarea_set_text(input, c"abc234".as_ptr());
            crate::c_bindings::spy_emit_event(input, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
        }

        assert_eq!(field.raw_digits(), "234");
        assert_eq!(field.formatted_text(), "WECHIP - 2 3 4");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::TextAreaSetText { text, .. } if text == b"WECHIP - 2 3 4\0"
            )),
            "expected rewritten formatted text for mask, got: {:?}",
            calls
        );
    }

    #[test]
    fn new_registers_value_changed_callback_on_text_area() {
        let screen = parent();
        let _field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::AddEventCb { code, .. } if *code == crate::c_bindings::LV_EVENT_VALUE_CHANGED
        )), "expected value-changed event registration, got: {:?}", calls);
    }

    #[test]
    fn left_slot_preset_unhides_slot_and_creates_text_and_arrow_labels() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        spy_drain();

        field.left_slot(
            LeftSlot::preset()
                .text("CH")
                .arrow("v")
                .width(Size::Px(72))
                .divider(true),
        );

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::RemoveFlag { .. })),
            "expected hidden flag removal for left slot, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetWidth { w: 72, .. })),
            "expected left slot width, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"CH\0"
            )),
            "expected text label, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"v\0"
            )),
            "expected arrow label, got: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 1, .. })),
            "expected divider border width, got: {:?}",
            calls
        );
    }

    #[test]
    fn left_slot_preset_registers_click_callback() {
        fn on_left_slot(_: crate::lvgl::event::Event) {}

        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        spy_drain();

        field.left_slot(LeftSlot::preset().text("CH").on_click(on_left_slot));

        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::AddEventCb { code, .. } if *code == crate::c_bindings::LV_EVENT_CLICKED
            )),
            "expected click callback registration, got: {:?}",
            calls
        );
    }

    #[test]
    fn custom_left_slot_returns_widget_handle_for_caller_children() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::mask("WECHIP - X X X X X X"));
        spy_drain();

        let slot = field.custom_left_slot(Size::Px(96));
        let _label = crate::lvgl::Label::new(&slot).text("WECHIP");

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ObjSetWidth { w: 96, .. })),
            "expected custom slot width, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"WECHIP\0"
            )),
            "expected caller child label, got: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "LeftSlot preset requires text, icon, arrow, or custom content")]
    fn empty_left_slot_preset_panics() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        field.left_slot(LeftSlot::preset());
    }

    #[test]
    fn left_slot_divider_uses_right_border_side() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        spy_drain();

        field.left_slot(LeftSlot::preset().text("CH").divider(true));

        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 1, .. })),
            "expected divider border width, got: {:?}",
            calls
        );
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderSide { value, .. } if *value == crate::lvgl::BorderSide::RIGHT.0)), "expected right-only divider border side, got: {:?}", calls);
    }

    #[test]
    #[should_panic(expected = "left slot already configured")]
    fn left_slot_panics_when_configured_twice() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        field.left_slot(LeftSlot::preset().text("CH"));
        field.left_slot(LeftSlot::preset().text("DE"));
    }

    #[test]
    #[should_panic(expected = "left slot already configured")]
    fn custom_left_slot_then_preset_panics() {
        let screen = parent();
        let field = PhoneFormatterField::new(&screen, FormatPreset::groups("+41 ", &[2, 3, 2, 2]));
        let _slot = field.custom_left_slot(Size::Px(96));
        field.left_slot(LeftSlot::preset().text("CH"));
    }
}
