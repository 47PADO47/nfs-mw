//! The gates a bar must pass to be drawn. Spec section 10. The caller reads the three flags from the world.

/// The flags that gate the bars. Each one is a plain boolean read by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gates {
    /// A chase is on (`TrafficWorld::is_pursuing()`).
    pub pursuing: bool,
    /// A pursuit shot (the catch pan or the arrest framing) is running.
    pub shot_running: bool,
    /// The HUD is shown, after the look-back rule.
    pub hud_visible: bool,
}

/// True only when a chase is on, no shot is running and the HUD is shown.
pub fn shows(gates: Gates) -> bool {
    gates.pursuing && !gates.shot_running && gates.hud_visible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_all_open_row_draws() {
        for bits in 0u8..8 {
            let gates = Gates { pursuing: bits & 1 != 0, shot_running: bits & 2 != 0, hud_visible: bits & 4 != 0 };
            let expected = gates.pursuing && !gates.shot_running && gates.hud_visible;
            assert_eq!(shows(gates), expected, "{gates:?}");
        }
        assert!(shows(Gates { pursuing: true, shot_running: false, hud_visible: true }));
    }

    #[test]
    fn no_pursuit_means_no_bars() {
        assert!(!shows(Gates { pursuing: false, shot_running: false, hud_visible: true }));
    }
}
