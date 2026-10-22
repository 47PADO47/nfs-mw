//! How many players share the career right now: one for a single player, two or more in co-op
//! (`docs/specs/coop-career.md` section 4). The front end sets it when co-op starts and ends (`frontend/flow_coop.rs`), and
//! the car lot reads it to pick the garage limits.

use std::sync::atomic::{AtomicUsize, Ordering};

/// The players sharing the career. One until co-op sets more; never below one.
static PLAYERS: AtomicUsize = AtomicUsize::new(1);

/// The players sharing the career right now, never below one.
pub fn coop_players() -> usize {
    PLAYERS.load(Ordering::Relaxed).max(1)
}

/// Sets the players sharing the career. Zero is stored as one.
pub fn set_coop_players(n: usize) {
    PLAYERS.store(n.max(1), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coop_players_never_drop_below_one() {
        let before = coop_players();
        set_coop_players(0);
        assert_eq!(coop_players(), 1, "zero players is stored as one");
        set_coop_players(2);
        assert_eq!(coop_players(), 2);
        set_coop_players(before);
    }
}
