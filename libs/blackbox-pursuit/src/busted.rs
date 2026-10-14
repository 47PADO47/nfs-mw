//! The busted meter as pure rules: a cop that is in sight, close and on the player's height fills the busted timer
//! while the player is slow, and the player is busted when the timer passes its timeout. Otherwise the timer drains.
//! Nothing here reads a world or a clock: the caller measures the nearest height-matched cop, the player's speed and
//! the frame time, and applies the outcome. The values come from the caller's [`BustTuning`].
//!
//! Spec: `docs/specs/ai-pursuit.md` sections 4.4 and 5.1-5.2 (decomp, rates as the spec states them).

/// The tuning of the busted timer. The caller fills it from the install's constants and the heat's table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BustTuning {
    /// Timer seconds past which the player is busted.
    pub timeout: f32,
    /// Horizontal reach (metres) of a height-matched cop while the player is not flashing.
    pub reach: f32,
    /// Horizontal reach (metres) while the player is flashing (the post-reset blink).
    pub flashing_reach: f32,
    /// Maximum height difference (metres) between a cop and the player for the cop to count.
    pub height_gate: f32,
    /// Timer seconds gained per second while the player is slow and in reach.
    pub fill_rate: f32,
    /// Timer seconds lost per second otherwise.
    pub drain_rate: f32,
    /// Multiplier on the fill rate while flashing.
    pub flashing_multiplier: f32,
}

/// What the caller measured this frame for the busted timer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BustInput {
    /// Horizontal distance (metres) to the nearest cop whose height matches the player's (see [`height_matches`]);
    /// infinite when there is none.
    pub cop_distance: f32,
    /// A cop can see the player.
    pub seen: bool,
    /// The player's speed, metres per second (the magnitude).
    pub player_speed: f32,
    /// The speed under which the player counts as slow, metres per second (the heat's table).
    pub slow_speed: f32,
    /// The player is flashing (the post-reset blink). The caller passes `false` until that state exists.
    pub flashing: bool,
}

/// Whether a cop's height difference from the player (metres, either sign) is within the gate. A cop on a bridge above
/// or a road below does not count.
pub fn height_matches(height_difference: f32, gate: f32) -> bool {
    height_difference.abs() < gate
}

/// The busted timer of one chase.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Busted {
    timer: f32,
    busted: bool,
}

impl Busted {
    /// The timer, seconds (0 at the start, past the timeout once busted).
    pub fn timer(&self) -> f32 {
        self.timer
    }

    /// Whether the player has been busted.
    pub fn is_busted(&self) -> bool {
        self.busted
    }

    /// The meter, 0 to 1: the timer's share of the timeout, and 1 once busted.
    pub fn meter(&self, tuning: &BustTuning) -> f32 {
        if self.busted {
            return 1.0;
        }
        (self.timer / tuning.timeout.max(f32::EPSILON)).clamp(0.0, 1.0)
    }

    /// Clears the timer and the busted flag: a new chase starts.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether the player is in the bust zone this frame: a cop in sight and within reach, and the player slow (or
    /// flashing, which counts as slow). `cop_distance` is the height-matched distance.
    pub fn in_zone(input: &BustInput, tuning: &BustTuning) -> bool {
        let reach = if input.flashing { tuning.flashing_reach } else { tuning.reach };
        let slow = input.flashing || input.player_speed < input.slow_speed;
        input.seen && slow && input.cop_distance < reach
    }

    /// Advances the timer by `dt` seconds. Returns `true` on the frame the player is busted (once per chase); a
    /// busted timer stays put and returns `false` after that. A bad `dt` does nothing.
    pub fn update(&mut self, dt: f32, input: &BustInput, tuning: &BustTuning) -> bool {
        if self.busted || !(dt > 0.0 && dt.is_finite()) {
            return false;
        }
        let rate = if Self::in_zone(input, tuning) {
            let multiplier = if input.flashing { tuning.flashing_multiplier } else { 1.0 };
            tuning.fill_rate * multiplier
        } else {
            -tuning.drain_rate
        };
        self.timer = (self.timer + rate * dt).max(0.0);
        if self.timer <= tuning.timeout {
            return false;
        }
        self.busted = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TUNING: BustTuning = BustTuning {
        timeout: 5.0,
        reach: 15.0,
        flashing_reach: 90.0,
        height_gate: 1.5,
        fill_rate: 1.0,
        drain_rate: 2.0,
        flashing_multiplier: 4.0,
    };

    fn input(cop_distance: f32, seen: bool, speed: f32) -> BustInput {
        BustInput { cop_distance, seen, player_speed: speed, slow_speed: 2.5, flashing: false }
    }

    #[test]
    fn a_close_level_slow_cop_fills_to_busted() {
        let mut bust = Busted::default();
        let close = input(10.0, true, 0.0);
        for _ in 0..9 {
            assert!(!bust.update(0.5, &close, &TUNING), "4.5 s is under the 5 s timeout");
        }
        assert!(!bust.update(0.5, &close, &TUNING), "exactly 5 s is not past the timeout");
        assert!(!bust.is_busted());
        assert!(bust.update(0.5, &close, &TUNING), "past 5 s the player is busted");
        assert!(bust.is_busted());
        assert!(!bust.update(0.5, &close, &TUNING), "the bust is reported once");
    }

    #[test]
    fn a_far_or_fast_player_drains_the_timer() {
        let mut bust = Busted::default();
        for _ in 0..4 {
            bust.update(0.5, &input(10.0, true, 0.0), &TUNING);
        }
        assert_eq!(bust.timer(), 2.0);
        bust.update(0.5, &input(40.0, true, 0.0), &TUNING);
        assert_eq!(bust.timer(), 1.0, "out of reach drains at 2 per second");
        bust.update(0.5, &input(10.0, true, 9.0), &TUNING);
        assert_eq!(bust.timer(), 0.0, "fast drains too, and the timer stops at zero");
        assert_eq!(bust.meter(&TUNING), 0.0);
    }

    #[test]
    fn a_cop_out_of_sight_does_not_fill() {
        let mut bust = Busted::default();
        bust.update(1.0, &input(10.0, false, 0.0), &TUNING);
        assert_eq!(bust.timer(), 0.0);
    }

    #[test]
    fn the_height_gate_blocks_a_cop_on_another_level() {
        assert!(height_matches(1.0, TUNING.height_gate));
        assert!(!height_matches(3.0, TUNING.height_gate), "a cop 3 m below does not count");
        assert!(!height_matches(-3.0, TUNING.height_gate), "either sign");
        // The caller passes an infinite height-matched distance when every cop is off the level.
        let mut bust = Busted::default();
        bust.update(1.0, &input(f32::INFINITY, true, 0.0), &TUNING);
        assert_eq!(bust.timer(), 0.0);
    }

    #[test]
    fn flashing_reaches_ninety_metres_and_fills_four_times_faster() {
        let flashing = BustInput { flashing: true, ..input(60.0, true, 20.0) };
        let mut bust = Busted::default();
        bust.update(0.5, &flashing, &TUNING);
        assert_eq!(bust.timer(), 2.0, "fill 1 per second times 4 while flashing");
        let mut plain = Busted::default();
        plain.update(0.5, &input(60.0, true, 0.0), &TUNING);
        assert_eq!(plain.timer(), 0.0, "60 m is out of the normal reach");
    }

    #[test]
    fn meter_is_the_timer_share_and_one_once_busted() {
        let mut bust = Busted::default();
        bust.update(1.25, &input(10.0, true, 0.0), &TUNING);
        assert_eq!(bust.meter(&TUNING), 0.25);
        bust.update(10.0, &input(10.0, true, 0.0), &TUNING);
        assert_eq!(bust.meter(&TUNING), 1.0);
    }

    #[test]
    fn reset_clears_a_bust() {
        let mut bust = Busted::default();
        bust.update(10.0, &input(10.0, true, 0.0), &TUNING);
        bust.reset();
        assert_eq!(bust, Busted::default());
    }
}
