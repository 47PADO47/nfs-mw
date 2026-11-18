//! The race trace's `sample` lines: every racer's place and speed, and the controls the player's car was given, every
//! tenth of a second of race time, for comparing how the cars accelerate and which lines they take.

use serde::Serialize;

/// Seconds of race time between two samples.
pub const SAMPLE_PERIOD: f64 = 0.1;

/// When the next sample is due. A sample is taken on a physics tick, the first whose race time has reached the next
/// multiple of [`SAMPLE_PERIOD`] (to half a tick), so the samples keep to the ticks and do not drift from the period.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SampleClock {
    /// Samples taken since the green light.
    taken: u64,
}

impl SampleClock {
    /// Whether the tick of `dt` seconds that has brought the race time to `race_time` takes a sample. The first is the
    /// tick of the green light (race time 0).
    pub fn due(&mut self, race_time: f32, dt: f32) -> bool {
        let at = self.taken as f64 * SAMPLE_PERIOD;
        if f64::from(race_time) + f64::from(dt.max(0.0)) * 0.5 < at {
            return false;
        }
        // A tick longer than the period takes one sample; the next is due after the multiple nearest its race time.
        self.taken = (f64::from(race_time) / SAMPLE_PERIOD + 0.5).floor().max(self.taken as f64) as u64 + 1;
        true
    }
}

/// The controls the player's car stepped with in the tick of a sample, after the input shaping: what the car was given,
/// whoever drove it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Controls {
    /// 0 to 1.
    pub throttle: f32,
    /// 0 to 1.
    pub brake: f32,
    /// -1 to 1, positive to the right.
    pub steering: f32,
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
}

/// One racer in a sample.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SampleRow {
    pub racer: usize,
    /// The car's place in the game's world coordinates, metres: x and y on the ground, z up (the axes of the data's
    /// `Position`).
    pub position: [f32; 3],
    /// Speed along the car, m/s, without its sign.
    pub speed: f32,
    /// The player's car only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controls: Option<Controls>,
}
