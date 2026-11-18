//! The race trace's `sample` lines: every racer's place and speed, the controls each car stepped with and what its AI
//! driver asked for, every tenth of a second of race time, for comparing how the cars accelerate and which lines they
//! take.

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

/// The controls a car stepped with in the tick of a sample, after the input shaping: what the car was given, whoever
/// drove it, and its engine speed after the step.
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
    /// Engine revolutions per minute.
    pub rpm: f32,
    /// The engine's drive torque this step, N m, and the wheels touching the ground.
    pub drive_torque: f32,
    pub wheels_on_ground: u32,
}

/// What the AI driver of a racer asked for in the tick of a sample: an opponent's driver, or the autopilot's in the
/// player's car.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DriverRow {
    /// The speed the driver is asked for, m/s: the think's, or the recovery speed while it reverses.
    pub target_speed: f32,
    /// The character's skill, 0 to 1.
    pub base_skill: f32,
    /// The catch-up offset added to the skill this tick (0 for the player's car).
    pub catch_up: f32,
    /// The skill the driver uses: the base plus the offset, in 0..1.
    pub skill: f32,
    /// The catch-up force budget, 0 to 0.5: it scales the car's engine torque, drag and downforce (0 for the player's
    /// car).
    pub catch_up_force: f32,
    /// The racer moves on its off-world stand-in, not by physics; `controls` are then those of its last physics step.
    pub stand_in: bool,
    /// The car's nitrous is burning (false for the player's car).
    pub nitrous: bool,
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
    /// Every racer with a car to read: the player's and the opponents'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controls: Option<Controls>,
    /// Every racer an AI driver drives: the opponents, and the player's car while the autopilot has it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<DriverRow>,
}
