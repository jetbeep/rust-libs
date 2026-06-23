use core::cell::RefCell;
use core::ffi::c_void;

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::c_bindings;

use super::color::Color;
use super::image::{ImageRetentionSlot, ImageSrc, set_retained_src_for_obj};
use super::state::LvObjFlag;
use super::widget::{LvObj, Widget};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellStatusId(pub u16);

impl CellStatusId {
    pub const DEFAULT: CellStatusId = CellStatusId(0);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl CellRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParcelLockerCell {
    pub row: usize,
    pub col: usize,
    pub rect: CellRect,
}

impl ParcelLockerCell {
    pub const fn new(row: usize, col: usize, rect: CellRect) -> Self {
        Self { row, col, rect }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellTap {
    pub index: usize,
    pub row: usize,
    pub col: usize,
    pub status: CellStatusId,
    pub disabled: bool,
}

/// Sparse style patch for a cell overlay. Only sets fields it is responsible for.
/// Fields left `None` will not override earlier layers when applied to `ResolvedCellStyle`.
/// Every restyle starts from `ResolvedCellStyle::blank()` (transparent background, no borders/outlines, total opa 255),
/// so `CellStyle::transparent()` results in transparent visuals when used as the only style.
#[derive(Copy, Clone)]
pub struct CellStyle {
    bg_color: Option<Color>,
    bg_opa: Option<u8>,
    border_color: Option<Color>,
    border_width: Option<i32>,
    border_opa: Option<u8>,
    outline_color: Option<Color>,
    outline_width: Option<i32>,
    outline_opa: Option<u8>,
    outline_pad: Option<i32>,
    opa: Option<u8>,
}

impl CellStyle {
    pub const fn transparent() -> Self {
        Self {
            bg_color: None,
            bg_opa: None,
            border_color: None,
            border_width: None,
            border_opa: None,
            outline_color: None,
            outline_width: None,
            outline_opa: None,
            outline_pad: None,
            opa: None,
        }
    }

    pub fn overlay(color: Color, opa: u8) -> Self {
        Self {
            bg_color: Some(color),
            bg_opa: Some(opa),
            ..Self::transparent()
        }
    }

    pub fn outline(color: Color, width: i32) -> Self {
        Self {
            outline_color: Some(color),
            outline_width: Some(width),
            outline_opa: Some(255),
            outline_pad: Some(0),
            ..Self::transparent()
        }
    }

    pub const fn opacity(opa: u8) -> Self {
        Self {
            opa: Some(opa),
            ..Self::transparent()
        }
    }
}

struct CellRuntime {
    definition: ParcelLockerCell,
    overlay: *mut c_bindings::lv_obj_t,
    status: CellStatusId,
    disabled: bool,
}

struct ParcelLockerInner {
    rows: usize,
    cols: usize,
    cells: Vec<CellRuntime>,
    selected: Option<usize>,
    default_style: CellStyle,
    selected_style: CellStyle,
    disabled_style: CellStyle,
    status_styles: BTreeMap<CellStatusId, CellStyle>,
}

struct CellEventCtx {
    inner: Rc<RefCell<ParcelLockerInner>>,
    callback: Rc<RefCell<Option<Box<dyn FnMut(CellTap)>>>>,
    index: usize,
    overlay: *mut c_bindings::lv_obj_t,
}

pub struct ParcelLocker {
    root: LvObj,
    inner: Rc<RefCell<ParcelLockerInner>>,
    callback: Rc<RefCell<Option<Box<dyn FnMut(CellTap)>>>>,
    event_contexts: Vec<Box<CellEventCtx>>,
}

#[derive(Copy, Clone)]
struct ResolvedCellStyle {
    bg_color: Color,
    bg_opa: u8,
    border_color: Color,
    border_width: i32,
    border_opa: u8,
    outline_color: Color,
    outline_width: i32,
    outline_opa: u8,
    outline_pad: i32,
    opa: u8,
}

impl ResolvedCellStyle {
    fn blank() -> Self {
        Self {
            bg_color: Color::black(),
            bg_opa: 0,
            border_color: Color::black(),
            border_width: 0,
            border_opa: 0,
            outline_color: Color::black(),
            outline_width: 0,
            outline_opa: 0,
            outline_pad: 0,
            opa: 255,
        }
    }

    fn apply_patch(&mut self, patch: CellStyle) {
        if let Some(value) = patch.bg_color {
            self.bg_color = value;
        }
        if let Some(value) = patch.bg_opa {
            self.bg_opa = value;
        }
        if let Some(value) = patch.border_color {
            self.border_color = value;
        }
        if let Some(value) = patch.border_width {
            self.border_width = value;
        }
        if let Some(value) = patch.border_opa {
            self.border_opa = value;
        }
        if let Some(value) = patch.outline_color {
            self.outline_color = value;
        }
        if let Some(value) = patch.outline_width {
            self.outline_width = value;
        }
        if let Some(value) = patch.outline_opa {
            self.outline_opa = value;
        }
        if let Some(value) = patch.outline_pad {
            self.outline_pad = value;
        }
        if let Some(value) = patch.opa {
            self.opa = value;
        }
    }
}

fn apply_resolved_style(obj: *mut c_bindings::lv_obj_t, style: ResolvedCellStyle) {
    unsafe {
        c_bindings::lv_obj_set_style_bg_color(obj, style.bg_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_bg_opa(obj, style.bg_opa, 0);
        c_bindings::lv_obj_set_style_border_color(obj, style.border_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_border_width(obj, style.border_width, 0);
        c_bindings::lv_obj_set_style_border_opa(obj, style.border_opa, 0);
        c_bindings::lv_obj_set_style_outline_color(obj, style.outline_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_outline_width(obj, style.outline_width, 0);
        c_bindings::lv_obj_set_style_outline_opa(obj, style.outline_opa, 0);
        c_bindings::lv_obj_set_style_outline_pad(obj, style.outline_pad, 0);
        c_bindings::lv_obj_set_style_opa(obj, style.opa, 0);
    }
}

fn assert_cell_index(index: usize, len: usize) {
    assert!(
        index < len,
        "ParcelLocker cell index {} is out of range 0..{}",
        index,
        len
    );
}

unsafe extern "C" fn on_cell_clicked(e: *mut c_bindings::lv_event_t) {
    let ctx = unsafe { c_bindings::lv_event_get_user_data(e) } as *mut CellEventCtx;
    if ctx.is_null() {
        return;
    }

    let ctx = unsafe { &mut *ctx };
    let tap = {
        // End the inner borrow before invoking user callbacks; they may call
        // state-mutating APIs like set_status or set_disabled.
        let inner = ctx.inner.borrow();
        assert_cell_index(ctx.index, inner.cells.len());
        let cell = &inner.cells[ctx.index];
        CellTap {
            index: ctx.index,
            row: cell.definition.row,
            col: cell.definition.col,
            status: cell.status,
            disabled: cell.disabled,
        }
    };

    if let Some(callback) = ctx.callback.borrow_mut().as_mut() {
        callback(tap);
    }
}

impl Widget for ParcelLocker {
    fn lv_obj(&self) -> &LvObj {
        &self.root
    }

    fn delete(mut self) {
        self.unregister_cell_callbacks();
        unsafe {
            c_bindings::lv_obj_delete(self.root.raw());
        }
    }
}

impl Drop for ParcelLocker {
    fn drop(&mut self) {
        self.unregister_cell_callbacks();
    }
}

impl ParcelLocker {
    pub fn new(parent: &impl Widget, rows: usize, cols: usize, cells: &[ParcelLockerCell]) -> Self {
        validate_layout(rows, cols, cells);

        let root = unsafe { c_bindings::lv_obj_create(parent.lv_obj().raw()) };
        if root.is_null() {
            panic!("lv_obj_create returned null for ParcelLocker root");
        }

        let cell_count = cells.len();
        let mut runtimes = Vec::with_capacity(cell_count);
        let mut overlays = Vec::with_capacity(cell_count);
        for cell in cells {
            let overlay = unsafe { c_bindings::lv_obj_create(root) };
            if overlay.is_null() {
                panic!("lv_obj_create returned null for ParcelLocker cell overlay");
            }
            unsafe {
                c_bindings::lv_obj_set_pos(overlay, cell.rect.x, cell.rect.y);
                c_bindings::lv_obj_set_size(overlay, cell.rect.w, cell.rect.h);
                c_bindings::lv_obj_add_flag(overlay, LvObjFlag::CLICKABLE.0);
            }
            overlays.push(overlay);
            runtimes.push(CellRuntime {
                definition: *cell,
                overlay,
                status: CellStatusId::DEFAULT,
                disabled: false,
            });
        }

        let inner = Rc::new(RefCell::new(ParcelLockerInner {
            rows,
            cols,
            cells: runtimes,
            selected: None,
            default_style: CellStyle::transparent(),
            selected_style: CellStyle::outline(Color::hex(0x00AEEF), 3),
            disabled_style: CellStyle::opacity(160),
            status_styles: BTreeMap::new(),
        }));

        let callback = Rc::new(RefCell::new(None));
        let mut event_contexts = Vec::with_capacity(cell_count);

        // Register callbacks with stable raw pointers
        for (index, overlay) in overlays.into_iter().enumerate() {
            let mut ctx = Box::new(CellEventCtx {
                inner: inner.clone(),
                callback: callback.clone(),
                index,
                overlay,
            });
            let raw = ctx.as_mut() as *mut CellEventCtx;
            unsafe {
                c_bindings::lv_obj_add_event_cb(
                    overlay,
                    Some(on_cell_clicked),
                    c_bindings::LV_EVENT_CLICKED,
                    raw as *mut c_void,
                );
            }
            event_contexts.push(ctx);
        }

        ParcelLocker {
            root: LvObj::from_raw(root),
            inner,
            callback,
            event_contexts,
        }
    }

    pub fn background(&self, src: ImageSrc) -> &Self {
        let obj = self.root.raw();
        set_retained_src_for_obj(obj, ImageRetentionSlot::Background, src, |src_ptr| unsafe {
            c_bindings::lv_obj_set_style_bg_image_src(obj, src_ptr, 0);
        });
        self
    }

    pub fn default_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().default_style = style;
        self.restyle_all();
        self
    }

    pub fn status_style(&self, status: CellStatusId, style: CellStyle) -> &Self {
        self.inner.borrow_mut().status_styles.insert(status, style);
        self.restyle_all_matching_status(status);
        self
    }

    pub fn selected_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().selected_style = style;
        if let Some(index) = self.inner.borrow().selected {
            self.restyle_cell(index);
        }
        self
    }

    pub fn disabled_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().disabled_style = style;
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            if self.inner.borrow().cells[index].disabled {
                self.restyle_cell(index);
            }
        }
        self
    }

    pub fn set_status(&self, index: usize, status: CellStatusId) -> &Self {
        let len = self.inner.borrow().cells.len();
        assert_cell_index(index, len);
        self.inner.borrow_mut().cells[index].status = status;
        self.restyle_cell(index);
        self
    }

    pub fn cell_status(&self, index: usize) -> CellStatusId {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].status
    }

    pub fn set_selected(&self, selected: Option<usize>) -> &Self {
        let len = self.inner.borrow().cells.len();
        if let Some(index) = selected {
            assert_cell_index(index, len);
        }

        let previous = self.inner.borrow().selected;
        if previous == selected {
            return self;
        }

        self.inner.borrow_mut().selected = selected;
        if let Some(index) = previous {
            self.restyle_cell(index);
        }
        if let Some(index) = selected {
            self.restyle_cell(index);
        }
        self
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner.borrow().selected
    }

    pub fn clear_selected(&self) -> &Self {
        self.set_selected(None)
    }

    pub fn set_disabled(&self, index: usize, disabled: bool) -> &Self {
        let len = self.inner.borrow().cells.len();
        assert_cell_index(index, len);
        self.inner.borrow_mut().cells[index].disabled = disabled;
        self.restyle_cell(index);
        self
    }

    pub fn is_disabled(&self, index: usize) -> bool {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].disabled
    }

    fn restyle_all(&self) {
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            self.restyle_cell(index);
        }
    }

    fn restyle_all_matching_status(&self, status: CellStatusId) {
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            if self.inner.borrow().cells[index].status == status {
                self.restyle_cell(index);
            }
        }
    }

    fn restyle_cell(&self, index: usize) {
        let (overlay, resolved) = {
            let inner = self.inner.borrow();
            assert_cell_index(index, inner.cells.len());
            let cell = &inner.cells[index];
            let mut resolved = ResolvedCellStyle::blank();
            resolved.apply_patch(inner.default_style);
            if let Some(status_style) = inner.status_styles.get(&cell.status).copied() {
                resolved.apply_patch(status_style);
            }
            if cell.disabled {
                resolved.apply_patch(inner.disabled_style);
            }
            if inner.selected == Some(index) {
                resolved.apply_patch(inner.selected_style);
            }
            (cell.overlay, resolved)
        };

        apply_resolved_style(overlay, resolved);
    }

    fn unregister_cell_callbacks(&mut self) {
        for ctx in &mut self.event_contexts {
            unsafe {
                c_bindings::lv_obj_remove_event_cb_with_user_data(
                    ctx.overlay,
                    Some(on_cell_clicked),
                    ctx.as_mut() as *mut CellEventCtx as *mut c_void,
                );
            }
        }
        self.event_contexts.clear();
    }

    pub fn on_cell_tap(&self, f: impl FnMut(CellTap) + 'static) -> &Self {
        *self.callback.borrow_mut() = Some(Box::new(f));
        self
    }

    #[cfg(test)]
    fn cell_overlay_raw(&self, index: usize) -> usize {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].overlay as usize
    }
}

pub(crate) fn validate_layout(rows: usize, cols: usize, cells: &[ParcelLockerCell]) {
    assert!(
        rows > 0 && cols > 0,
        "ParcelLocker matrix dimensions must be non-zero"
    );
    assert!(!cells.is_empty(), "ParcelLocker requires at least one cell");

    let mut seen = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        assert!(
            cell.rect.w > 0 && cell.rect.h > 0,
            "ParcelLocker cell {} rectangle must have positive width and height",
            index
        );
        assert!(
            cell.row < rows,
            "ParcelLocker cell {} row {} is outside row count {}",
            index,
            cell.row,
            rows
        );
        assert!(
            cell.col < cols,
            "ParcelLocker cell {} column {} is outside column count {}",
            index,
            cell.col,
            cols
        );
        assert!(
            seen.insert((cell.row, cell.col)),
            "ParcelLocker duplicate matrix coordinate row {} column {}",
            cell.row,
            cell.col
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CELLS: &[ParcelLockerCell] = &[
        ParcelLockerCell::new(0, 0, CellRect::new(10, 20, 80, 60)),
        ParcelLockerCell::new(0, 1, CellRect::new(96, 20, 80, 60)),
        ParcelLockerCell::new(1, 0, CellRect::new(10, 86, 80, 120)),
    ];

    #[test]
    fn cell_rect_constructor_stores_geometry() {
        let rect = CellRect::new(10, 20, 80, 60);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.w, 80);
        assert_eq!(rect.h, 60);
    }

    #[test]
    fn parcel_locker_cell_constructor_stores_metadata() {
        let cell = ParcelLockerCell::new(1, 2, CellRect::new(3, 4, 5, 6));
        assert_eq!(cell.row, 1);
        assert_eq!(cell.col, 2);
        assert_eq!(cell.rect, CellRect::new(3, 4, 5, 6));
    }

    #[test]
    fn validate_layout_accepts_unique_cells_inside_matrix() {
        validate_layout(2, 2, CELLS);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker matrix dimensions must be non-zero")]
    fn validate_layout_rejects_zero_dimensions() {
        validate_layout(0, 2, CELLS);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker requires at least one cell")]
    fn validate_layout_rejects_empty_cells() {
        validate_layout(2, 2, &[]);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 rectangle must have positive width and height")]
    fn validate_layout_rejects_non_positive_rectangles() {
        static BAD: &[ParcelLockerCell] =
            &[ParcelLockerCell::new(0, 0, CellRect::new(0, 0, 0, 20))];
        validate_layout(1, 1, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 row 2 is outside row count 2")]
    fn validate_layout_rejects_row_outside_matrix() {
        static BAD: &[ParcelLockerCell] =
            &[ParcelLockerCell::new(2, 0, CellRect::new(0, 0, 10, 10))];
        validate_layout(2, 1, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 column 3 is outside column count 3")]
    fn validate_layout_rejects_column_outside_matrix() {
        static BAD: &[ParcelLockerCell] =
            &[ParcelLockerCell::new(0, 3, CellRect::new(0, 0, 10, 10))];
        validate_layout(1, 3, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker duplicate matrix coordinate row 0 column 0")]
    fn validate_layout_rejects_duplicate_row_column() {
        static BAD: &[ParcelLockerCell] = &[
            ParcelLockerCell::new(0, 0, CellRect::new(0, 0, 10, 10)),
            ParcelLockerCell::new(0, 0, CellRect::new(12, 0, 10, 10)),
        ];
        validate_layout(1, 1, BAD);
    }

    fn setup() -> crate::lvgl::Screen {
        crate::c_bindings::reset_all_thread_local_spy_state();
        crate::lvgl::Screen::active()
    }

    #[test]
    fn new_creates_root_and_one_overlay_per_cell() {
        let screen = setup();
        let _locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        let calls = crate::c_bindings::spy_drain();
        let creates: Vec<_> = calls
            .iter()
            .filter_map(|call| match call {
                crate::c_bindings::LvCall::ObjCreate { obj, parent } => Some((*obj, *parent)),
                _ => None,
            })
            .collect();

        assert_eq!(
            creates.len(),
            4,
            "expected root plus three overlays: {:?}",
            calls
        );
        let root = creates[0].0;
        assert_eq!(
            creates[1].1, root,
            "first overlay should be parented to root"
        );
        assert_eq!(
            creates[2].1, root,
            "second overlay should be parented to root"
        );
        assert_eq!(
            creates[3].1, root,
            "third overlay should be parented to root"
        );
    }

    #[test]
    fn new_positions_and_sizes_each_overlay_from_cell_rects() {
        let screen = setup();
        let _locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        let calls = crate::c_bindings::spy_drain();

        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::ObjSetPos { x: 10, y: 20, .. })),
            "missing first overlay position: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::ObjSetSize { w: 80, h: 60, .. }
            )),
            "missing first overlay size: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::ObjSetPos { x: 96, y: 20, .. })),
            "missing second overlay position: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::ObjSetSize { w: 80, h: 120, .. }
            )),
            "missing tall overlay size: {:?}",
            calls
        );
    }

    #[test]
    fn background_applies_image_to_root() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        crate::c_bindings::spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { crate::lvgl::ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };

        locker.background(src);

        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::SetStyleBgImageSrc { src: recorded, .. }
                    if *recorded == core::ptr::addr_of!(dummy) as usize
            )),
            "expected background image source call, got: {:?}",
            calls
        );
    }

    #[test]
    fn set_status_applies_status_style_to_target_overlay() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.status_style(
            CellStatusId(7),
            CellStyle::overlay(Color::hex(0x00AA00), 88),
        );
        crate::c_bindings::spy_drain();

        locker.set_status(1, CellStatusId(7));

        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::SetStyleBgOpa { opa: 88, .. })),
            "expected status style opacity, got: {:?}",
            calls
        );
        assert_eq!(locker.cell_status(1), CellStatusId(7));
    }

    #[test]
    fn set_status_without_mapping_falls_back_to_default_style() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.default_style(CellStyle::overlay(Color::hex(0x111111), 22));
        crate::c_bindings::spy_drain();

        locker.set_status(2, CellStatusId(99));

        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::SetStyleBgOpa { opa: 22, .. })),
            "expected default style fallback, got: {:?}",
            calls
        );
    }

    #[test]
    fn set_selected_moves_single_selection() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.selected_style(CellStyle::outline(Color::hex(0x00AEEF), 4));
        crate::c_bindings::spy_drain();

        locker.set_selected(Some(0));
        locker.set_selected(Some(2));

        let calls = crate::c_bindings::spy_drain();
        let outline_width_calls = calls
            .iter()
            .filter(|c| matches!(c, crate::c_bindings::LvCall::SetStyleOutlineWidth { .. }))
            .count();
        assert!(
            outline_width_calls >= 2,
            "expected prior and new selection restyles: {:?}",
            calls
        );
        assert_eq!(locker.selected(), Some(2));
    }

    #[test]
    fn set_selected_none_clears_selection() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_selected(Some(0));
        crate::c_bindings::spy_drain();

        locker.set_selected(None);

        assert_eq!(locker.selected(), None);
        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::SetStyleOutlineWidth { value: 0, .. }
            )),
            "expected cleared outline width, got: {:?}",
            calls
        );
    }

    #[test]
    fn disabled_state_is_queryable_and_restyled() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        crate::c_bindings::spy_drain();

        locker.set_disabled(1, true);

        assert!(locker.is_disabled(1));
        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::SetStyleOpa { opa: 160, .. })),
            "expected disabled opacity style, got: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell index 99 is out of range 0..3")]
    fn index_methods_panic_on_out_of_range() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_status(99, CellStatusId(1));
    }

    #[test]
    fn layered_styles_do_not_wipe_each_other() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.status_style(
            CellStatusId(5),
            CellStyle::overlay(Color::hex(0xFF8800), 88),
        );
        locker.disabled_style(CellStyle::opacity(160));
        locker.selected_style(CellStyle::outline(Color::hex(0x00AEEF), 4));
        locker.set_status(1, CellStatusId(5));
        locker.set_disabled(1, true);
        crate::c_bindings::spy_drain();

        locker.set_selected(Some(1));

        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::SetStyleBgOpa { opa: 88, .. })),
            "selected style must not wipe status bg opacity: {:?}",
            calls
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::c_bindings::LvCall::SetStyleOpa { opa: 160, .. })),
            "selected style must not wipe disabled opacity: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::SetStyleOutlineWidth { value: 4, .. }
            )),
            "selected style must set outline width: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell index 99 is out of range 0..3")]
    fn set_selected_panics_on_out_of_range() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_selected(Some(99));
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell index 99 is out of range 0..3")]
    fn cell_status_panics_on_out_of_range() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.cell_status(99);
    }

    #[test]
    fn cell_tap_callback_reports_index_metadata_status_and_disabled() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);

        locker.set_status(1, CellStatusId(7)).set_disabled(1, true);

        let captured = alloc::rc::Rc::new(core::cell::RefCell::new(None));
        let captured_for_cb = captured.clone();
        locker.on_cell_tap(move |tap| {
            *captured_for_cb.borrow_mut() = Some(tap);
        });

        let overlay = locker.cell_overlay_raw(1);
        crate::c_bindings::spy_emit_event(
            overlay as *mut crate::c_bindings::lv_obj_t,
            crate::c_bindings::LV_EVENT_CLICKED,
        );

        assert_eq!(
            captured.borrow().clone(),
            Some(CellTap {
                index: 1,
                row: 0,
                col: 1,
                status: CellStatusId(7),
                disabled: true,
            })
        );
    }

    #[test]
    fn disabled_cells_still_emit_callbacks() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_disabled(2, true);

        let fires = alloc::rc::Rc::new(core::cell::RefCell::new(0));
        let fires_for_cb = fires.clone();
        locker.on_cell_tap(move |tap| {
            assert_eq!(tap.index, 2);
            assert!(tap.disabled);
            *fires_for_cb.borrow_mut() += 1;
        });

        let overlay = locker.cell_overlay_raw(2);
        crate::c_bindings::spy_emit_event(
            overlay as *mut crate::c_bindings::lv_obj_t,
            crate::c_bindings::LV_EVENT_CLICKED,
        );

        assert_eq!(*fires.borrow(), 1);
    }

    #[test]
    fn drop_unregisters_callbacks_without_borrowing_inner() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        let inner = locker.inner.clone();
        let _borrow = inner.borrow_mut();

        drop(locker);
    }

    #[test]
    fn drop_unregisters_cell_event_callbacks() {
        let screen = setup();
        let overlay;
        {
            let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
            locker.on_cell_tap(|_| {});
            overlay = locker.cell_overlay_raw(0);
            crate::c_bindings::spy_drain();
        }

        let calls = crate::c_bindings::spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                crate::c_bindings::LvCall::RemoveEventCbWithUserData { obj, .. }
                    if *obj == overlay
            )),
            "expected event callback cleanup for overlay {overlay:#x}, got: {:?}",
            calls
        );
    }

    #[test]
    fn delete_unregisters_cell_callbacks_before_deleting_root() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.on_cell_tap(|_| {});
        let root = locker.lv_obj().raw() as usize;
        crate::c_bindings::spy_drain();

        locker.delete();

        let calls = crate::c_bindings::spy_drain();
        let root_delete_pos = calls
            .iter()
            .position(|c| matches!(c, crate::c_bindings::LvCall::ObjDelete { obj } if *obj == root))
            .expect("expected root delete call");
        let callback_removals: Vec<_> = calls
            .iter()
            .enumerate()
            .filter_map(|(index, call)| match call {
                crate::c_bindings::LvCall::RemoveEventCbWithUserData { .. } => Some(index),
                _ => None,
            })
            .collect();

        assert_eq!(
            callback_removals.len(),
            CELLS.len(),
            "expected one callback cleanup per overlay: {:?}",
            calls
        );
        assert!(
            callback_removals
                .iter()
                .all(|removal_pos| *removal_pos < root_delete_pos),
            "callback cleanup must happen before deleting root and child overlays: {:?}",
            calls
        );
    }
}

#[cfg(test)]
mod export_tests {
    #[test]
    fn prelude_exports_parcel_locker_types() {
        use crate::lvgl::prelude::*;

        let _status = CellStatusId::DEFAULT;
        let _rect = CellRect::new(0, 0, 10, 10);
        let _cell = ParcelLockerCell::new(0, 0, _rect);
        let _style = CellStyle::transparent();
    }
}
