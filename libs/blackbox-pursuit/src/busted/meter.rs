//! Signed proximity warning and arrest-timer blend. The caller supplies game tuning.

use super::{BustTuning, Busted};

#[derive(Debug, Clone, Copy)]
pub struct MeterTuning {
    pub bust_dead_zone: f32,
    pub evade_dead_zone: f32,
    pub front_sight: f32,
    pub warning_speed: f32,
    pub neutral_speed: f32,
    pub fresh_sighting: f32,
    pub sight_grace: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MeterInput {
    /// Three-dimensional distance to the nearest live pursuing or roadblock unit.
    pub distance: f32,
    pub speed: f32,
    pub slow_speed: f32,
    pub since_seen: f32,
    pub evade: f32,
}

fn proximity(input: MeterInput, bust: &BustTuning, tune: &MeterTuning) -> f32 {
    if input.since_seen > tune.sight_grace {
        return -1.0;
    }
    if input.since_seen > tune.fresh_sighting {
        return (-0.5 - input.since_seen / (2.0 * tune.sight_grace)).clamp(-1.0, -0.5);
    }
    if input.distance > tune.evade_dead_zone {
        let span = (tune.front_sight - tune.evade_dead_zone).max(f32::EPSILON);
        return (-0.1 - 0.4 * (input.distance - tune.evade_dead_zone) / span).clamp(-0.5, -0.1);
    }
    if input.distance >= tune.bust_dead_zone || input.speed > tune.neutral_speed {
        return 0.0;
    }
    let distance_span = (tune.bust_dead_zone - bust.reach).max(f32::EPSILON);
    let distance = ((tune.bust_dead_zone - input.distance) / distance_span).clamp(0.0, 1.0);
    let speed_span = (tune.warning_speed - input.slow_speed).max(f32::EPSILON);
    let speed = ((tune.warning_speed - input.speed) / speed_span).clamp(0.0, 1.0);
    (0.3 * distance + 0.7 * speed) * 0.4 + 0.1
}

/// Blend the timer with the proximity warning; the display applies its terminal-state clamp separately.
pub fn time_until_busted(timer: f32, warning: f32, evade: f32, timeout: f32) -> f32 {
    if timer > 0.03 {
        let timer = (timer / timeout.max(f32::EPSILON)).min(1.0);
        return (1.0 - warning) * timer + warning;
    }
    if evade >= 0.05 {
        return -1.0;
    }
    warning
}

impl Busted {
    /// The signed five-part bar: orange warning precedes the red timer, and loss is negative.
    pub fn display_meter(&self, input: MeterInput, bust: &BustTuning, tune: &MeterTuning) -> f32 {
        if self.is_busted() {
            return 1.0;
        }
        let warning = proximity(input, bust, tune);
        time_until_busted(self.timer(), warning, input.evade, bust.timeout).clamp(-1.0, 0.99)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::busted::BustInput;

    const BUST: BustTuning = BustTuning {
        timeout: 5.0,
        reach: 15.0,
        flashing_reach: 90.0,
        height_gate: 1.5,
        fill_rate: 1.0,
        drain_rate: 2.0,
        flashing_multiplier: 4.0,
        flashing_speed_gate: false,
    };
    const DISPLAY: MeterTuning = MeterTuning {
        bust_dead_zone: 35.0,
        evade_dead_zone: 50.0,
        front_sight: 151.0,
        warning_speed: 100.0 / 3.6,
        neutral_speed: 70.0 * 0.44704,
        fresh_sighting: 0.29,
        sight_grace: 7.0,
    };

    fn input(distance: f32, speed: f32, since_seen: f32) -> MeterInput {
        MeterInput { distance, speed, since_seen, slow_speed: 2.5, evade: 0.0 }
    }

    #[test]
    fn orange_warning_precedes_the_red_timer_and_distance_can_favour_escape() {
        let bust = Busted::default();
        assert_eq!(bust.display_meter(input(10.0, 0.0, -0.25), &BUST, &DISPLAY), 0.5);
        assert_eq!(bust.display_meter(input(40.0, 10.0, -0.25), &BUST, &DISPLAY), 0.0);
        assert_eq!(bust.display_meter(input(151.0, 20.0, -0.25), &BUST, &DISPLAY), -0.5);
        assert_eq!(bust.display_meter(input(10.0, 40.0, -0.25), &BUST, &DISPLAY), 0.0);
    }

    #[test]
    fn lost_contact_moves_the_warning_towards_cooldown() {
        let bust = Busted::default();
        let near = bust.display_meter(input(10.0, 0.0, 0.5), &BUST, &DISPLAY);
        let lost = bust.display_meter(input(10.0, 0.0, 6.0), &BUST, &DISPLAY);
        assert!(near < -0.5 && lost < near);
        assert_eq!(bust.display_meter(input(10.0, 0.0, 7.1), &BUST, &DISPLAY), -1.0);
    }

    #[test]
    fn red_timer_blends_with_the_warning_and_only_busted_reaches_one() {
        let mut bust = Busted::default();
        let close = BustInput { cop_distance: 10.0, seen: true, player_speed: 0.0, slow_speed: 2.5, flashing: false };
        bust.update(2.5, &close, &BUST);
        assert_eq!(bust.display_meter(input(10.0, 0.0, -0.25), &BUST, &DISPLAY), 0.75);
        bust.update(2.5, &close, &BUST);
        assert_eq!(bust.display_meter(input(10.0, 0.0, -0.25), &BUST, &DISPLAY), 0.99);
        bust.update(0.01, &close, &BUST);
        assert_eq!(bust.display_meter(input(10.0, 0.0, -0.25), &BUST, &DISPLAY), 1.0);
    }

    #[test]
    fn timer_blend_matches_edge_cases_from_the_original_instruction_oracle() {
        // Independently emulated original instructions, not a live-game capture.
        for (timer, warning, evade, expected) in [
            (1.0, 0.0, 0.0, 0.2),
            (2.0, 0.5, 0.0, 0.7),
            (4.9, -0.8, 0.0, 0.964_000_05),
            (0.0, 0.3, 0.05, -1.0),
            (0.5, 0.3, 0.5, 0.37),
        ] {
            let actual = time_until_busted(timer, warning, evade, 5.0);
            assert!((actual - expected).abs() < 1e-6, "timer{timer}, warning{warning}, evade{evade}: {actual}");
        }
    }
}
