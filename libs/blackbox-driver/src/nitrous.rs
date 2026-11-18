//! The race action's nitrous: the tank and boost a racer may use after matching, and when it presses the button.
//! Spec: `docs/specs/ai-driver-speed-skill.md` sections 2 and 7.1.

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A car's nitrous hardware: seconds of burn in a full tank, and the extra torque while it burns (the multiplier less
/// one). Both 0 without nitrous.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NitrousHardware {
    pub capacity: f32,
    pub boost: f32,
}

impl NitrousHardware {
    pub fn present(&self) -> bool {
        self.capacity > 0.0 && self.boost > 0.0
    }
}

/// What a racer may use of its nitrous, fixed when it is created: the matched boost, the usable share of the tank and
/// the seconds of a full tank.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NitrousLimits {
    pub boost: f32,
    pub usable: f32,
    pub bottle_time: f32,
}

impl NitrousLimits {
    /// Matches `own` to the weakest of itself and `players` (`bias` 0 takes the weakest boost, 1 keeps its own). A
    /// player autopilot (`always`) may use the whole tank.
    pub fn matched(own: NitrousHardware, players: &[NitrousHardware], bias: f32, always: bool) -> Self {
        let weakest = |get: fn(&NitrousHardware) -> f32| players.iter().map(get).fold(get(&own), |a, b| a.min(b));
        let boost = lerp(weakest(|n| n.boost.max(0.0)), own.boost.max(0.0), bias.clamp(0.0, 1.0));
        let capacity = weakest(|n| n.capacity.max(0.0));
        if always {
            return Self { boost, usable: 1.0, bottle_time: own.capacity };
        }
        if own.capacity <= 0.0 || own.boost <= 0.0 || boost <= 0.0 {
            return Self { boost, usable: 0.0, bottle_time: 0.0 };
        }
        Self { boost, usable: (capacity / own.capacity) * (boost / own.boost), bottle_time: own.capacity }
    }

    /// Seconds of nitrous the racer is willing to burn now: 0 below 10 m/s, at or above its requested speed, without
    /// usable nitrous, or when the tank (0..1) holds no more than the reserve it keeps.
    pub fn potential(&self, speed: f32, speed_limit: f32, skill: f32, burning: bool, tank: f32) -> f32 {
        if speed < 10.0 || speed >= speed_limit || self.boost <= 0.0 || self.usable <= 0.0 {
            return 0.0;
        }
        let off_limit = 1.0 - lerp(0.33, 1.0, skill) * self.usable;
        let needed = match burning {
            true => off_limit,
            false => (1.0 - off_limit) * lerp(0.5, 0.3, skill),
        };
        if tank <= needed {
            return 0.0;
        }
        (tank - needed) * self.bottle_time
    }
}

/// One think's view of the racer for its nitrous choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NitrousView {
    pub speed: f32,
    pub speed_limit: f32,
    pub potential_speed: f32,
    pub potential_accel: f32,
    /// The car's acceleration since the last think, m/s².
    pub acceleration: f32,
    pub skill: f32,
    /// The button is held and the tank is not empty.
    pub burning: bool,
    /// Tank level, 0..1.
    pub tank: f32,
    /// The racer may not use nitrous now (staging, turning, steering round something in its way).
    pub barred: bool,
}

/// The racer's nitrous timer: seconds on (positive) or off (negative), from 0 when the race action begins.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NitrousTimer {
    pub seconds: f32,
}

impl NitrousTimer {
    /// Decides the button for this think of `dt` seconds.
    pub fn decide(&mut self, limits: &NitrousLimits, view: &NitrousView, dt: f32) -> bool {
        let offer = limits.potential(view.speed, view.speed_limit, view.skill, view.burning, view.tank);
        let mut want = false;
        if !view.barred && offer > 0.0 {
            let scale = lerp(0.25, 1.0, view.skill);
            let speed_gap = scale * (view.potential_speed - view.speed);
            let accel_gap = scale * (view.potential_accel - view.acceleration);
            want = match view.burning {
                false => {
                    let off_time = lerp(20.0, 4.0, view.skill);
                    self.seconds < -off_time && speed_gap > 15.0 && accel_gap > 0.5 * view.potential_accel
                }
                true => {
                    let on_time = lerp(2.0, 4.0, view.skill);
                    !(self.seconds > on_time && (speed_gap < 5.0 || accel_gap < 0.15 * view.potential_accel))
                }
            };
        }
        self.seconds = match want {
            true => (self.seconds + dt).max(0.0),
            false => (self.seconds - dt).min(0.0),
        };
        want
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(capacity: f32, boost: f32) -> NitrousHardware {
        NitrousHardware { capacity, boost }
    }

    #[test]
    fn matching_takes_the_weaker_tank_and_boost() {
        let limits = NitrousLimits::matched(hw(5.0, 1.0), &[hw(2.5, 0.75)], 0.0, false);
        assert!((limits.boost - 0.75).abs() < 1e-6);
        assert!((limits.usable - 0.375).abs() < 1e-6);
        assert_eq!(limits.bottle_time, 5.0);
        let player = NitrousLimits::matched(hw(2.5, 0.75), &[], 0.0, true);
        assert_eq!((player.usable, player.bottle_time), (1.0, 2.5));
        assert_eq!(NitrousLimits::matched(hw(0.0, 0.0), &[hw(2.5, 0.75)], 0.0, false).usable, 0.0);
    }

    #[test]
    fn the_first_burn_waits_for_the_off_time_of_the_skill() {
        let limits = NitrousLimits::matched(hw(5.0, 1.0), &[hw(2.5, 0.75)], 0.0, false);
        let view = NitrousView {
            speed: 50.0,
            speed_limit: 60.0,
            potential_speed: 100.0,
            potential_accel: 8.0,
            acceleration: 0.0,
            skill: 0.5,
            burning: false,
            tank: 1.0,
            barred: false,
        };
        let mut timer = NitrousTimer::default();
        let dt = 1.0 / 30.0;
        let mut ticks = 0;
        while !timer.decide(&limits, &view, dt) {
            ticks += 1;
            assert!(ticks < 1000);
        }
        // 12 s off at skill 0.5 (lerp 20 -> 4), counted from the start.
        assert!((ticks as f32 * dt - 12.0).abs() < 0.05, "{}", ticks as f32 * dt);
        assert!(timer.seconds >= 0.0);
    }

    #[test]
    fn no_nitrous_below_ten_metres_per_second_or_at_the_limit() {
        let limits = NitrousLimits::matched(hw(5.0, 1.0), &[], 0.0, false);
        assert_eq!(limits.potential(9.9, 60.0, 0.5, false, 1.0), 0.0);
        assert_eq!(limits.potential(60.0, 60.0, 0.5, false, 1.0), 0.0);
        assert!(limits.potential(30.0, 60.0, 0.5, false, 1.0) > 0.0);
    }
}
