//! The per-heat tables of the escape: how many seconds the cops must lose the player for, and how much a hiding spot
//! speeds that up. The state machine that uses them is in [`crate::cooldown`]. The timeout is per heat level
//! (`evadetimeout`, install-verified); time spent in a hiding spot counts faster (`HiddenZoneTimeMultiplier`).
//! Spec: `docs/specs/ai-pursuit.md` section 6, `docs/specs/ai-pursuit-heat.md` section 2.2.

/// Unseen seconds needed to evade, per heat level 1 to 10.
pub const TIMEOUT: [f32; 10] = [20.0, 45.0, 75.0, 90.0, 120.0, 120.0, 120.0, 220.0, 60.0, 220.0];

/// Progress multiplier inside a hiding spot, per heat level 1 to 10.
pub const HIDDEN_MULTIPLIER: [f32; 10] = [4.0, 5.0, 6.0, 7.0, 8.0, 8.0, 8.0, 4.0, 2.0, 4.0];

/// The unseen timeout of a heat level (clamped to 1..=10).
pub fn timeout(level: u32) -> f32 {
    TIMEOUT[(level as usize).clamp(1, TIMEOUT.len()) - 1]
}

/// The hiding multiplier of a heat level (clamped to 1..=10).
pub fn hidden_multiplier(level: u32) -> f32 {
    HIDDEN_MULTIPLIER[(level as usize).clamp(1, HIDDEN_MULTIPLIER.len()) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timeouts_are_the_install_values() {
        assert_eq!(timeout(1), 20.0);
        assert_eq!(timeout(10), 220.0);
        assert_eq!(timeout(0), 20.0, "levels clamp to 1");
    }

    #[test]
    fn the_hiding_multipliers_are_the_install_values() {
        assert_eq!(hidden_multiplier(1), 4.0);
        assert_eq!(hidden_multiplier(9), 2.0);
        assert_eq!(hidden_multiplier(11), 4.0, "levels clamp to 10");
    }
}
