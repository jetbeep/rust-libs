use std::cell::RefCell;
use futures::channel::mpsc;

use crate::error::Error;
use crate::proto::bus::LockStatus;
use crate::simulator::config::validate_board_lock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Closed,
    Open,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeypadKey {
    Digit(u8),  // 0-9
    Star,       // *
    Hash,       // #
    A,
    B,
    C,
    D,
}

impl KeypadKey {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '0'..='9' => Some(KeypadKey::Digit(c as u8 - b'0')),
            '*' => Some(KeypadKey::Star),
            '#' => Some(KeypadKey::Hash),
            'A' | 'a' => Some(KeypadKey::A),
            'B' | 'b' => Some(KeypadKey::B),
            'C' | 'c' => Some(KeypadKey::C),
            'D' | 'd' => Some(KeypadKey::D),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            KeypadKey::Digit(0) => "0",
            KeypadKey::Digit(1) => "1",
            KeypadKey::Digit(2) => "2",
            KeypadKey::Digit(3) => "3",
            KeypadKey::Digit(4) => "4",
            KeypadKey::Digit(5) => "5",
            KeypadKey::Digit(6) => "6",
            KeypadKey::Digit(7) => "7",
            KeypadKey::Digit(8) => "8",
            KeypadKey::Digit(9) => "9",
            KeypadKey::Digit(_) => "?",
            KeypadKey::Star => "*",
            KeypadKey::Hash => "#",
            KeypadKey::A => "A",
            KeypadKey::B => "B",
            KeypadKey::C => "C",
            KeypadKey::D => "D",
        }
    }
}

pub struct CellState {
    pub board_id: u32,
    pub lock_id: u32,
    pub door_state: DoorState,
    pub cell_name: String,
    pub size: String,
    pub open_check_policy: OpenCheckPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCheckPolicy {
    None,
    Before,
    After,
    Always,
}

struct ScannerState {
    active: bool,
}

struct SimulatorInner {
    cells: Vec<CellState>,
    scanner: ScannerState,
    keypad_tx: Option<mpsc::UnboundedSender<KeypadKey>>,
    barcode_tx: Option<mpsc::UnboundedSender<String>>,
}

thread_local! {
    static STATE: RefCell<Option<SimulatorInner>> = const { RefCell::new(None) };
    // UI callbacks live independently of cell state so they survive
    // re-init when the active layout changes.
    static UI_REFRESH: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    static INVALID_LOCK_ALERT: RefCell<Option<Box<dyn Fn(u32, u32, &str)>>> =
        const { RefCell::new(None) };
    static DOOR_OPENED_NOTIFIER: RefCell<Option<Box<dyn Fn(u32, u32)>>> =
        const { RefCell::new(None) };
}

pub fn init_state(cells: Vec<CellState>) {
    STATE.with(|s| {
        *s.borrow_mut() = Some(SimulatorInner {
            cells,
            scanner: ScannerState { active: false },
            keypad_tx: None,
            barcode_tx: None,
        });
    });
}

pub fn set_ui_refresh(cb: Box<dyn Fn()>) {
    UI_REFRESH.with(|s| *s.borrow_mut() = Some(cb));
}

/// Register the UI's invalid-lock alert handler. Called once from
/// `ui::create_window`; left unset in unit tests.
pub fn set_invalid_lock_alert(cb: Box<dyn Fn(u32, u32, &str)>) {
    INVALID_LOCK_ALERT.with(|s| *s.borrow_mut() = Some(cb));
}

/// Register a callback fired whenever a door transitions to Open. The UI
/// uses this to auto-scroll the lockers panel to the freshly opened cell.
pub fn set_door_opened_notifier(cb: Box<dyn Fn(u32, u32)>) {
    DOOR_OPENED_NOTIFIER.with(|s| *s.borrow_mut() = Some(cb));
}

fn fire_door_opened(board_id: u32, lock_id: u32) {
    DOOR_OPENED_NOTIFIER.with(|s| {
        if let Some(cb) = s.borrow().as_ref() {
            cb(board_id, lock_id);
        }
    });
}

fn fire_invalid_lock_alert(board_id: u32, lock_id: u32, reason: &str) {
    INVALID_LOCK_ALERT.with(|s| {
        if let Some(cb) = s.borrow().as_ref() {
            cb(board_id, lock_id, reason);
        }
    });
}

fn request_ui_refresh() {
    UI_REFRESH.with(|s| {
        if let Some(cb) = s.borrow().as_ref() {
            cb();
        }
    });
}

/// Open a lock. Returns error if door is in Error state.
pub fn lock_open(board_id: u32, lock_id: u32) -> Result<(), Error> {
    if let Err(reason) = validate_board_lock(board_id, lock_id) {
        log::error!("simulator: rejected open ({}, {}) — {}", board_id, lock_id, reason);
        fire_invalid_lock_alert(board_id, lock_id, &reason);
        return Err(Error {
            code: -7,
            message: format!("lock {}:{} out of controller range: {}", board_id, lock_id, reason),
        });
    }

    STATE.with(|s| {
        let mut inner = s.borrow_mut();
        let inner = inner.as_mut().expect("simulator not initialized");
        let cell = inner.cells.iter_mut().find(|c| c.board_id == board_id && c.lock_id == lock_id);
        match cell {
            Some(cell) => {
                let before_state = cell.door_state;
                let policy = cell.open_check_policy;

                // "before": fail if already opened.
                if matches!(policy, OpenCheckPolicy::Before | OpenCheckPolicy::Always)
                    && before_state == DoorState::Open
                {
                    return Err(Error {
                        code: -5,
                        message: format!(
                            "lock {}:{} open check before failed: already opened",
                            board_id, lock_id
                        ),
                    });
                }

                // Simulate open attempt.
                cell.door_state = DoorState::Open;

                // "after": fail if pre-open state was not closed.
                if matches!(policy, OpenCheckPolicy::After | OpenCheckPolicy::Always)
                    && before_state != DoorState::Closed
                {
                    return Err(Error {
                        code: -5,
                        message: format!(
                            "lock {}:{} open check after failed: previous state was {:?}",
                            board_id, lock_id, before_state
                        ),
                    });
                }

                log::info!(
                    "simulator: lock {}:{} opened (policy={:?}, prev={:?})",
                    board_id,
                    lock_id,
                    policy,
                    before_state
                );
                Ok(())
            }
            None => Err(Error {
                code: -2,
                message: format!("lock {}:{} not found", board_id, lock_id),
            }),
        }
    })?;
    request_ui_refresh();
    fire_door_opened(board_id, lock_id);
    Ok(())
}

/// Get lock statuses indexed by lock id so callers can check `statuses[lock_id]`.
pub fn lock_statuses_get(board_id: u32) -> Result<Vec<LockStatus>, Error> {
    STATE.with(|s| {
        let inner = s.borrow();
        let inner = inner.as_ref().expect("simulator not initialized");

        let max_lock_id = inner
            .cells
            .iter()
            .filter(|cell| cell.board_id == board_id)
            .map(|cell| cell.lock_id)
            .max()
            .unwrap_or(0);

        let mut statuses = vec![LockStatus::Disabled; max_lock_id as usize + 1];

        for cell in &inner.cells {
            if cell.board_id != board_id {
                continue;
            }

            let status = match cell.door_state {
                DoorState::Closed => LockStatus::Closed,
                DoorState::Open => LockStatus::Opened,
                DoorState::Error => LockStatus::Disabled,
            };

            statuses[cell.lock_id as usize] = status;
        }

        Ok(statuses)
    })
}

/// Close a door (called from UI click).
pub fn door_close(board_id: u32, lock_id: u32) {
    STATE.with(|s| {
        let mut inner = s.borrow_mut();
        let inner = inner.as_mut().expect("simulator not initialized");
        if let Some(cell) = inner.cells.iter_mut().find(|c| c.board_id == board_id && c.lock_id == lock_id) {
            cell.door_state = DoorState::Closed;
            log::info!("simulator: door {}:{} closed", board_id, lock_id);
        }
    });
    request_ui_refresh();
}

/// Set door to error state (called from UI).
pub fn door_set_error(board_id: u32, lock_id: u32) {
    STATE.with(|s| {
        let mut inner = s.borrow_mut();
        let inner = inner.as_mut().expect("simulator not initialized");
        if let Some(cell) = inner.cells.iter_mut().find(|c| c.board_id == board_id && c.lock_id == lock_id) {
            cell.door_state = DoorState::Error;
            log::info!("simulator: door {}:{} set to error", board_id, lock_id);
        }
    });
    request_ui_refresh();
}

/// Get current door state for a cell.
pub fn get_door_state(board_id: u32, lock_id: u32) -> DoorState {
    STATE.with(|s| {
        let inner = s.borrow();
        let inner = inner.as_ref().expect("simulator not initialized");
        inner.cells.iter()
            .find(|c| c.board_id == board_id && c.lock_id == lock_id)
            .map(|c| c.door_state)
            .unwrap_or(DoorState::Error)
    })
}

/// Cycle door state: Open → Closed, Closed → Error, Error → Closed.
pub fn door_cycle_state(board_id: u32, lock_id: u32) {
    STATE.with(|s| {
        let mut inner = s.borrow_mut();
        let inner = inner.as_mut().expect("simulator not initialized");
        if let Some(cell) = inner.cells.iter_mut().find(|c| c.board_id == board_id && c.lock_id == lock_id) {
            cell.door_state = match cell.door_state {
                DoorState::Open => DoorState::Closed,
                DoorState::Closed => DoorState::Error,
                DoorState::Error => DoorState::Closed,
            };
            log::info!("simulator: door {}:{} → {:?}", board_id, lock_id, cell.door_state);
        }
    });
    request_ui_refresh();
}

// --- Barcode scanner ---

pub fn scanner_start() {
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.scanner.active = true;
            log::info!("simulator: barcode scanner started");
        }
    });
    request_ui_refresh();
}

pub fn scanner_stop() {
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.scanner.active = false;
            log::info!("simulator: barcode scanner stopped");
        }
    });
    request_ui_refresh();
}

pub fn is_scanner_active() -> bool {
    STATE.with(|s| {
        s.borrow().as_ref()
            .map(|inner| inner.scanner.active)
            .unwrap_or(false)
    })
}

pub fn set_barcode_sender(tx: mpsc::UnboundedSender<String>) {
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.barcode_tx = Some(tx);
        }
    });
}

pub fn barcode_unsubscribe() {
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.barcode_tx = None;
        }
    });
}

/// Submit a scanned barcode (called from UI). Only works if scanner is active.
pub fn submit_barcode(barcode: String) {
    STATE.with(|s| {
        let inner = s.borrow();
        let inner = inner.as_ref().expect("simulator not initialized");
        if !inner.scanner.active {
            log::warn!("simulator: barcode submitted but scanner not active");
            return;
        }
        if let Some(tx) = inner.barcode_tx.as_ref() {
            let result: Result<(), _> = tx.unbounded_send(barcode.clone());
            if result.is_err() {
                log::warn!("simulator: barcode channel closed");
            } else {
                log::info!("simulator: barcode scanned: {}", barcode);
            }
        } else {
            log::warn!("simulator: no barcode subscriber");
        }
    });
}

// --- Keypad ---

pub fn keypad_subscribe() -> mpsc::UnboundedReceiver<KeypadKey> {
    let (tx, rx) = mpsc::unbounded();
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.keypad_tx = Some(tx);
        }
    });
    rx
}

pub fn keypad_unsubscribe() {
    STATE.with(|s| {
        if let Some(inner) = s.borrow_mut().as_mut() {
            inner.keypad_tx = None;
        }
    });
}

/// Send a keypad press (called from UI).
pub fn keypad_press(key: KeypadKey) {
    STATE.with(|s| {
        let inner = s.borrow();
        let inner = inner.as_ref().expect("simulator not initialized");
        if let Some(tx) = inner.keypad_tx.as_ref() {
            let result: Result<(), _> = tx.unbounded_send(key);
            if result.is_err() {
                log::warn!("simulator: keypad channel closed");
            } else {
                log::info!("simulator: keypad press: {}", key.label());
            }
        }
    });
}

/// Get a snapshot of all cells (for UI rendering).
pub fn get_cells_snapshot() -> Vec<(u32, u32, DoorState, String, String)> {
    STATE.with(|s| {
        let inner = s.borrow();
        let inner = inner.as_ref().expect("simulator not initialized");
        inner.cells.iter()
            .map(|c| (c.board_id, c.lock_id, c.door_state, c.cell_name.clone(), c.size.clone()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_with(cells: Vec<CellState>) {
        init_state(cells);
    }

    fn cell(board_id: u32, lock_id: u32) -> CellState {
        CellState {
            board_id,
            lock_id,
            door_state: DoorState::Closed,
            cell_name: format!("{}-{}", board_id, lock_id),
            size: "M".into(),
            open_check_policy: OpenCheckPolicy::Always,
        }
    }

    #[test]
    fn lock_open_rejects_out_of_range_board_lock() {
        init_with(vec![cell(1, 1)]);
        // board 0 only allows lock 1..=3
        let err = lock_open(0, 5).unwrap_err();
        assert_eq!(err.code, -7);
        // board 1..=10 only allows lock 1..=24
        let err = lock_open(2, 25).unwrap_err();
        assert_eq!(err.code, -7);
        // board 11 invalid
        let err = lock_open(11, 1).unwrap_err();
        assert_eq!(err.code, -7);
    }

    #[test]
    fn lock_open_in_range_still_returns_not_found_when_cell_missing() {
        init_with(vec![]);
        let err = lock_open(1, 1).unwrap_err();
        // Range is valid but no such cell — falls through to the original behavior.
        assert_eq!(err.code, -2);
    }
}
