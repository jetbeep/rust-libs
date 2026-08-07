//! Simulator LVGL UI — locker layout + keypad + barcode scanner.
//! Renders in its own SDL window, fully isolated from the main app display.

use std::ffi::c_void;

use crate::lvgl::*;
use super::config::{CellConfig, LockerColumn};
use super::state;

// ── Cell click callback data ──

struct CellCbData {
    board_id: u32,
    lock_id: u32,
}

// ── UI object references for refresh ──

struct CellUi {
    obj: *mut lv_obj_t,
    board_id: u32,
    lock_id: u32,
}

struct SimulatorUi {
    cells: Vec<CellUi>,
    barcode_input: *mut lv_obj_t,
    scan_btn: *mut lv_obj_t,
    sim_screen: *mut lv_obj_t,
    root: *mut lv_obj_t,
    lockers_panel: *mut lv_obj_t,
    ctrl_panel: *mut lv_obj_t,
    sim_disp: *mut lv_display_t,
    group: LvGroup,
    win_w: i32,
    win_h: i32,
    #[allow(dead_code)]
    layout_dropdown: Option<*mut lv_obj_t>,
}

/// UI object references for the live `user_params` editor modal, present only
/// while the modal is open.
struct ModalUi {
    root: *mut lv_obj_t,
    textarea: *mut lv_obj_t,
    error: *mut lv_obj_t,
    tab_config_btn: *mut lv_obj_t,
    tab_timing_btn: *mut lv_obj_t,
    config_title: *mut lv_obj_t,
    util_row: *mut lv_obj_t,
    phys_title: *mut lv_obj_t,
    door_open_row: *mut lv_obj_t,
    cell_status_row: *mut lv_obj_t,
    door_open_label: *mut lv_obj_t,
    cell_status_label: *mut lv_obj_t,
    door_open_ms: std::cell::Cell<u32>,
    cell_status_ms: std::cell::Cell<u32>,
    /// `false` = "Config" (user_params JSON) tab active, `true` = "Timing"
    /// (physical world simulation) tab active.
    showing_timing_tab: std::cell::Cell<bool>,
}

/// Step size and clamp for the physical-timing steppers (ms).
const PHYS_MS_STEP: u32 = 100;
const PHYS_MS_MAX: u32 = 10_000;

// Store UI refs in thread-local for refresh callbacks
std::thread_local! {
    static UI: std::cell::RefCell<Option<SimulatorUi>> = const { std::cell::RefCell::new(None) };
    static MODAL: std::cell::RefCell<Option<ModalUi>> = const { std::cell::RefCell::new(None) };
}

/// Pixel scale factor: config dimensions (cm-ish) → pixels
const SCALE: i32 = 3;
/// Default column-major scale used only by the catalog window-sizing math;
/// `rebuild_lockers` recomputes a per-layout scale that fills the viewport.
const COLUMN_SCALE: i32 = 2;
/// Min / max bounds for the dynamic column-major scale.
const COLUMN_SCALE_MIN: f32 = 0.2;
const COLUMN_SCALE_MAX: f32 = 6.0;
/// Window padding
const WIN_PAD: i32 = 15;
/// Gap between locker columns
const LOCKER_GAP: i32 = 20;
/// Right panel width (keypad + barcode)
const RIGHT_PANEL_W: i32 = 330;
/// Minimum right panel height (keypad 4x4 + barcode + labels)
const RIGHT_PANEL_MIN_H: i32 = 500;
/// Maximum on-screen width of the lockers viewport; larger layouts scroll.
const LOCKERS_VIEW_MAX_W: i32 = 255;
/// Maximum on-screen height of the lockers viewport; larger layouts scroll.
const LOCKERS_VIEW_MAX_H: i32 = 500;
/// Width of the service-rack mini-locker (board 0, locks 1-3).
const SERVICE_RACK_W: i32 = 60;
/// Per-cell height inside the service rack.
const SERVICE_CELL_H: i32 = 45;
/// How many columns of a column-major layout we aim to show at once.
/// Fractional — 2.5 means ~2 full columns + half of the third peeking, so
/// users see there's more to scroll to.
const VISIBLE_COLUMNS: f32 = 2.5;

/// One-time setup: SDL display, input devices, right panel (keypad + barcode + layout dropdown),
/// and the empty lockers panel. Sized to fit the largest layout in the catalog.
pub fn create_window(catalog: &super::layouts::LayoutCatalog) {
    let (locker_total_w, locker_max_h) = catalog_window_dims(catalog);
    let view_w = (locker_total_w + LOCKER_GAP + SERVICE_RACK_W + LOCKER_GAP).min(LOCKERS_VIEW_MAX_W);
    let view_h = locker_max_h.max(RIGHT_PANEL_MIN_H).min(LOCKERS_VIEW_MAX_H);
    let content_h = view_h + 40;
    let win_w = view_w + RIGHT_PANEL_W + WIN_PAD * 3;
    let win_h = content_h + WIN_PAD * 2;

    let main_disp = lv_display_get_default();
    let sim_disp = lv_sdl_window_create(win_w, win_h);
    lv_sdl_window_set_title(&sim_disp, "Locker Simulator");
    lv_display_set_default(&sim_disp);

    let mouse = lv_sdl_mouse_create();
    lv_indev_set_display(&mouse, &sim_disp);
    let kb = lv_sdl_keyboard_create();
    lv_indev_set_display(&kb, &sim_disp);

    let group = lv_group_create();
    lv_indev_set_group(&mouse, &group);
    lv_indev_set_group(&kb, &group);
    lv_group_set_default(&group);

    let screen = lv_screen_active();
    lv_obj_set_style_bg_color(&screen, lv_color_hex_fn(0x2B2B3D), 0);
    lv_obj_remove_flag(&screen, LV_OBJ_FLAG_SCROLLABLE);

    let root = lv_obj_create(&screen);
    lv_obj_set_size(&root, win_w, win_h);
    lv_obj_set_style_bg_opa(&root, 0, 0);
    lv_obj_set_style_border_width(&root, 0, 0);
    lv_obj_set_style_pad_all(&root, WIN_PAD, 0);
    lv_obj_set_style_pad_column(&root, WIN_PAD, 0);
    lv_obj_set_flex_flow(&root, LV_FLEX_FLOW_ROW);
    lv_obj_remove_flag(&root, LV_OBJ_FLAG_SCROLLABLE);

    // Lockers panel — empty for now; rebuild_lockers will populate it.
    // Horizontal scrolling kept (LV_OBJ_FLAG_SCROLLABLE on by default) for
    // wide layouts; vertical fit is enforced by column_scale_for_layout, so
    // no vertical scrollbar should ever appear.
    let lockers_panel = lv_obj_create(&root);
    lv_obj_set_size(&lockers_panel, view_w, content_h);
    lv_obj_set_style_bg_opa(&lockers_panel, 0, 0);
    lv_obj_set_style_border_width(&lockers_panel, 0, 0);
    lv_obj_set_style_pad_all(&lockers_panel, 0, 0);
    lv_obj_set_style_pad_column(&lockers_panel, LOCKER_GAP, 0);
    lv_obj_set_flex_flow(&lockers_panel, LV_FLEX_FLOW_ROW);
    lv_obj_set_scroll_dir(&lockers_panel, LV_DIR_HOR);

    // Right side: layout dropdown (optional), keypad, barcode.
    let ctrl_panel = lv_obj_create(&root);
    lv_obj_set_size(&ctrl_panel, RIGHT_PANEL_W, content_h);
    lv_obj_set_style_bg_opa(&ctrl_panel, 0, 0);
    lv_obj_set_style_border_width(&ctrl_panel, 0, 0);
    lv_obj_set_style_pad_all(&ctrl_panel, 0, 0);
    lv_obj_set_style_pad_row(&ctrl_panel, 15, 0);
    lv_obj_set_flex_flow(&ctrl_panel, LV_FLEX_FLOW_COLUMN);
    lv_obj_remove_flag(&ctrl_panel, LV_OBJ_FLAG_SCROLLABLE);

    let layout_dropdown = if catalog.layouts.len() > 1 {
        Some(create_layout_dropdown(&ctrl_panel, catalog).obj)
    } else {
        None
    };

    create_keypad(&ctrl_panel);
    let (barcode_input, scan_btn) = create_barcode_scanner(&ctrl_panel);
    lv_group_add_obj(&group, &barcode_input);
    // Floating gear button, pinned to the top-right corner of the screen.
    create_settings_button(&screen);

    let sim_disp_ptr = sim_disp.disp;
    UI.with(|ui| {
        *ui.borrow_mut() = Some(SimulatorUi {
            cells: Vec::new(),
            barcode_input: barcode_input.obj,
            scan_btn: scan_btn.obj,
            sim_screen: screen.obj,
            root: root.obj,
            lockers_panel: lockers_panel.obj,
            ctrl_panel: ctrl_panel.obj,
            sim_disp: sim_disp_ptr,
            group,
            win_w,
            win_h,
            layout_dropdown,
        });
    });
    std::mem::forget(barcode_input);
    std::mem::forget(scan_btn);
    std::mem::forget(lockers_panel);
    std::mem::forget(ctrl_panel);
    std::mem::forget(root);
    std::mem::forget(screen);

    state::set_ui_refresh(Box::new(refresh));
    state::set_invalid_lock_alert(Box::new(show_invalid_lock_alert));
    state::set_door_opened_notifier(Box::new(scroll_to_cell));

    lv_display_set_default(&main_disp);
    log::info!("simulator: window created ({}x{})", win_w, win_h);
}

/// Tear down the lockers panel's children and rebuild for the given layout.
pub fn rebuild_lockers(layout: &super::layouts::Layout) {
    UI.with(|ui_cell| {
        let mut ui_borrow = ui_cell.borrow_mut();
        let ui = match ui_borrow.as_mut() {
            Some(u) => u,
            None => return,
        };

        let panel = LvObj { obj: ui.lockers_panel };
        lv_obj_clean(&panel);

        // Autosize the SDL window + containers so the entire layout is
        // visible without horizontal scrolling. Sizes are derived from the
        // same scale factors used to render the cells below.
        let (view_w, view_h) = layout_view_dims(layout);
        let content_h = view_h + 40;
        let mut win_w = view_w + RIGHT_PANEL_W + WIN_PAD * 3;
        let mut win_h = content_h + WIN_PAD * 2;

        // Cap window at 80% of the monitor so it always fits on-screen.
        // Inner panels keep their natural size so the lockers panel scrolls
        // when clamping kicks in.
        if let Some((mon_w, mon_h)) = sdl_monitor_size() {
            let max_w = (mon_w as f32 * 0.8).round() as i32;
            let max_h = (mon_h as f32 * 0.8).round() as i32;
            if win_w > max_w { win_w = max_w; }
            if win_h > max_h { win_h = max_h; }
        }

        let sim_disp = LvDisplay { disp: ui.sim_disp };
        lv_sdl_window_set_resolution(&sim_disp, win_w, win_h);
        lv_sdl_window_set_title(&sim_disp, &format!("Locker Simulator — {}", layout.name));
        std::mem::forget(sim_disp);

        let root_obj = LvObj { obj: ui.root };
        lv_obj_set_size(&root_obj, win_w, win_h);
        std::mem::forget(root_obj);

        lv_obj_set_size(&panel, view_w, content_h);

        let ctrl_obj = LvObj { obj: ui.ctrl_panel };
        lv_obj_set_size(&ctrl_obj, RIGHT_PANEL_W, content_h);
        std::mem::forget(ctrl_obj);

        let mut cell_uis: Vec<CellUi> = Vec::new();

        // Dynamic column-major scale: separate horizontal and vertical
        // factors so cells fill the available height even when width is the
        // binding constraint (eliminates vertical gap under the lockers).
        let (scale_w_f, scale_h_f) = column_scale_for_layout(layout);
        let scale_w_px = |v: u32| ((v as f32) * scale_w_f).round() as i32;
        let scale_h_raw_px = |v: f32| (v * scale_h_f).round() as i32;

        for locker in &layout.lockers {
            if let Some(columns) = &locker.columns {
                let total_w: i32 = columns.iter().map(|c| scale_w_px(c.width)).sum::<i32>()
                    + 2 * (columns.len() as i32 - 1).max(0);
                let target_raw_h = locker_target_raw_column_height(columns);
                let max_col_h = scale_h_raw_px(target_raw_h);
                let locker_obj = lv_obj_create(&panel);
                lv_obj_set_size(&locker_obj, total_w, max_col_h);
                lv_obj_set_style_bg_opa(&locker_obj, 0, 0);
                lv_obj_set_style_border_width(&locker_obj, 0, 0);
                lv_obj_set_style_pad_all(&locker_obj, 0, 0);
                lv_obj_set_style_pad_column(&locker_obj, 2, 0);
                lv_obj_set_flex_flow(&locker_obj, LV_FLEX_FLOW_ROW);
                lv_obj_remove_flag(&locker_obj, LV_OBJ_FLAG_SCROLLABLE);

                for column in columns {
                    let col_w = scale_w_px(column.width);
                    let column_stretch = column_stretch(column, target_raw_h);
                    let gap_px = stretched_gap_px(column_stretch, scale_h_f);
                    let last_cell_idx = column.cells.len().saturating_sub(1);
                    let mut used_h = 0;
                    let col_obj = lv_obj_create(&locker_obj);
                    lv_obj_set_size(&col_obj, col_w, max_col_h);
                    lv_obj_set_style_bg_opa(&col_obj, 0, 0);
                    lv_obj_set_style_border_width(&col_obj, 0, 0);
                    lv_obj_set_style_pad_all(&col_obj, 0, 0);
                    lv_obj_set_style_pad_row(&col_obj, gap_px, 0);
                    lv_obj_set_flex_flow(&col_obj, LV_FLEX_FLOW_COLUMN);
                    lv_obj_remove_flag(&col_obj, LV_OBJ_FLAG_SCROLLABLE);

                    for (cell_idx, cell_cfg) in column.cells.iter().enumerate() {
                        let cell_h = if cell_idx == last_cell_idx {
                            (max_col_h - used_h - gap_px * last_cell_idx as i32).max(0)
                        } else {
                            let h = stretched_cell_height_px(cell_cfg, column_stretch, scale_h_f);
                            used_h += h;
                            h
                        };
                        if let (Some(board_id), Some(lock_id)) =
                            (cell_cfg.board_id, cell_cfg.lock_id)
                        {
                            let name = cell_cfg.cell_name.as_deref().unwrap_or("");
                            let size = cell_cfg.size.as_deref().unwrap_or("");
                            let cell_obj = create_cell_widget(
                                &col_obj, col_w, cell_h, name, size, board_id, lock_id,
                                cell_cfg.pinpad,
                            );
                            cell_uis.push(CellUi {
                                obj: cell_obj.obj,
                                board_id,
                                lock_id,
                            });
                            std::mem::forget(cell_obj);
                        } else {
                            // Filler / service slot: takes physical space but is
                            // not interactive.
                            let spacer = lv_obj_create(&col_obj);
                            lv_obj_set_size(&spacer, col_w, cell_h);
                            lv_obj_set_style_bg_opa(&spacer, 0, 0);
                            lv_obj_set_style_border_width(&spacer, 0, 0);
                            lv_obj_set_style_pad_all(&spacer, 0, 0);
                            lv_obj_remove_flag(&spacer, LV_OBJ_FLAG_SCROLLABLE);
                            std::mem::forget(spacer);
                        }
                    }

                    std::mem::forget(col_obj);
                }
                std::mem::forget(locker_obj);
            } else if let Some(row_cells) = &locker.cells {
                let (row_scale_w_f, row_scale_h_f) = row_scale_for_layout(layout);
                let scale_w_i = |v: u32| ((v as f32) * row_scale_w_f).round() as i32;
                let scale_h_i = |v: u32| ((v as f32) * row_scale_h_f).round() as i32;
                let col_w = scale_w_i(locker.width);
                let col = lv_obj_create(&panel);
                let col_h: i32 = row_cells.iter().map(|c| scale_h_i(c.height)).sum::<i32>()
                    + 2 * (row_cells.len() as i32 - 1).max(0);
                lv_obj_set_size(&col, col_w, col_h);
                lv_obj_set_style_bg_opa(&col, 0, 0);
                lv_obj_set_style_border_width(&col, 0, 0);
                lv_obj_set_style_pad_all(&col, 0, 0);
                lv_obj_set_style_pad_row(&col, 2, 0);
                lv_obj_set_flex_flow(&col, LV_FLEX_FLOW_COLUMN);
                lv_obj_remove_flag(&col, LV_OBJ_FLAG_SCROLLABLE);

                for cell_cfg in row_cells {
                    let cell_h = scale_h_i(cell_cfg.height);
                    if let Some(ref inner_columns) = cell_cfg.columns {
                        let row = lv_obj_create(&col);
                        lv_obj_set_size(&row, col_w, cell_h);
                        lv_obj_set_style_bg_opa(&row, 0, 0);
                        lv_obj_set_style_border_width(&row, 0, 0);
                        lv_obj_set_style_pad_all(&row, 0, 0);
                        lv_obj_set_style_pad_column(&row, 2, 0);
                        lv_obj_set_flex_flow(&row, LV_FLEX_FLOW_ROW);
                        lv_obj_remove_flag(&row, LV_OBJ_FLAG_SCROLLABLE);

                        for col_cfg in inner_columns {
                            let cell_obj = create_cell_widget(
                                &row, 0, cell_h,
                                &col_cfg.cell_name, &col_cfg.size,
                                col_cfg.board_id, col_cfg.lock_id, false,
                            );
                            lv_obj_set_flex_grow(&cell_obj, 1);
                            cell_uis.push(CellUi {
                                obj: cell_obj.obj,
                                board_id: col_cfg.board_id,
                                lock_id: col_cfg.lock_id,
                            });
                            std::mem::forget(cell_obj);
                        }
                    } else if let (Some(board_id), Some(lock_id)) = (cell_cfg.board_id, cell_cfg.lock_id) {
                        let name = cell_cfg.cell_name.as_deref().unwrap_or("");
                        let size = cell_cfg.size.as_deref().unwrap_or("");
                        let cell_obj = create_cell_widget(
                            &col, col_w, cell_h,
                            name, size, board_id, lock_id, cell_cfg.pinpad,
                        );
                        cell_uis.push(CellUi {
                            obj: cell_obj.obj, board_id, lock_id,
                        });
                        std::mem::forget(cell_obj);
                    }
                }
            }
        }

        // Always-on service rack: board 0, locks 1-3. Mirrors the controller's
        // dedicated service compartments and keeps them clickable in the UI
        // regardless of the active layout.
        let svc = lv_obj_create(&panel);
        let svc_h = SERVICE_CELL_H * 3 + 2 * 2 + 25;
        lv_obj_set_size(&svc, SERVICE_RACK_W, svc_h);
        lv_obj_set_style_bg_opa(&svc, 0, 0);
        lv_obj_set_style_border_width(&svc, 1, 0);
        lv_obj_set_style_border_color(&svc, lv_color_hex_fn(0x555570), 0);
        lv_obj_set_style_pad_all(&svc, 4, 0);
        lv_obj_set_style_pad_row(&svc, 2, 0);
        lv_obj_set_style_radius(&svc, 4, 0);
        lv_obj_set_flex_flow(&svc, LV_FLEX_FLOW_COLUMN);
        lv_obj_remove_flag(&svc, LV_OBJ_FLAG_SCROLLABLE);

        let svc_title = lv_label_create(&svc);
        lv_label_set_text(&svc_title, "Service");
        lv_obj_set_style_text_color(&svc_title, lv_color_hex_fn(0xCCCCCC), 0);
        lv_obj_set_style_text_font(&svc_title, &lv_font_montserrat_14(), 0);

        for lock_id in 1u32..=3 {
            let cell_obj = create_cell_widget(
                &svc,
                SERVICE_RACK_W - 10,
                SERVICE_CELL_H,
                &format!("S{}", lock_id),
                "SVC",
                0,
                lock_id,
                false,
            );
            cell_uis.push(CellUi {
                obj: cell_obj.obj,
                board_id: 0,
                lock_id,
            });
            std::mem::forget(cell_obj);
        }
        std::mem::forget(svc);

        ui.cells = cell_uis;
        std::mem::forget(panel);
    });

    refresh();
}

const CELL_GAP_RAW: f32 = 2.0;

fn column_raw_height(column: &LockerColumn) -> f32 {
    let cells_h: u32 = column.cells.iter().map(|cell| cell.height).sum();
    let gaps = CELL_GAP_RAW * (column.cells.len() as f32 - 1.0).max(0.0);
    cells_h as f32 + gaps
}

fn locker_target_raw_column_height(columns: &[LockerColumn]) -> f32 {
    columns.iter().map(column_raw_height).fold(0.0_f32, f32::max)
}

fn column_stretch(column: &LockerColumn, target_raw_h: f32) -> f32 {
    let col_raw_h = column_raw_height(column);
    if col_raw_h <= 0.0 || target_raw_h <= 0.0 {
        1.0
    } else {
        (target_raw_h / col_raw_h).max(1.0)
    }
}

fn stretched_cell_height_px(cell: &CellConfig, column_stretch: f32, scale_h_f: f32) -> i32 {
    ((cell.height as f32) * column_stretch * scale_h_f).round() as i32
}

fn stretched_gap_px(column_stretch: f32, scale_h_f: f32) -> i32 {
    (CELL_GAP_RAW * column_stretch * scale_h_f).round() as i32
}

/// Pick row-major scale factors (separate horizontal and vertical) so
/// row-major layouts also fill the viewport height instead of leaving a
/// gap below the last cell. Width side: locker width + service rack +
/// gaps should fit in `LOCKERS_VIEW_MAX_W`. Height side: tallest column
/// (sum of cell heights) should fit in `LOCKERS_VIEW_MAX_H`.
fn row_scale_for_layout(layout: &super::layouts::Layout) -> (f32, f32) {
    let max_raw_w: u32 = layout
        .lockers
        .iter()
        .filter(|l| l.cells.is_some())
        .map(|l| l.width)
        .max()
        .unwrap_or(0);

    let max_raw_h: u32 = layout
        .lockers
        .iter()
        .filter_map(|l| l.cells.as_ref())
        .map(|cells| cells.iter().map(|c| c.height).sum::<u32>())
        .max()
        .unwrap_or(0);

    if max_raw_w == 0 || max_raw_h == 0 {
        return (SCALE as f32, SCALE as f32);
    }

    let target_w =
        (LOCKERS_VIEW_MAX_W - SERVICE_RACK_W - LOCKER_GAP - LOCKER_GAP).max(60) as f32;
    let target_h = (LOCKERS_VIEW_MAX_H - 8).max(60) as f32;

    let scale_w = (target_w / max_raw_w as f32).clamp(COLUMN_SCALE_MIN, COLUMN_SCALE_MAX);
    let scale_h = (target_h / max_raw_h as f32).clamp(COLUMN_SCALE_MIN, COLUMN_SCALE_MAX);
    (scale_w, scale_h)
}

/// Pick column-major scale factors that satisfy BOTH constraints:
///  * the first `VISIBLE_COLUMNS` columns fit horizontally inside the
///    viewport,
///  * the tallest column fits vertically inside the viewport height.
/// Returns `(scale_w, scale_h)` independently so cells stretch to fill the
/// available height even when width is the binding constraint, avoiding
/// the vertical gap below short layouts. Both clamped to
/// [`COLUMN_SCALE_MIN`, `COLUMN_SCALE_MAX`].
fn column_scale_for_layout(layout: &super::layouts::Layout) -> (f32, f32) {
    // --- Width side: width of first VISIBLE_COLUMNS of the widest locker.
    let visible_raw_w: f32 = layout
        .lockers
        .iter()
        .filter_map(|l| l.columns.as_ref())
        .map(|cols| {
            let want = VISIBLE_COLUMNS.min(cols.len() as f32);
            let full = want.floor() as usize;
            let frac = want - full as f32;
            let full_w: u32 = cols.iter().take(full).map(|c| c.width).sum();
            let frac_w: f32 = cols
                .get(full)
                .map(|c| c.width as f32 * frac)
                .unwrap_or(0.0);
            let gaps = 2.0 * (full.saturating_sub(1) as f32 + if frac > 0.0 { 1.0 } else { 0.0 });
            full_w as f32 + frac_w + gaps
        })
        .fold(0.0_f32, f32::max);

    // --- Height side: tallest raw column target across the layout. Each
    //     shorter column is stretched to this target during rendering.
    let max_raw_col_h: f32 = layout
        .lockers
        .iter()
        .filter_map(|l| l.columns.as_ref())
        .map(|cols| locker_target_raw_column_height(cols))
        .fold(0.0_f32, f32::max);

    if visible_raw_w <= 0.0 || max_raw_col_h <= 0.0 {
        return (COLUMN_SCALE as f32, COLUMN_SCALE as f32);
    }

    let target_w = (LOCKERS_VIEW_MAX_W - LOCKER_GAP).max(100) as f32;
    let target_h = (LOCKERS_VIEW_MAX_H - 8).max(100) as f32;

    let scale_w = (target_w / visible_raw_w).clamp(COLUMN_SCALE_MIN, COLUMN_SCALE_MAX);
    let scale_h = (target_h / max_raw_col_h).clamp(COLUMN_SCALE_MIN, COLUMN_SCALE_MAX);
    (scale_w, scale_h)
}

/// Compute the pixel dimensions of the lockers viewport for `layout` using
/// the same scale logic applied during rendering. Used by `rebuild_lockers`
/// to autosize the SDL window so the entire layout is visible.
fn layout_view_dims(layout: &super::layouts::Layout) -> (i32, i32) {
    let (col_scale_w, col_scale_h) = column_scale_for_layout(layout);
    let (row_scale_w, row_scale_h) = row_scale_for_layout(layout);

    let mut total_w: i32 = 0;
    let mut max_h: i32 = 0;
    let mut locker_count: i32 = 0;

    for locker in &layout.lockers {
        locker_count += 1;
        if let Some(columns) = &locker.columns {
            let w: i32 = columns
                .iter()
                .map(|c| ((c.width as f32) * col_scale_w).round() as i32)
                .sum::<i32>()
                + 2 * (columns.len() as i32 - 1).max(0);
            let raw_h = locker_target_raw_column_height(columns);
            let h = (raw_h * col_scale_h).round() as i32;
            total_w += w;
            if h > max_h { max_h = h; }
        } else if let Some(cells) = &locker.cells {
            let w = ((locker.width as f32) * row_scale_w).round() as i32;
            let h: i32 = cells
                .iter()
                .map(|c| ((c.height as f32) * row_scale_h).round() as i32)
                .sum::<i32>()
                + 2 * (cells.len() as i32 - 1).max(0);
            total_w += w;
            if h > max_h { max_h = h; }
        }
    }

    // Gaps between lockers + service rack + gap before/after it.
    let gaps = LOCKER_GAP * (locker_count - 1).max(0);
    let svc_h = SERVICE_CELL_H * 3 + 2 * 2 + 25;
    let view_w = total_w + gaps + LOCKER_GAP + SERVICE_RACK_W;
    let view_h = max_h.max(svc_h).max(RIGHT_PANEL_MIN_H);
    (view_w.max(100), view_h)
}

fn catalog_window_dims(catalog: &super::layouts::LayoutCatalog) -> (i32, i32) {
    let mut max_w = 0i32;
    let mut max_h = 0i32;
    for layout in &catalog.layouts {
        let mut layout_w = 0i32;
        let mut layout_h = 0i32;
        for locker in &layout.lockers {
            if let Some(columns) = &locker.columns {
                let lw: i32 = columns.iter().map(|c| c.width as i32 * COLUMN_SCALE).sum::<i32>();
                let lh: i32 = columns
                    .iter()
                    .map(|c| c.cells.iter().map(|cell| cell.height as i32 * COLUMN_SCALE).sum::<i32>())
                    .max()
                    .unwrap_or(0);
                layout_w += lw;
                if lh > layout_h { layout_h = lh; }
            } else if let Some(cells) = &locker.cells {
                layout_w += locker.width as i32 * SCALE;
                let lh: i32 = cells.iter().map(|c| c.height as i32 * SCALE).sum::<i32>();
                if lh > layout_h { layout_h = lh; }
            }
        }
        layout_w += LOCKER_GAP * (layout.lockers.len() as i32 - 1).max(0);
        if layout_w > max_w { max_w = layout_w; }
        if layout_h > max_h { max_h = layout_h; }
    }
    if max_h == 0 { max_h = 400; }
    (max_w, max_h)
}

fn create_layout_dropdown(parent: &LvObj, catalog: &super::layouts::LayoutCatalog) -> LvObj {
    let title = lv_label_create(parent);
    lv_label_set_text(&title, "Layout");
    lv_obj_set_style_text_color(&title, lv_color_hex_fn(0xCCCCCC), 0);
    lv_obj_set_style_text_font(&title, &lv_font_montserrat_14(), 0);
    std::mem::forget(title);

    let options: String = catalog
        .layouts
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let dd = lv_dropdown_create_obj(parent);
    lv_obj_set_width(&dd, RIGHT_PANEL_W - 10);
    lv_dropdown_set_options_str(&dd, &options);

    let active = super::active_layout();
    if let Some(active_name) = active {
        if let Some(idx) = catalog.layouts.iter().position(|l| l.name == active_name) {
            lv_dropdown_set_selected_idx(&dd, idx as u32);
        }
    }

    lv_obj_add_event_cb(
        &dd,
        layout_dropdown_cb,
        LV_EVENT_VALUE_CHANGED,
        std::ptr::null_mut(),
    );

    dd
}

unsafe extern "C" fn layout_dropdown_cb(e: *mut lv_event_t) {
    let dd = lv_event_get_target_obj(e);
    let name = lv_dropdown_get_selected_text(&dd);
    std::mem::forget(dd);
    if !name.is_empty() {
        super::apply_layout(&name);
    }
}

fn create_cell_widget(
    parent: &LvObj,
    w: i32, h: i32,
    name: &str, size: &str,
    board_id: u32, lock_id: u32,
    has_pinpad: bool,
) -> LvObj {
    let cell = lv_obj_create(parent);
    lv_obj_set_size(&cell, w, h);
    lv_obj_set_style_radius(&cell, 4, 0);
    lv_obj_set_style_border_color(&cell, lv_color_hex_fn(0x888888), 0);
    lv_obj_set_style_border_width(&cell, 1, 0);
    lv_obj_set_style_pad_all(&cell, 5, 0);
    lv_obj_remove_flag(&cell, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_add_flag(&cell, LV_OBJ_FLAG_CLICKABLE);

    // Set initial color (closed = dark bg)
    apply_door_color(&cell, state::DoorState::Closed);

    // Label: "N - Size"
    let label_text = if size.is_empty() {
        name.to_string()
    } else {
        format!("{} - {}", name, size)
    };
    let label = lv_label_create(&cell);
    lv_label_set_text(&label, &label_text);
    lv_obj_set_style_text_color(&label, lv_color_hex_fn(0xDDDDDD), 0);
    lv_obj_set_style_text_font(&label, &lv_font_montserrat_14(), 0);

    // Pinpad indicator
    if has_pinpad {
        let icon = lv_label_create(&cell);
        lv_label_set_text(&icon, ":::");
        lv_obj_align(&icon, LvAlign::TopRight, -5, 5);
        lv_obj_set_style_text_color(&icon, lv_color_hex_fn(0x888888), 0);
    }

    // Click handler — cycle door state
    let cb_data = Box::new(CellCbData { board_id, lock_id });
    lv_obj_add_event_cb(
        &cell,
        cell_click_cb,
        LV_EVENT_CLICKED,
        Box::into_raw(cb_data) as *mut c_void,
    );

    cell
}

unsafe extern "C" fn cell_click_cb(e: *mut lv_event_t) {
    let user_data = lv_event_get_user_data(e);
    if user_data.is_null() {
        return;
    }
    let data = unsafe { &*(user_data as *const CellCbData) };
    state::door_cycle_state(data.board_id, data.lock_id);
}

fn apply_door_color(obj: &LvObj, door_state: state::DoorState) {
    let color = match door_state {
        state::DoorState::Closed => lv_color_hex_fn(0x3A3A4E),  // Dark gray
        state::DoorState::Open   => lv_color_hex_fn(0x2E7D32),  // Green
        state::DoorState::Error  => lv_color_hex_fn(0xC62828),  // Red
    };
    lv_obj_set_style_bg_color(obj, color, 0);
    lv_obj_set_style_bg_opa(obj, LV_OPA_COVER, 0);
}

fn create_keypad(parent: &LvObj) {
    // Keypad title
    let title = lv_label_create(parent);
    lv_label_set_text(&title, "Keypad");
    lv_obj_set_style_text_color(&title, lv_color_hex_fn(0xCCCCCC), 0);
    lv_obj_set_style_text_font(&title, &lv_font_montserrat_14(), 0);

    // 4x4 grid
    let keys: [[&str; 4]; 4] = [
        ["1", "2", "3", "A"],
        ["4", "5", "6", "B"],
        ["7", "8", "9", "C"],
        ["*", "0", "#", "D"],
    ];

    let grid = lv_obj_create(parent);
    let btn_size = 40;
    let gap = 4;
    let pad = 5;
    let grid_w = btn_size * 4 + gap * 3 + pad * 2;
    let grid_h = btn_size * 4 + gap * 3 + pad * 2;
    lv_obj_set_size(&grid, grid_w, grid_h);
    lv_obj_set_style_bg_opa(&grid, 0, 0);
    lv_obj_set_style_border_width(&grid, 0, 0);
    lv_obj_set_style_pad_all(&grid, 5, 0);
    lv_obj_set_style_pad_row(&grid, gap, 0);
    lv_obj_set_style_pad_column(&grid, gap, 0);
    lv_obj_set_flex_flow(&grid, LV_FLEX_FLOW_ROW_WRAP);
    lv_obj_remove_flag(&grid, LV_OBJ_FLAG_SCROLLABLE);

    for row in &keys {
        for &key_label in row {
            let btn = lv_button_create(&grid);
            lv_obj_set_size(&btn, btn_size, btn_size);
            lv_obj_set_style_bg_color(&btn, lv_color_hex_fn(0x4A4A5E), 0);
            lv_obj_set_style_radius(&btn, 6, 0);

            let label = lv_label_create(&btn);
            lv_label_set_text(&label, key_label);
            lv_obj_align(&label, LvAlign::Center, 0, 0);
            lv_obj_set_style_text_color(&label, lv_color_hex_fn(0xEEEEEE), 0);

            // Store key char in user_data (fits in a pointer)
            let key_char = key_label.chars().next().unwrap();
            let user_data = key_char as u32 as usize as *mut c_void;
            lv_obj_add_event_cb(&btn, keypad_click_cb, LV_EVENT_CLICKED, user_data);
        }
    }
}

unsafe extern "C" fn keypad_click_cb(e: *mut lv_event_t) {
    let user_data = lv_event_get_user_data(e);
    let key_char = user_data as usize as u32;
    if let Some(c) = char::from_u32(key_char) {
        if let Some(key) = state::KeypadKey::from_char(c) {
            state::keypad_press(key);
        }
    }
}

fn create_barcode_scanner(parent: &LvObj) -> (LvObj, LvObj) {
    // Title
    let title = lv_label_create(parent);
    lv_label_set_text(&title, "Barcode Scanner");
    lv_obj_set_style_text_color(&title, lv_color_hex_fn(0xCCCCCC), 0);
    lv_obj_set_style_text_font(&title, &lv_font_montserrat_14(), 0);

    // Container
    let cont = lv_obj_create(parent);
    lv_obj_set_width(&cont, RIGHT_PANEL_W);
    lv_obj_set_height(&cont, 90);
    lv_obj_set_style_bg_opa(&cont, 0, 0);
    lv_obj_set_style_border_width(&cont, 0, 0);
    lv_obj_set_style_pad_all(&cont, 0, 0);
    lv_obj_set_style_pad_row(&cont, 8, 0);
    lv_obj_set_flex_flow(&cont, LV_FLEX_FLOW_COLUMN);
    lv_obj_remove_flag(&cont, LV_OBJ_FLAG_SCROLLABLE);

    // Text input
    let input = lv_textarea_create(&cont);
    lv_obj_set_width(&input, RIGHT_PANEL_W - 10);
    lv_obj_set_height(&input, 36);
    lv_textarea_set_placeholder_text(&input, "Enter barcode...");
    lv_textarea_set_one_line(&input, true);
    lv_obj_add_flag(&input, LV_OBJ_FLAG_CLICK_FOCUSABLE);
    lv_obj_set_style_bg_color(&input, lv_color_hex_fn(0x3A3A4E), 0);
    lv_obj_set_style_text_color(&input, lv_color_hex_fn(0xEEEEEE), 0);
    lv_obj_set_style_border_color(&input, lv_color_hex_fn(0x666666), 0);

    // Scan button
    let btn = lv_button_create(&cont);
    lv_obj_set_width(&btn, RIGHT_PANEL_W - 10);
    lv_obj_set_height(&btn, 36);
    lv_obj_set_style_bg_color(&btn, lv_color_hex_fn(0x1565C0), 0);
    lv_obj_set_style_radius(&btn, 6, 0);

    let btn_label = lv_label_create(&btn);
    lv_label_set_text(&btn_label, "Scan");
    lv_obj_align(&btn_label, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&btn_label, lv_color_hex_fn(0xFFFFFF), 0);

    // Store textarea pointer as user_data for the button callback
    lv_obj_add_event_cb(&btn, scan_click_cb, LV_EVENT_CLICKED, input.obj as *mut c_void);

    // Initially disable only the Scan button (keep typing enabled)
    if !state::is_scanner_active() {
        lv_obj_add_state(&btn, LV_STATE_DISABLED);
    }

    (input, btn)
}

unsafe extern "C" fn scan_click_cb(e: *mut lv_event_t) {
    let user_data = lv_event_get_user_data(e);
    if user_data.is_null() {
        return;
    }
    // user_data is the textarea obj pointer
    let ta = LvObj { obj: user_data as *mut lv_obj_t };
    let text = lv_textarea_get_text(&ta);
    if !text.is_empty() {
        state::submit_barcode(text);
    }

    // After scanning, stop scanner and disable the Scan button until started again.
    state::scanner_stop();
    lv_textarea_set_text(&ta, "");

    std::mem::forget(ta); // don't drop — LVGL owns it
}

// ── Live user_params editor ──

/// FontAwesome gear glyph bundled with the Montserrat symbol range
/// (`LV_SYMBOL_SETTINGS`).
const SYMBOL_SETTINGS: &str = "\u{F013}";

/// Add a small floating gear button in the top-right corner that opens the live
/// `user_params` JSON editor modal. Right-aligned to the screen so it stays
/// pinned to the corner even when the window is resized on layout switches.
fn create_settings_button(screen: &LvObj) {
    let btn = lv_button_create(screen);
    lv_obj_set_size(&btn, 30, 30);
    lv_obj_align(&btn, LvAlign::TopRight, -6, 6);
    lv_obj_set_style_bg_color(&btn, lv_color_hex_fn(0x455A64), 0);
    lv_obj_set_style_radius(&btn, 15, 0);
    lv_obj_set_style_pad_all(&btn, 0, 0);

    let label = lv_label_create(&btn);
    lv_label_set_text(&label, SYMBOL_SETTINGS);
    lv_obj_align(&label, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&label, lv_color_hex_fn(0xFFFFFF), 0);

    lv_obj_add_event_cb(&btn, settings_click_cb, LV_EVENT_CLICKED, std::ptr::null_mut());
    std::mem::forget(label);
    std::mem::forget(btn);
}

unsafe extern "C" fn settings_click_cb(_e: *mut lv_event_t) {
    open_settings_modal();
}

/// Build and show the modal JSON editor, seeded with the current `user_params`.
fn open_settings_modal() {
    // Never stack two modals.
    if MODAL.with(|m| m.borrow().is_some()) {
        return;
    }

    let (screen_ptr, group_ptr, win_w, win_h) = match UI.with(|ui| {
        ui.borrow()
            .as_ref()
            .map(|u| (u.sim_screen, u.group.raw(), u.win_w, u.win_h))
    }) {
        Some(v) => v,
        None => return,
    };

    let screen = LvObj { obj: screen_ptr };

    // Full-screen dimmed backdrop (sized to the window so the centered panel
    // lands in the middle of the visible screen; scrollbars removed).
    let backdrop = lv_obj_create(&screen);
    lv_obj_set_size(&backdrop, win_w, win_h);
    lv_obj_set_pos(&backdrop, 0, 0);
    lv_obj_remove_flag(&backdrop, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_style_bg_color(&backdrop, lv_color_hex_fn(0x000000), 0);
    lv_obj_set_style_bg_opa(&backdrop, 200, 0);
    lv_obj_set_style_border_width(&backdrop, 0, 0);
    lv_obj_set_style_pad_all(&backdrop, 0, 0);
    lv_obj_set_style_radius(&backdrop, 0, 0);

    // Centered editor panel, clamped to fit smaller windows. `TAB_BAR_H`
    // reserves space for the "Config" / "Timing" tab bar at the top.
    const TAB_BAR_H: i32 = 40;
    let panel_w = 520.min(win_w - 20);
    let panel_h = (470 + TAB_BAR_H).min(win_h - 20);
    let inner_w = panel_w - 30;
    let ta_h = panel_h - 205 - TAB_BAR_H;
    let panel = lv_obj_create(&backdrop);
    lv_obj_set_size(&panel, panel_w, panel_h);
    lv_obj_align(&panel, LvAlign::Center, 0, 0);
    lv_obj_remove_flag(&panel, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_style_bg_color(&panel, lv_color_hex_fn(0x2B2B3D), 0);
    lv_obj_set_style_bg_opa(&panel, LV_OPA_COVER, 0);
    lv_obj_set_style_radius(&panel, 8, 0);
    lv_obj_set_style_border_color(&panel, lv_color_hex_fn(0x555566), 0);
    lv_obj_set_style_border_width(&panel, 1, 0);
    lv_obj_set_style_pad_all(&panel, 15, 0);
    lv_obj_set_style_pad_row(&panel, 10, 0);
    lv_obj_set_flex_flow(&panel, LV_FLEX_FLOW_COLUMN);

    // Tab bar: "Config" (user_params JSON) / "Timing" (physical world
    // simulation), so hardware-latency settings are visually separated from
    // the raw locker JSON config.
    let tab_row = lv_obj_create(&panel);
    lv_obj_set_width(&tab_row, inner_w);
    lv_obj_set_height(&tab_row, TAB_BAR_H - 6);
    lv_obj_set_style_bg_opa(&tab_row, 0, 0);
    lv_obj_set_style_border_width(&tab_row, 0, 0);
    lv_obj_set_style_pad_all(&tab_row, 0, 0);
    lv_obj_set_style_pad_column(&tab_row, 8, 0);
    lv_obj_set_flex_flow(&tab_row, LV_FLEX_FLOW_ROW);
    lv_obj_remove_flag(&tab_row, LV_OBJ_FLAG_SCROLLABLE);

    let tab_config_btn = lv_button_create(&tab_row);
    lv_obj_set_size(&tab_config_btn, 140, TAB_BAR_H - 6);
    lv_obj_set_style_radius(&tab_config_btn, 6, 0);
    let tab_config_lbl = lv_label_create(&tab_config_btn);
    lv_label_set_text(&tab_config_lbl, "Config");
    lv_obj_align(&tab_config_lbl, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&tab_config_lbl, lv_color_hex_fn(0xFFFFFF), 0);
    lv_obj_add_event_cb(&tab_config_btn, modal_tab_config_cb, LV_EVENT_CLICKED, std::ptr::null_mut());
    std::mem::forget(tab_config_lbl);

    let tab_timing_btn = lv_button_create(&tab_row);
    lv_obj_set_size(&tab_timing_btn, 140, TAB_BAR_H - 6);
    lv_obj_set_style_radius(&tab_timing_btn, 6, 0);
    let tab_timing_lbl = lv_label_create(&tab_timing_btn);
    lv_label_set_text(&tab_timing_lbl, "Timing");
    lv_obj_align(&tab_timing_lbl, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&tab_timing_lbl, lv_color_hex_fn(0xFFFFFF), 0);
    lv_obj_add_event_cb(&tab_timing_btn, modal_tab_timing_cb, LV_EVENT_CLICKED, std::ptr::null_mut());
    std::mem::forget(tab_timing_lbl);
    std::mem::forget(tab_row);

    // ── "Config" tab: raw user_params JSON editor ──────────────────────
    let config_title = lv_label_create(&panel);
    lv_label_set_text(&config_title, "Edit user_params (JSON)");
    lv_obj_set_style_text_color(&config_title, lv_color_hex_fn(0xEEEEEE), 0);
    lv_obj_set_style_text_font(&config_title, &lv_font_montserrat_14(), 0);

    let ta = lv_textarea_create(&panel);
    lv_obj_set_size(&ta, inner_w, ta_h);
    lv_textarea_set_one_line(&ta, false);
    lv_obj_set_style_bg_color(&ta, lv_color_hex_fn(0x3A3A4E), 0);
    lv_obj_set_style_text_color(&ta, lv_color_hex_fn(0xEEEEEE), 0);
    lv_obj_set_style_border_color(&ta, lv_color_hex_fn(0x666666), 0);
    let seed = super::config_editor::current_user_params_json()
        .unwrap_or_else(|| "{}".to_string());
    lv_textarea_set_text(&ta, &seed);
    // Route the SDL keyboard to the editor while the modal is open.
    let group = LvGroup::from_raw(group_ptr);
    lv_group_add_obj(&group, &ta);
    lv_group_focus_obj(&ta);
    std::mem::forget(group);

    // Utility row: Paste / Copy / Clear.
    let util_row = lv_obj_create(&panel);
    lv_obj_set_width(&util_row, inner_w);
    lv_obj_set_height(&util_row, 40);
    lv_obj_set_style_bg_opa(&util_row, 0, 0);
    lv_obj_set_style_border_width(&util_row, 0, 0);
    lv_obj_set_style_pad_all(&util_row, 0, 0);
    lv_obj_set_style_pad_column(&util_row, 8, 0);
    lv_obj_set_flex_flow(&util_row, LV_FLEX_FLOW_ROW);
    lv_obj_remove_flag(&util_row, LV_OBJ_FLAG_SCROLLABLE);
    make_modal_button(&util_row, 100, 0x37474F, "Paste", modal_paste_cb);
    make_modal_button(&util_row, 100, 0x37474F, "Copy", modal_copy_cb);
    make_modal_button(&util_row, 100, 0x6D4C41, "Clear", modal_clear_cb);

    // ── "Timing" tab: physical world simulation (door-open / cell-status
    // hardware latency), hidden until the user switches tabs ─────────────
    let timing = state::get_physical_timing();
    let phys_title = lv_label_create(&panel);
    lv_label_set_text(&phys_title, "Physical world simulation");
    lv_obj_set_style_text_color(&phys_title, lv_color_hex_fn(0xEEEEEE), 0);
    lv_obj_set_style_text_font(&phys_title, &lv_font_montserrat_14(), 0);
    lv_obj_add_flag(&phys_title, LV_OBJ_FLAG_HIDDEN);

    let (door_open_row, door_open_label) = make_stepper_row(
        &panel,
        inner_w,
        "Door opening time (ms)",
        timing.door_open_ms,
        stepper_door_dec_cb,
        stepper_door_inc_cb,
    );
    lv_obj_add_flag(&door_open_row, LV_OBJ_FLAG_HIDDEN);
    let (cell_status_row, cell_status_label) = make_stepper_row(
        &panel,
        inner_w,
        "Get cell door status (ms)",
        timing.cell_status_ms,
        stepper_cell_dec_cb,
        stepper_cell_inc_cb,
    );
    lv_obj_add_flag(&cell_status_row, LV_OBJ_FLAG_HIDDEN);

    let err = lv_label_create(&panel);
    lv_label_set_text(&err, "");
    lv_obj_set_style_text_color(&err, lv_color_hex_fn(0xE57373), 0);
    lv_obj_set_style_text_font(&err, &lv_font_montserrat_14(), 0);

    let row = lv_obj_create(&panel);
    lv_obj_set_width(&row, inner_w);
    lv_obj_set_height(&row, 44);
    lv_obj_set_style_bg_opa(&row, 0, 0);
    lv_obj_set_style_border_width(&row, 0, 0);
    lv_obj_set_style_pad_all(&row, 0, 0);
    lv_obj_set_style_pad_column(&row, 10, 0);
    lv_obj_set_flex_flow(&row, LV_FLEX_FLOW_ROW);
    lv_obj_remove_flag(&row, LV_OBJ_FLAG_SCROLLABLE);

    let save_btn = lv_button_create(&row);
    lv_obj_set_size(&save_btn, 110, 40);
    lv_obj_set_style_bg_color(&save_btn, lv_color_hex_fn(0x2E7D32), 0);
    lv_obj_set_style_radius(&save_btn, 6, 0);
    let save_lbl = lv_label_create(&save_btn);
    lv_label_set_text(&save_lbl, "Save & apply");
    lv_obj_align(&save_lbl, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&save_lbl, lv_color_hex_fn(0xFFFFFF), 0);
    lv_obj_add_event_cb(&save_btn, modal_save_cb, LV_EVENT_CLICKED, std::ptr::null_mut());

    let cancel_btn = lv_button_create(&row);
    lv_obj_set_size(&cancel_btn, 100, 40);
    lv_obj_set_style_bg_color(&cancel_btn, lv_color_hex_fn(0x555566), 0);
    lv_obj_set_style_radius(&cancel_btn, 6, 0);
    let cancel_lbl = lv_label_create(&cancel_btn);
    lv_label_set_text(&cancel_lbl, "Cancel");
    lv_obj_align(&cancel_lbl, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&cancel_lbl, lv_color_hex_fn(0xFFFFFF), 0);
    lv_obj_add_event_cb(&cancel_btn, modal_cancel_cb, LV_EVENT_CLICKED, std::ptr::null_mut());

    MODAL.with(|m| {
        *m.borrow_mut() = Some(ModalUi {
            root: backdrop.obj,
            textarea: ta.obj,
            error: err.obj,
            tab_config_btn: tab_config_btn.obj,
            tab_timing_btn: tab_timing_btn.obj,
            config_title: config_title.obj,
            util_row: util_row.obj,
            phys_title: phys_title.obj,
            door_open_row: door_open_row.obj,
            cell_status_row: cell_status_row.obj,
            door_open_label: door_open_label.obj,
            cell_status_label: cell_status_label.obj,
            door_open_ms: std::cell::Cell::new(timing.door_open_ms),
            cell_status_ms: std::cell::Cell::new(timing.cell_status_ms),
            showing_timing_tab: std::cell::Cell::new(false),
        });
    });
    // Config tab is active by default; style the tab buttons accordingly.
    set_tab_button_active(&tab_config_btn, true);
    set_tab_button_active(&tab_timing_btn, false);

    std::mem::forget(ta);
    std::mem::forget(err);
    std::mem::forget(tab_config_btn);
    std::mem::forget(tab_timing_btn);
    std::mem::forget(config_title);
    std::mem::forget(util_row);
    std::mem::forget(phys_title);
    std::mem::forget(door_open_row);
    std::mem::forget(cell_status_row);
    std::mem::forget(door_open_label);
    std::mem::forget(cell_status_label);
    std::mem::forget(save_btn);
    std::mem::forget(save_lbl);
    std::mem::forget(cancel_btn);
    std::mem::forget(cancel_lbl);
    std::mem::forget(row);
    std::mem::forget(panel);
    std::mem::forget(backdrop);
    std::mem::forget(screen);
}

unsafe extern "C" fn modal_save_cb(_e: *mut lv_event_t) {
    let (text, err_ptr, timing) = match MODAL.with(|m| {
        m.borrow().as_ref().map(|modal| {
            let ta = LvObj { obj: modal.textarea };
            let text = lv_textarea_get_text(&ta);
            std::mem::forget(ta);
            let timing = state::PhysicalTiming {
                door_open_ms: modal.door_open_ms.get(),
                cell_status_ms: modal.cell_status_ms.get(),
            };
            (text, modal.error, timing)
        })
    }) {
        Some(v) => v,
        None => return,
    };

    match super::config_editor::save_user_params_json(&text) {
        Ok(()) => {
            state::set_physical_timing(timing);
            super::config_editor::trigger_profile_reload();
            close_settings_modal();
        }
        Err(e) => {
            let err_label = LvObj { obj: err_ptr };
            lv_label_set_text(&err_label, &e);
            std::mem::forget(err_label);
        }
    }
}

unsafe extern "C" fn modal_cancel_cb(_e: *mut lv_event_t) {
    close_settings_modal();
}

/// Create a labeled row with `-`/`+` steppers for one physical-timing value,
/// seeded from `initial_ms`. Returns `(row, value_label)`: the row so callers
/// can hide/show the whole control when switching tabs, and the value label
/// so stepper callbacks can refresh its text.
fn make_stepper_row(
    panel: &LvObj,
    inner_w: i32,
    label_text: &str,
    initial_ms: u32,
    dec_cb: lv_event_cb_t,
    inc_cb: lv_event_cb_t,
) -> (LvObj, LvObj) {
    let row = lv_obj_create(panel);
    lv_obj_set_width(&row, inner_w);
    lv_obj_set_height(&row, 34);
    lv_obj_set_style_bg_opa(&row, 0, 0);
    lv_obj_set_style_border_width(&row, 0, 0);
    lv_obj_set_style_pad_all(&row, 0, 0);
    lv_obj_set_style_pad_column(&row, 8, 0);
    lv_obj_set_flex_flow(&row, LV_FLEX_FLOW_ROW);
    lv_obj_set_flex_align(
        &row,
        LV_FLEX_ALIGN_START,
        LV_FLEX_ALIGN_CENTER,
        LV_FLEX_ALIGN_CENTER,
    );
    lv_obj_remove_flag(&row, LV_OBJ_FLAG_SCROLLABLE);

    let title = lv_label_create(&row);
    lv_label_set_text(&title, label_text);
    lv_obj_set_style_text_color(&title, lv_color_hex_fn(0xCCCCCC), 0);
    // Grow to fill remaining space so the stepper lands at the row's right edge.
    lv_obj_set_flex_grow(&title, 1);
    std::mem::forget(title);

    let stepper = lv_obj_create(&row);
    lv_obj_set_height(&stepper, 34);
    lv_obj_set_width(&stepper, 150);
    lv_obj_set_style_bg_opa(&stepper, 0, 0);
    lv_obj_set_style_border_width(&stepper, 0, 0);
    lv_obj_set_style_pad_all(&stepper, 0, 0);
    lv_obj_set_style_pad_column(&stepper, 8, 0);
    lv_obj_set_flex_flow(&stepper, LV_FLEX_FLOW_ROW);
    lv_obj_set_flex_align(
        &stepper,
        LV_FLEX_ALIGN_CENTER,
        LV_FLEX_ALIGN_CENTER,
        LV_FLEX_ALIGN_CENTER,
    );
    lv_obj_remove_flag(&stepper, LV_OBJ_FLAG_SCROLLABLE);

    make_modal_button(&stepper, 34, 0x37474F, "-", dec_cb);

    let value_label = lv_label_create(&stepper);
    lv_label_set_text(&value_label, &initial_ms.to_string());
    lv_obj_set_style_text_color(&value_label, lv_color_hex_fn(0xEEEEEE), 0);
    lv_obj_set_width(&value_label, 50);

    make_modal_button(&stepper, 34, 0x37474F, "+", inc_cb);

    std::mem::forget(stepper);
    (row, value_label)
}

/// Highlight the active tab button (accent bg) vs. the inactive one (muted bg).
fn set_tab_button_active(btn: &LvObj, active: bool) {
    let color = if active { 0x2E7D32 } else { 0x37474F };
    lv_obj_set_style_bg_color(btn, lv_color_hex_fn(color), 0);
}

/// Show the "Config" (user_params JSON) tab and hide the "Timing" tab.
unsafe extern "C" fn modal_tab_config_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            if !modal.showing_timing_tab.get() {
                return;
            }
            modal.showing_timing_tab.set(false);
            set_tab_visibility(modal, false);
        }
    });
}

/// Show the "Timing" (physical world simulation) tab and hide "Config".
unsafe extern "C" fn modal_tab_timing_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            if modal.showing_timing_tab.get() {
                return;
            }
            modal.showing_timing_tab.set(true);
            set_tab_visibility(modal, true);
        }
    });
}

/// Toggle widget visibility and tab button styling for `showing_timing`.
fn set_tab_visibility(modal: &ModalUi, showing_timing: bool) {
    let config_widgets = [modal.config_title, modal.textarea, modal.util_row];
    let timing_widgets = [modal.phys_title, modal.door_open_row, modal.cell_status_row];
    for ptr in config_widgets {
        let obj = LvObj { obj: ptr };
        if showing_timing {
            lv_obj_add_flag(&obj, LV_OBJ_FLAG_HIDDEN);
        } else {
            lv_obj_remove_flag(&obj, LV_OBJ_FLAG_HIDDEN);
        }
        std::mem::forget(obj);
    }
    for ptr in timing_widgets {
        let obj = LvObj { obj: ptr };
        if showing_timing {
            lv_obj_remove_flag(&obj, LV_OBJ_FLAG_HIDDEN);
        } else {
            lv_obj_add_flag(&obj, LV_OBJ_FLAG_HIDDEN);
        }
        std::mem::forget(obj);
    }
    let config_btn = LvObj { obj: modal.tab_config_btn };
    let timing_btn = LvObj { obj: modal.tab_timing_btn };
    set_tab_button_active(&config_btn, !showing_timing);
    set_tab_button_active(&timing_btn, showing_timing);
    std::mem::forget(config_btn);
    std::mem::forget(timing_btn);
}


/// Adjust one of the modal's in-memory timing values by `delta_ms` (clamped
/// to `[0, PHYS_MS_MAX]`) and refresh its label. `delta_ms` may be negative.
fn adjust_stepper(value: &std::cell::Cell<u32>, label_ptr: *mut lv_obj_t, delta_ms: i32) {
    let current = value.get() as i32;
    let next = (current + delta_ms).clamp(0, PHYS_MS_MAX as i32) as u32;
    value.set(next);
    let label = LvObj { obj: label_ptr };
    lv_label_set_text(&label, &next.to_string());
    std::mem::forget(label);
}

unsafe extern "C" fn stepper_door_dec_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            adjust_stepper(&modal.door_open_ms, modal.door_open_label, -(PHYS_MS_STEP as i32));
        }
    });
}

unsafe extern "C" fn stepper_door_inc_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            adjust_stepper(&modal.door_open_ms, modal.door_open_label, PHYS_MS_STEP as i32);
        }
    });
}

unsafe extern "C" fn stepper_cell_dec_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            adjust_stepper(&modal.cell_status_ms, modal.cell_status_label, -(PHYS_MS_STEP as i32));
        }
    });
}

unsafe extern "C" fn stepper_cell_inc_cb(_e: *mut lv_event_t) {
    MODAL.with(|m| {
        if let Some(modal) = m.borrow().as_ref() {
            adjust_stepper(&modal.cell_status_ms, modal.cell_status_label, PHYS_MS_STEP as i32);
        }
    });
}

/// Create a labeled button inside a modal row and wire its click callback.
fn make_modal_button(row: &LvObj, w: i32, bg: u32, text: &str, cb: lv_event_cb_t) {
    let btn = lv_button_create(row);
    lv_obj_set_size(&btn, w, 40);
    lv_obj_set_style_bg_color(&btn, lv_color_hex_fn(bg), 0);
    lv_obj_set_style_radius(&btn, 6, 0);
    let lbl = lv_label_create(&btn);
    lv_label_set_text(&lbl, text);
    lv_obj_align(&lbl, LvAlign::Center, 0, 0);
    lv_obj_set_style_text_color(&lbl, lv_color_hex_fn(0xFFFFFF), 0);
    lv_obj_add_event_cb(&btn, cb, LV_EVENT_CLICKED, std::ptr::null_mut());
    std::mem::forget(lbl);
    std::mem::forget(btn);
}

/// Insert the OS clipboard contents into the editor at the cursor.
unsafe extern "C" fn modal_paste_cb(_e: *mut lv_event_t) {
    let ta_ptr = match MODAL.with(|m| m.borrow().as_ref().map(|modal| modal.textarea)) {
        Some(p) => p,
        None => return,
    };
    let clip = sdl_get_clipboard_text();
    if clip.is_empty() {
        return;
    }
    let ta = LvObj { obj: ta_ptr };
    lv_textarea_add_text(&ta, &clip);
    std::mem::forget(ta);
}

/// Copy the entire editor contents to the OS clipboard.
unsafe extern "C" fn modal_copy_cb(_e: *mut lv_event_t) {
    let ta_ptr = match MODAL.with(|m| m.borrow().as_ref().map(|modal| modal.textarea)) {
        Some(p) => p,
        None => return,
    };
    let ta = LvObj { obj: ta_ptr };
    let text = lv_textarea_get_text(&ta);
    std::mem::forget(ta);
    sdl_set_clipboard_text(&text);
}

/// Clear the editor to an empty JSON object.
unsafe extern "C" fn modal_clear_cb(_e: *mut lv_event_t) {
    let ta_ptr = match MODAL.with(|m| m.borrow().as_ref().map(|modal| modal.textarea)) {
        Some(p) => p,
        None => return,
    };
    let ta = LvObj { obj: ta_ptr };
    lv_textarea_set_text(&ta, "{}");
    std::mem::forget(ta);
}

/// Delete the modal (and all its children) and clear the stored refs.
fn close_settings_modal() {
    if let Some(modal) = MODAL.with(|m| m.borrow_mut().take()) {
        lv_obj_delete(LvObj { obj: modal.root });
    }
}

/// Refresh all cell colors and scanner enabled state based on current simulator state.
pub fn refresh() {
    UI.with(|ui| {
        let ui = ui.borrow();
        let ui = match ui.as_ref() {
            Some(u) => u,
            None => return,
        };

        // Update cell colors
        for cell_ui in &ui.cells {
            let door_state = state::get_door_state(cell_ui.board_id, cell_ui.lock_id);
            let obj = LvObj { obj: cell_ui.obj };
            apply_door_color(&obj, door_state);
            std::mem::forget(obj);
        }

        // Update barcode scanner enabled/disabled
        let scanner_active = state::is_scanner_active();
        let input = LvObj { obj: ui.barcode_input };
        let btn = LvObj { obj: ui.scan_btn };

        // Keep textarea editable at all times so typed characters are visible.
        lv_obj_remove_state(&input, LV_STATE_DISABLED);
        if scanner_active {
            lv_obj_remove_state(&btn, LV_STATE_DISABLED);
        } else {
            lv_obj_add_state(&btn, LV_STATE_DISABLED);
        }
        std::mem::forget(input);
        std::mem::forget(btn);
    });
}

/// Auto-scroll the lockers panel so the cell at `(board_id, lock_id)` is in
/// view. Called from `state::lock_open` after a successful open.
pub fn scroll_to_cell(board_id: u32, lock_id: u32) {
    UI.with(|ui_cell| {
        let ui = ui_cell.borrow();
        let ui = match ui.as_ref() {
            Some(u) => u,
            None => return,
        };
        if let Some(cell) = ui
            .cells
            .iter()
            .find(|c| c.board_id == board_id && c.lock_id == lock_id)
        {
            let obj = LvObj { obj: cell.obj };
            lv_obj_scroll_to_view(&obj, LV_ANIM_ON);
            std::mem::forget(obj);
        }
    });
}

/// Show a modal msgbox on the simulator screen reporting an invalid
/// `(board, lock)` open attempt. Called from `state::lock_open` after a
/// range/catalog validation failure. Safe only on the LVGL/main thread —
/// all `bus::lock_open` call sites in this workspace dispatch through the
/// single-threaded `executor::run` from LVGL event callbacks, so this
/// holds today.
pub fn show_invalid_lock_alert(board_id: u32, lock_id: u32, reason: &str) {
    UI.with(|ui_cell| {
        let ui = ui_cell.borrow();
        let screen_ptr = match ui.as_ref() {
            Some(u) => u.sim_screen,
            None => return,
        };
        let sim_screen = LvObj { obj: screen_ptr };
        let body = format!(
            "board {} lock {} is out of controller range.\n\n{}",
            board_id, lock_id, reason
        );
        // `lv_msgbox_create` routes the new widget to the parent's display,
        // so passing the simulator screen as parent is enough — no need to
        // toggle the default display here.
        lv_msgbox_show(&sim_screen, "Invalid lock command", &body);
        std::mem::forget(sim_screen);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::{
        config::{CellConfig, LockerColumn, LockerConfig, OpenCheckPolicy},
        layouts::Layout,
    };

    fn cell(height: u32) -> CellConfig {
        CellConfig {
            cell_name: None,
            height,
            size: None,
            board_id: Some(1),
            lock_id: Some(1),
            pinpad: false,
            depth: None,
            columns: None,
        }
    }

    fn filler(height: u32) -> CellConfig {
        CellConfig {
            board_id: None,
            lock_id: None,
            ..cell(height)
        }
    }

    #[test]
    fn column_stretch_makes_short_column_visible_cells_reach_target_height() {
        let short = LockerColumn { width: 10, cells: vec![cell(10), filler(20)] };
        let tall = LockerColumn { width: 10, cells: vec![cell(70)] };
        let target_raw_h = locker_target_raw_column_height(&[short.clone(), tall.clone()]);
        let short_stretch = column_stretch(&short, target_raw_h);
        let tall_stretch = column_stretch(&tall, target_raw_h);

        let short_sum: i32 = short
            .cells
            .iter()
            .map(|cell| stretched_cell_height_px(cell, short_stretch, 1.0))
            .sum::<i32>()
            + stretched_gap_px(short_stretch, 1.0) * (short.cells.len() as i32 - 1);
        let tall_sum: i32 = tall
            .cells
            .iter()
            .map(|cell| stretched_cell_height_px(cell, tall_stretch, 1.0))
            .sum::<i32>()
            + stretched_gap_px(tall_stretch, 1.0) * (tall.cells.len() as i32 - 1);

        assert_eq!(target_raw_h, 70.0);
        assert_eq!(short_sum, 70);
        assert_eq!(tall_sum, 70);
    }

    #[test]
    fn column_scale_can_shrink_below_one_for_tall_layouts() {
        let layout = Layout {
            name: "tall".to_string(),
            lockers: vec![LockerConfig {
                width: 10,
                depth: 10,
                open_check_policy: OpenCheckPolicy::Always,
                cells: None,
                columns: Some(vec![LockerColumn {
                    width: 10,
                    cells: vec![cell(1_000)],
                }]),
            }],
        };

        let (_scale_w, scale_h) = column_scale_for_layout(&layout);
        assert!(scale_h < 1.0);
    }
}
