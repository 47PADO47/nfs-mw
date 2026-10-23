//! The installed-game tests take turns. Several open the real world and pace a real-time clock (the net threads of a
//! loopback session run in real time), so side by side they starve each other and every datagram looks late.

use std::sync::{Mutex, MutexGuard, PoisonError};

static TURN: Mutex<()> = Mutex::new(());

/// Waits for the other installed tests; hold the guard for the whole test.
pub(super) fn take() -> MutexGuard<'static, ()> {
    TURN.lock().unwrap_or_else(PoisonError::into_inner)
}
