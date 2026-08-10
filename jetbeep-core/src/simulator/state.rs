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
    // Configurable "physical world" latency applied before servicing
    // lock_open/lock_statuses_get, so the simulator behaves like real
    // hardware (which does not answer these calls instantly). Independent
    // of `STATE` so it survives layout re-inits.
    static PHYSICAL_TIMING: RefCell<PhysicalTiming> =
        RefCell::new(PhysicalTiming::default());
    // Configurable network-connectivity simulation applied before servicing
    // outbound `server_request` calls. Independent of `PHYSICAL_TIMING` (which
    // models lock hardware latency) and of `STATE` so it survives layout
    // re-inits.
    static NETWORK_SIM: RefCell<NetworkSim> = RefCell::new(NetworkSim::default());
    // Tiny xorshift PRNG state for jitter + failure rolls (no `rand` dep in
    // scope). Seeded lazily from a monotonic counter on first use.
    static NET_RNG: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Simulated hardware latency for door-open and cell-door-status bus calls.
/// Defaults mirror the timing of real locker hardware; both are adjustable
/// at runtime from the app's Settings screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalTiming {
    /// Delay before `lock_open` resolves (ms). Default: 1200.
    pub door_open_ms: u32,
    /// Delay before `lock_statuses_get` resolves (ms). Default: 300.
    pub cell_status_ms: u32,
}

impl Default for PhysicalTiming {
    fn default() -> Self {
        Self {
            door_open_ms: 1200,
            cell_status_ms: 300,
        }
    }
}

/// Returns the current simulated physical-world timing.
pub fn get_physical_timing() -> PhysicalTiming {
    PHYSICAL_TIMING.with(|t| *t.borrow())
}

/// Updates the simulated physical-world timing (e.g. from the app's
/// Settings screen). Takes effect on the next `lock_open` /
/// `lock_statuses_get` call.
pub fn set_physical_timing(timing: PhysicalTiming) {
    PHYSICAL_TIMING.with(|t| *t.borrow_mut() = timing);
}

/// Base network latency profile applied to outbound `server_request` calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// No added latency.
    Normal,
    /// Moderate mobile latency (~600 ms ± 400).
    Mobile,
    /// Heavy 2G-style latency (~2500 ms ± 1000).
    Slow,
    /// Every request fails immediately with a connectivity error.
    Offline,
}

impl NetworkMode {
    /// All modes in dropdown selection-index order. Single source of truth for
    /// the UI options and the index round-trip below.
    pub const ALL: [NetworkMode; 4] = [
        NetworkMode::Normal,
        NetworkMode::Mobile,
        NetworkMode::Slow,
        NetworkMode::Offline,
    ];

    /// Human-readable label used by the simulator UI dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            NetworkMode::Normal => "Normal",
            NetworkMode::Mobile => "Mobile",
            NetworkMode::Slow => "Slow / 2G",
            NetworkMode::Offline => "Offline",
        }
    }

    /// UI dropdown options, newline-separated, in selection-index order.
    /// Derived from [`NetworkMode::ALL`]/[`NetworkMode::label`] so the labels
    /// stay the single source of truth.
    pub fn ui_options() -> String {
        Self::ALL
            .iter()
            .map(|m| m.label())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Map a dropdown selection index back to a mode. Order matches
    /// [`NetworkMode::ALL`]; out-of-range indices fall back to `Normal`.
    pub fn from_index(idx: u32) -> Self {
        Self::ALL
            .get(idx as usize)
            .copied()
            .unwrap_or(NetworkMode::Normal)
    }

    /// Dropdown selection index for this mode.
    pub fn to_index(self) -> u32 {
        Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as u32
    }

    /// Base delay (ms) and jitter amplitude (± ms) for this mode.
    fn base_delay(self) -> (u32, u32) {
        match self {
            NetworkMode::Normal => (0, 0),
            NetworkMode::Mobile => (600, 400),
            NetworkMode::Slow => (2500, 1000),
            NetworkMode::Offline => (0, 0),
        }
    }
}

/// Which error a simulated failed `server_request` returns, so the app's
/// error-handling paths can be exercised independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Timeout,
    ServerError,
    BadRequest,
}

impl FailureKind {
    /// All kinds in dropdown selection-index order. Single source of truth for
    /// the UI options and the index round-trip below.
    pub const ALL: [FailureKind; 3] = [
        FailureKind::Timeout,
        FailureKind::ServerError,
        FailureKind::BadRequest,
    ];

    /// Human-readable label used by the simulator UI dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            FailureKind::Timeout => "Timeout",
            FailureKind::ServerError => "Server error (5xx)",
            FailureKind::BadRequest => "Bad request (4xx)",
        }
    }

    /// UI dropdown options, newline-separated, in selection-index order.
    /// Derived from [`FailureKind::ALL`]/[`FailureKind::label`] so the labels
    /// stay the single source of truth.
    pub fn ui_options() -> String {
        Self::ALL
            .iter()
            .map(|k| k.label())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Map a dropdown selection index back to a kind. Order matches
    /// [`FailureKind::ALL`]; out-of-range indices fall back to `Timeout`.
    pub fn from_index(idx: u32) -> Self {
        Self::ALL
            .get(idx as usize)
            .copied()
            .unwrap_or(FailureKind::Timeout)
    }

    /// Dropdown selection index for this kind.
    pub fn to_index(self) -> u32 {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0) as u32
    }

    /// The `Error` this failure kind produces.
    fn error(self) -> Error {
        match self {
            FailureKind::Timeout => Error {
                code: -2,
                message: "network timeout (simulated)".to_string(),
            },
            FailureKind::ServerError => Error {
                code: -1,
                message: "server error 500 (simulated)".to_string(),
            },
            FailureKind::BadRequest => Error {
                code: -1,
                message: "bad request 400 (simulated)".to_string(),
            },
        }
    }
}

/// Maximum configurable extra latency (ms) for the network simulation.
pub const NETWORK_EXTRA_LATENCY_MAX: u32 = 10_000;

/// Configurable network-connectivity simulation for outbound `server_request`
/// calls. Independent of [`PhysicalTiming`], which models lock hardware
/// latency. Adjustable at runtime from the simulator's Settings modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkSim {
    /// Base latency profile (and offline short-circuit).
    pub mode: NetworkMode,
    /// Percentage of requests to randomly fail (0..=100). Ignored when
    /// `mode` is `Offline` (which fails everything).
    pub failure_rate: u32,
    /// Which error a randomly failed request returns.
    pub failure_kind: FailureKind,
    /// Fixed delay (ms) added on top of the mode latency.
    pub extra_latency_ms: u32,
}

impl Default for NetworkSim {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Normal,
            failure_rate: 0,
            failure_kind: FailureKind::Timeout,
            extra_latency_ms: 0,
        }
    }
}

/// Returns the current network-connectivity simulation config.
pub fn get_network_sim() -> NetworkSim {
    NETWORK_SIM.with(|n| *n.borrow())
}

/// Updates the network-connectivity simulation config (e.g. from the
/// simulator's Settings modal). Takes effect on the next `server_request`.
pub fn set_network_sim(sim: NetworkSim) {
    NETWORK_SIM.with(|n| *n.borrow_mut() = sim);
}

/// Advance the thread-local xorshift PRNG and return the next `u32`.
/// Seeded lazily from a monotonic nanosecond counter on first use.
fn net_rng_next() -> u32 {
    NET_RNG.with(|cell| {
        let mut x = cell.get();
        if x == 0 {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0x9E37_79B9);
            x = seed | 1;
        }
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        cell.set(x);
        x
    })
}

/// Compute the outcome of the network simulation for one `server_request`.
/// Returns the delay to apply (ms) and, if the request should fail, the
/// `Error` to return instead of performing it.
pub fn network_sim_outcome() -> (u32, Option<Error>) {
    let sim = get_network_sim();

    if sim.mode == NetworkMode::Offline {
        return (
            0,
            Some(Error {
                code: -1,
                message: "network offline (simulated)".to_string(),
            }),
        );
    }

    // Base mode delay with symmetric jitter, plus the fixed extra latency.
    let (base, jitter) = sim.mode.base_delay();
    let delay = if jitter > 0 {
        let span = jitter * 2 + 1;
        let offset = (net_rng_next() % span) as i64 - jitter as i64;
        (base as i64 + offset).max(0) as u32
    } else {
        base
    }
    .saturating_add(sim.extra_latency_ms);

    // Independent fault injection.
    let fail = if sim.failure_rate == 0 {
        false
    } else if sim.failure_rate >= 100 {
        true
    } else {
        (net_rng_next() % 100) < sim.failure_rate
    };

    let err = if fail { Some(sim.failure_kind.error()) } else { None };
    (delay, err)
}

pub fn init_state(cells: Vec<CellState>) {
    STATE.with(|s| {
        let mut slot = s.borrow_mut();
        // Preserve live peripheral connections across a layout re-init: the
        // barcode/keypad senders and the scanner power state belong to the
        // running app (subscribed once in `app_main`), not to the locker
        // layout. Only the cell list changes when the active layout switches.
        // This mirrors the UI callbacks, which live in separate thread-locals
        // for the same reason.
        let (scanner, keypad_tx, barcode_tx) = match slot.take() {
            Some(prev) => (prev.scanner, prev.keypad_tx, prev.barcode_tx),
            None => (ScannerState { active: false }, None, None),
        };
        *slot = Some(SimulatorInner {
            cells,
            scanner,
            keypad_tx,
            barcode_tx,
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

    #[test]
    fn network_offline_always_fails_with_no_delay() {
        set_network_sim(NetworkSim {
            mode: NetworkMode::Offline,
            failure_rate: 0,
            failure_kind: FailureKind::Timeout,
            extra_latency_ms: 5000,
        });
        for _ in 0..50 {
            let (delay, err) = network_sim_outcome();
            assert_eq!(delay, 0);
            assert!(err.is_some());
        }
        set_network_sim(NetworkSim::default());
    }

    #[test]
    fn network_failure_rate_bounds() {
        // 0% never fails.
        set_network_sim(NetworkSim {
            mode: NetworkMode::Normal,
            failure_rate: 0,
            failure_kind: FailureKind::ServerError,
            extra_latency_ms: 0,
        });
        for _ in 0..100 {
            assert!(network_sim_outcome().1.is_none());
        }
        // 100% always fails.
        set_network_sim(NetworkSim {
            mode: NetworkMode::Normal,
            failure_rate: 100,
            failure_kind: FailureKind::ServerError,
            extra_latency_ms: 0,
        });
        for _ in 0..100 {
            assert!(network_sim_outcome().1.is_some());
        }
        set_network_sim(NetworkSim::default());
    }

    #[test]
    fn network_normal_delay_is_extra_latency_only() {
        set_network_sim(NetworkSim {
            mode: NetworkMode::Normal,
            failure_rate: 0,
            failure_kind: FailureKind::Timeout,
            extra_latency_ms: 750,
        });
        let (delay, err) = network_sim_outcome();
        assert_eq!(delay, 750);
        assert!(err.is_none());
        set_network_sim(NetworkSim::default());
    }

    #[test]
    fn network_failure_kind_maps_to_error_code() {
        for (kind, code) in [
            (FailureKind::Timeout, -2),
            (FailureKind::ServerError, -1),
            (FailureKind::BadRequest, -1),
        ] {
            set_network_sim(NetworkSim {
                mode: NetworkMode::Normal,
                failure_rate: 100,
                failure_kind: kind,
                extra_latency_ms: 0,
            });
            let err = network_sim_outcome().1.expect("should fail at 100%");
            assert_eq!(err.code, code, "kind {:?}", kind);
        }
        set_network_sim(NetworkSim::default());
    }

    #[test]
    fn network_mode_index_roundtrip() {
        for mode in [
            NetworkMode::Normal,
            NetworkMode::Mobile,
            NetworkMode::Slow,
            NetworkMode::Offline,
        ] {
            assert_eq!(NetworkMode::from_index(mode.to_index()), mode);
        }
        for kind in [
            FailureKind::Timeout,
            FailureKind::ServerError,
            FailureKind::BadRequest,
        ] {
            assert_eq!(FailureKind::from_index(kind.to_index()), kind);
        }
    }
}
