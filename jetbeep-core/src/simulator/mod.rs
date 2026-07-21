pub mod config;
pub mod config_editor;
pub mod layouts;
pub mod state;
pub mod ui;

use std::cell::RefCell;
use std::path::Path;

use config::{CellConfig, LockerConfig};
use layouts::LayoutCatalog;
use state::CellState;

fn map_open_check_policy(policy: config::OpenCheckPolicy) -> state::OpenCheckPolicy {
    match policy {
        config::OpenCheckPolicy::None => state::OpenCheckPolicy::None,
        config::OpenCheckPolicy::Before => state::OpenCheckPolicy::Before,
        config::OpenCheckPolicy::After => state::OpenCheckPolicy::After,
        config::OpenCheckPolicy::Always => state::OpenCheckPolicy::Always,
    }
}

thread_local! {
    static CATALOG: RefCell<Option<LayoutCatalog>> = const { RefCell::new(None) };
    static ACTIVE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn init(config_path: &str, cli_override: Option<&str>) {
    log::info!("simulator: loading config from {}", config_path);

    let catalog = layouts::load_catalog(Path::new(config_path), cli_override)
        .unwrap_or_else(|e| panic!("Failed to load simulator catalog '{}': {}", config_path, e));

    log::info!(
        "simulator: loaded {} layout(s); default = \"{}\"",
        catalog.layouts.len(),
        catalog.default_name
    );

    let default = catalog.default_name.clone();
    ACTIVE.with(|a| *a.borrow_mut() = Some(default.clone()));

    ui::create_window(&catalog);

    CATALOG.with(|c| *c.borrow_mut() = Some(catalog));
    apply_layout(&default);
}

/// Switch the active layout: reset cell state and rebuild the lockers area.
pub fn apply_layout(name: &str) {
    // Clone the layout out of the catalog so we drop the CATALOG borrow before
    // calling into `ui::*` or `state::*` — this avoids re-entrancy panics if a
    // UI callback ever swaps the catalog or invokes `apply_layout` again.
    let layout = CATALOG.with(|catalog_cell| {
        let catalog = catalog_cell.borrow();
        let catalog = catalog.as_ref()?;
        catalog.get(name).cloned()
    });

    let layout = match layout {
        Some(l) => l,
        None => {
            if CATALOG.with(|c| c.borrow().is_none()) {
                log::error!("simulator: apply_layout called before init");
            } else {
                log::warn!("simulator: apply_layout(\"{}\") — unknown layout, ignoring", name);
            }
            return;
        }
    };

    let cells = build_cells(&layout.lockers);
    state::init_state(cells);
    ui::rebuild_lockers(&layout);

    ACTIVE.with(|a| *a.borrow_mut() = Some(layout.name.clone()));
    log::info!("simulator: switched to layout \"{}\"", layout.name);
}

pub fn active_layout() -> Option<String> {
    ACTIVE.with(|a| a.borrow().clone())
}

fn build_cells(lockers: &[LockerConfig]) -> Vec<CellState> {
    let mut cells = Vec::new();
    for locker in lockers {
        let policy = map_open_check_policy(locker.open_check_policy);
        if let Some(row_cells) = &locker.cells {
            for cell_cfg in row_cells {
                push_cell_state(&mut cells, cell_cfg, policy);
            }
        } else if let Some(columns) = &locker.columns {
            for column in columns {
                for cell_cfg in &column.cells {
                    push_cell_state(&mut cells, cell_cfg, policy);
                }
            }
        }
    }
    // Always inject the 3 service-rack compartments (board 0 / locks 1-3)
    // so every layout exposes the controller's service slots.
    for lock_id in 1..=3 {
        if !cells.iter().any(|c| c.board_id == 0 && c.lock_id == lock_id) {
            cells.push(CellState {
                board_id: 0,
                lock_id,
                door_state: state::DoorState::Closed,
                cell_name: format!("S{}", lock_id),
                size: "SVC".to_string(),
                open_check_policy: state::OpenCheckPolicy::None,
            });
        }
    }
    cells
}

fn push_cell_state(
    out: &mut Vec<CellState>,
    cell_cfg: &CellConfig,
    policy: state::OpenCheckPolicy,
) {
    if let Some(ref columns) = cell_cfg.columns {
        for col in columns {
            out.push(CellState {
                board_id: col.board_id,
                lock_id: col.lock_id,
                door_state: state::DoorState::Closed,
                cell_name: col.cell_name.clone(),
                size: col.size.clone(),
                open_check_policy: policy,
            });
        }
    } else if let (Some(board_id), Some(lock_id)) = (cell_cfg.board_id, cell_cfg.lock_id) {
        out.push(CellState {
            board_id,
            lock_id,
            door_state: state::DoorState::Closed,
            cell_name: cell_cfg.cell_name.clone().unwrap_or_default(),
            size: cell_cfg.size.clone().unwrap_or_default(),
            open_check_policy: policy,
        });
    } else {
        log::warn!(
            "simulator: dropping malformed cell (no columns and no board_id/lock_id) — \
             layouts validator should have caught this"
        );
    }
}
