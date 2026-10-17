//! The numbers of the cop health bars. [`BarParams::default`] gives the values of `docs/specs/cop-health-bars.md`
//! (sections 5, 6, 7 and 9). A caller that wants other numbers builds its own value; the rules take it as an input.

/// Size, placement, timing and distance fade of the bars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarParams {
    /// Bar length in metres (`BAR_WIDTH`).
    pub width: f32,
    /// Bar height in metres (`BAR_HEIGHT`).
    pub height: f32,
    /// Lift above the cop's position along world up, in metres (`BAR_LIFT`).
    pub lift: f32,
    /// Opacity of every colour (`BAR_ALPHA`).
    pub alpha: f32,
    /// Border thickness in metres (`BORDER_WIDTH`).
    pub border: f32,
    /// Simulated seconds the lagging segment holds after a new drop (`LAG_HOLD_SECS`).
    pub lag_hold_secs: f32,
    /// Fraction of a full bar the lagging segment falls per second (`LAG_DRAIN_RATE`).
    pub lag_drain_rate: f32,
    /// Seconds between two flash switches (`FLASH_PERIOD_SECS`).
    pub flash_period_secs: f32,
    /// Seconds after death when the fade starts (`DEATH_FADE_START_SECS`).
    pub fade_start_secs: f32,
    /// Seconds after death when the bar is gone (`DISABLED_HIDE_SECS`).
    pub hide_secs: f32,
    /// Distance where the near fade starts, alpha 0 (`NEAR_FADE_MIN`), in metres.
    pub near_fade_min: f32,
    /// Distance where the near fade is complete, alpha 1 (`NEAR_FADE_MAX`), in metres.
    pub near_fade_max: f32,
    /// Distance where the far fade starts, alpha 1 (`FAR_FADE_MIN`), in metres.
    pub far_fade_min: f32,
    /// Distance where the far fade is complete, alpha 0 (`FAR_FADE_MAX`), in metres.
    pub far_fade_max: f32,
}

impl Default for BarParams {
    fn default() -> Self {
        Self {
            width: 1.4,
            height: 0.13,
            lift: 1.15,
            alpha: 0.7,
            border: 0.02,
            lag_hold_secs: 0.5,
            lag_drain_rate: 0.75,
            flash_period_secs: 0.05,
            fade_start_secs: 1.0,
            hide_secs: 2.0,
            near_fade_min: 2.0,
            near_fade_max: 5.0,
            far_fade_min: 70.0,
            far_fade_max: 110.0,
        }
    }
}
