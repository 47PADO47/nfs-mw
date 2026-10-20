//! The catch-up (rubber band) controller (`docs/specs/ai-racers-catchup.md`).

/// Samples kept in the error history, one per second.
pub const HISTORY: usize = 10;

/// The gains of the controller. **[unconfirmed]** (the original body was not read).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatchUpGains {
    /// Skill per metre of error: `CatchUpSkill / CatchUpSpread * 100`.
    pub proportional: f32,
    /// `CatchUpIntegral`.
    pub integral: f32,
    /// `CatchUpDerivative`.
    pub derivative: f32,
    /// The integral term is clamped to this magnitude.
    pub integral_limit: f32,
}

impl CatchUpGains {
    /// The gains from a race's fields: `skill` and `spread` as `CatchUpSkill` and `CatchUpSpread`.
    pub fn from_race(skill: f32, spread: f32, integral: f32, derivative: f32) -> Self {
        let proportional = if spread > 0.0 { skill / spread * 100.0 } else { 0.0 };
        Self { proportional, integral, derivative, integral_limit: 20_000.0 }
    }
}

impl Default for CatchUpGains {
    /// The `basicrace` template values: skill 0.2, spread 1000, integral 5e-5, derivative 0.001.
    fn default() -> Self {
        Self::from_race(0.2, 1000.0, 5e-5, 0.001)
    }
}

/// One racer's catch-up state.
#[derive(Debug, Clone, PartialEq)]
pub struct CatchUp {
    gains: CatchUpGains,
    history: [f32; HISTORY],
    filled: usize,
    integral: f32,
    previous: Option<f32>,
    skill: f32,
}

impl CatchUp {
    pub fn new(gains: CatchUpGains) -> Self {
        Self { gains, history: [0.0; HISTORY], filled: 0, integral: 0.0, previous: None, skill: 0.0 }
    }

    /// The glue error of a racer, metres: positive when the racer is behind the player.
    /// `route_length / 100 * (player_percent - own_percent) * performance_ratio`
    /// (`docs/specs/ai-driver-speed-skill.md` section 8.2).
    pub fn error(route_length: f32, player_percent: f32, own_percent: f32, player_top_speed_rating: f32) -> f32 {
        let ratio = 1.0 + (1.0 - player_top_speed_rating) * 0.5;
        route_length / 100.0 * (player_percent - own_percent) * ratio
    }

    /// The signed skill offset now (the last value [`CatchUp::update`] gave).
    pub fn skill(&self) -> f32 {
        self.skill
    }

    /// The mean of the error history.
    pub fn mean_error(&self) -> f32 {
        if self.filled == 0 {
            return 0.0;
        }
        self.history[..self.filled].iter().sum::<f32>() / self.filled as f32
    }

    /// Feeds one error sample taken `dt` seconds after the last (nominally one second) and returns the new skill
    /// offset in -1..1.
    pub fn update(&mut self, error: f32, dt: f32) -> f32 {
        let dt = dt.max(1e-3);
        self.history.rotate_right(1);
        self.history[0] = error;
        self.filled = (self.filled + 1).min(HISTORY);
        let limit = self.gains.integral_limit;
        self.integral = (self.integral + self.mean_error() * dt).clamp(-limit, limit);
        let derivative = self.previous.map_or(0.0, |p| (error - p) / dt);
        self.previous = Some(error);
        let g = self.gains;
        self.skill = (g.proportional * error + g.integral * self.integral + g.derivative * derivative).clamp(-1.0, 1.0);
        self.skill
    }

    /// Clears the history (a restart, or a racer that is not glued for a while).
    pub fn reset(&mut self) {
        *self = Self::new(self.gains);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_is_positive_behind_the_player() {
        let behind = CatchUp::error(5000.0, 60.0, 50.0, 1.0);
        assert!((behind - 500.0).abs() < 1e-3, "{behind}");
        let ahead = CatchUp::error(5000.0, 50.0, 60.0, 1.0);
        assert!(ahead < 0.0);
    }

    #[test]
    fn a_weaker_player_car_widens_the_error() {
        let strong = CatchUp::error(1000.0, 10.0, 0.0, 1.0);
        let weak = CatchUp::error(1000.0, 10.0, 0.0, 0.0);
        assert!(weak > strong);
    }

    #[test]
    fn a_racer_behind_gets_a_positive_skill_and_one_ahead_a_negative_one() {
        let mut behind = CatchUp::new(CatchUpGains::default());
        let mut ahead = CatchUp::new(CatchUpGains::default());
        let (b, a) = (behind.update(400.0, 1.0), ahead.update(-400.0, 1.0));
        assert!(b > 0.0 && a < 0.0, "{b} {a}");
        assert!((b + a).abs() < 1e-5, "symmetric on the first sample");
    }

    #[test]
    fn the_skill_stays_within_one() {
        let mut state = CatchUp::new(CatchUpGains::default());
        for _ in 0..50 {
            state.update(1.0e6, 1.0);
        }
        assert_eq!(state.skill(), 1.0);
    }

    #[test]
    fn a_zero_error_gives_no_offset() {
        let mut state = CatchUp::new(CatchUpGains::default());
        assert_eq!(state.update(0.0, 1.0), 0.0);
    }

    #[test]
    fn reset_clears_the_history() {
        let mut state = CatchUp::new(CatchUpGains::default());
        state.update(300.0, 1.0);
        state.reset();
        assert_eq!(state.mean_error(), 0.0);
        assert_eq!(state.skill(), 0.0);
    }

    #[test]
    fn a_zero_spread_turns_the_proportional_term_off() {
        assert_eq!(CatchUpGains::from_race(0.2, 0.0, 0.0, 0.0).proportional, 0.0);
    }
}
