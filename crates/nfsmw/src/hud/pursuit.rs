//! The pursuit readout the HUD shows during a chase: the pursuit board's timer and the cool-down (evade) bar. What
//! each part shows and where it comes from is in `docs/specs/hud-pursuit.md`.

/// The chase as the HUD shows it. `None` in [`HudState`](super::HudState) means no pursuit.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PursuitHud {
    /// Seconds since the chase began.
    pub secs: f32,
    /// How far the player is from getting away, 0 at the start and 1 when the cops give up.
    pub evade: f32,
    /// The cop cars wrecked so far in the chase: the counter beside the board's cops icon.
    pub cops_destroyed: u32,
    /// Original signed display value: +1 busted, 0 neutral, -1 cooldown.
    pub busted: f32,
    /// The pursuit is in its actual cooldown phase, independently of the fill fraction.
    pub cooldown: bool,
}

impl PursuitHud {
    /// The chase from the world's numbers: `secs_to_evade` is the unseen time still to go, out of the evade
    /// `timeout` of the heat level. The evaded share is what is already done of that timeout. No cop is counted yet.
    pub fn from_evade_time(secs: f32, secs_to_evade: f32, timeout: f32) -> Self {
        let secs = secs.max(0.0);
        if timeout <= 0.0 {
            return Self { secs, ..Default::default() };
        }
        Self { secs, evade: (1.0 - secs_to_evade / timeout).clamp(0.0, 1.0), ..Default::default() }
    }

    /// Whether the cool-down bar is on the screen.
    pub fn cooldown_shown(&self) -> bool {
        self.cooldown
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
    fn cooldown_is_an_explicit_phase_instead_of_an_inferred_fill_threshold() {
        assert!(!PursuitHud { evade: 0.5, ..Default::default() }.cooldown_shown());
        assert!(PursuitHud { evade: 0.0, cooldown: true, ..Default::default() }.cooldown_shown());
    }

    #[test]
    fn the_timer_reads_minutes_seconds_and_hundredths() {
        assert_eq!(timer_text(0.0), "00:00.00");
        assert_eq!(timer_text(12.345), "00:12.34");
        assert_eq!(timer_text(75.5), "01:15.50");
        assert_eq!(timer_text(-1.0), "00:00.00");
    }
}
