//! Application generation counter used to soft-kill a running app.
//!
//! The platform entry point bumps the generation before tearing down the
//! current app. Work submitted to the main workq captures the generation at
//! submission time and is silently dropped when it becomes stale, so
//! asynchronous completions belonging to a killed app never run.

use core::sync::atomic::{AtomicU32, Ordering};

static APP_GENERATION: AtomicU32 = AtomicU32::new(0);

/// Current application generation.
pub fn current() -> u32 {
    APP_GENERATION.load(Ordering::Relaxed)
}

/// Invalidate all work belonging to the current generation and start a new one.
///
/// Returns the new generation.
pub fn bump() -> u32 {
    APP_GENERATION.fetch_add(1, Ordering::Relaxed) + 1
}
