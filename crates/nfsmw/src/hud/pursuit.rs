//! The pursuit readout the HUD shows during a chase: the pursuit board's timer and the cool-down (evade) bar. What
//! each part shows and where it comes from is in `docs/specs/hud-pursuit.md`.

/// The cool-down bar shows from this much evaded, 0 to 1 (the spec's `evade_level >= 0.05`, decomp).
pub const COOLDOWN_SHOWN_FROM: f32 = 0.05;

/// The chase as the HUD shows it. `None` in [`HudState`](super::HudState) means no pursuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitHud {
    /// Seconds since the chase began.
    pub secs: f32,
    /// How far the player is from getting away, 0 at the start and 1 when the cops give up.
    pub evade: f32,
}

impl PursuitHud {
    /// The chase from the world's numbers: `secs_to_evade` is the unseen time still to go, out of the evade
    /// `timeout` of the heat level. The evaded share is what is already done of that timeout.
    pub fn from_evade_time(secs: f32, secs_to_evade: f32, timeout: f32) -> Self {
        let secs = secs.max(0.0);
        if timeout <= 0.0 {
            return Self { secs, evade: 0.0 };
        }
        Self { secs, evade: (1.0 - secs_to_evade / timeout).clamp(0.0, 1.0) }
    }

    /// Whether the cool-down bar is on the screen.
    pub fn cooldown_shown(&self) -> bool {
        self.evade >= COOLDOWN_SHOWN_FROM
    }
}

/// The pursuit board's timer, `mm:ss.cc` (the package's text is `00:00.00`).
pub fn timer_text(secs: f32) -> String {
    let hundredths = (secs.max(0.0) * 100.0) as u64;
    format!("{:02}:{:02}.{:02}", hundredths / 6000, (hundredths / 100) % 60, hundredths % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_evade_bar_maps_the_share_of_the_timeout_done() {
        // A 10 s timeout with 10 s left is nothing done; with 2.5 s left a quarter is done (below 0.05 it is hidden).
        assert_eq!(PursuitHud::from_evade_time(1.0, 10.0, 10.0).evade, 0.0);
        assert_eq!(PursuitHud::from_evade_time(1.0, 7.5, 10.0).evade, 0.25);
        assert_eq!(PursuitHud::from_evade_time(1.0, 0.0, 10.0).evade, 1.0);
        // Past the timeout or with no timeout the share stays in 0..1.
        assert_eq!(PursuitHud::from_evade_time(1.0, -3.0, 10.0).evade, 1.0);
        assert_eq!(PursuitHud::from_evade_time(1.0, 4.0, 0.0).evade, 0.0);
    }

    #[test]
    fn the_cooldown_bar_shows_from_five_per_cent_evaded() {
        assert!(!PursuitHud { secs: 0.0, evade: 0.04 }.cooldown_shown());
        assert!(PursuitHud { secs: 0.0, evade: 0.05 }.cooldown_shown());
    }

    #[test]
    fn the_timer_reads_minutes_seconds_and_hundredths() {
        assert_eq!(timer_text(0.0), "00:00.00");
        assert_eq!(timer_text(12.345), "00:12.34");
        assert_eq!(timer_text(75.5), "01:15.50");
        assert_eq!(timer_text(-1.0), "00:00.00");
    }
}
