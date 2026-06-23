use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpenCheckPolicy {
    None,
    Before,
    After,
    Always,
}

impl Default for OpenCheckPolicy {
    fn default() -> Self {
        Self::Always
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LockerConfig {
    pub width: u32,
    pub depth: u32,
    #[serde(default)]
    pub open_check_policy: OpenCheckPolicy,
    #[serde(default)]
    pub cells: Option<Vec<CellConfig>>,
    #[serde(default)]
    pub columns: Option<Vec<LockerColumn>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LockerColumn {
    pub width: u32,
    pub cells: Vec<CellConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CellConfig {
    pub cell_name: Option<String>,
    pub height: u32,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub board_id: Option<u32>,
    #[serde(default)]
    pub lock_id: Option<u32>,
    #[serde(default)]
    pub pinpad: bool,
    #[serde(default)]
    pub depth: Option<u32>,
    /// If present, this row is split into side-by-side columns
    #[serde(default)]
    pub columns: Option<Vec<ColumnConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ColumnConfig {
    pub cell_name: String,
    pub width: u32,
    pub size: String,
    pub board_id: u32,
    pub lock_id: u32,
}

/// Validate a (board_id, lock_id) pair against real lock-controller hardware limits.
///
/// Rules:
/// - board 0 → lock 1..=3
/// - boards 1..=10 → lock 1..=24
/// - any other board → invalid
pub fn validate_board_lock(board_id: u32, lock_id: u32) -> Result<(), String> {
    match board_id {
        0 if (1..=3).contains(&lock_id) => Ok(()),
        1..=10 if (1..=24).contains(&lock_id) => Ok(()),
        _ => Err(format!(
            "invalid (board={}, lock={}): board 0 allows lock 1..=3, boards 1..=10 allow lock 1..=24",
            board_id, lock_id
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_0_accepts_lock_1_through_3() {
        assert!(validate_board_lock(0, 1).is_ok());
        assert!(validate_board_lock(0, 2).is_ok());
        assert!(validate_board_lock(0, 3).is_ok());
    }

    #[test]
    fn board_0_rejects_lock_0_and_above_3() {
        assert!(validate_board_lock(0, 0).is_err());
        assert!(validate_board_lock(0, 4).is_err());
        assert!(validate_board_lock(0, 24).is_err());
    }

    #[test]
    fn boards_1_through_10_accept_lock_1_through_24() {
        for board in 1..=10u32 {
            assert!(validate_board_lock(board, 1).is_ok(), "board={} lock=1", board);
            assert!(validate_board_lock(board, 12).is_ok(), "board={} lock=12", board);
            assert!(validate_board_lock(board, 24).is_ok(), "board={} lock=24", board);
        }
    }

    #[test]
    fn boards_1_through_10_reject_lock_0_and_above_24() {
        for board in 1..=10u32 {
            assert!(validate_board_lock(board, 0).is_err());
            assert!(validate_board_lock(board, 25).is_err());
        }
    }

    #[test]
    fn board_11_and_above_rejected_for_any_lock() {
        assert!(validate_board_lock(11, 1).is_err());
        assert!(validate_board_lock(99, 5).is_err());
    }
}
