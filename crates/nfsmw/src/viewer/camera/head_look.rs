//! The look of the rider's head: the right stick of a pad turns the helmet camera a little to either side and it springs back to
//! straight ahead when the stick is let go (`docs/specs/ride6-bike-cameras.md`, section 6.2). Only the camera in a bike's helmet reads
//! it; a car, the other cameras of a bike and the bike's own motion never see it.

/// The most the head turns to either side, degrees. RIDE 6's `HelmetIgnitionSideViewCameraBehavior.YawModifier.OffsetLimits`
/// (-11 to 11), the same in the base container every bike takes it from [D].
pub const LOOK_YAW_LIMIT_DEG: f32 = 11.0;
/// RIDE 6's own most the head tips up or down, degrees: `PitchModifier.OffsetLimits` (-10 to 10) of the same behaviour [D]. Not
/// used any more: the player asked for more ("let me look a bit up and down too"), see [`LOOK_UP_LIMIT_DEG`].
pub const LOOK_PITCH_DATA_LIMIT_DEG: f32 = 10.0;
/// The most the head tips up, degrees [C]: a quarter of a turn of a rider's neck looking over the screen of a fairing, enough to
/// see a bend's exit and the sky above the rest view (which looks about 26 degrees down, so +25 puts the view on the horizon).
pub const LOOK_UP_LIMIT_DEG: f32 = 25.0;
/// The most the head tips down, degrees [C]: more than up, since the interesting things (the dash, the hands, the front tyre,
/// the road at the wheel) are below the rest view. At rest the view is 26 degrees down, so this adds up to 56 degrees below the
/// horizon, short of the bike's own tank and front tyre.
pub const LOOK_DOWN_LIMIT_DEG: f32 = 30.0;
/// How fast the look follows the stick and returns to straight ahead, per second: the behaviour's `RotatorFilterSpeed` (4; 5 on one
/// bike's container) [D]. Read as the rate of an exponential approach, so the look closes 63 percent of the way to where the stick
/// asks every quarter of a second, and is back within a hundredth of a degree two seconds after the stick is released [C]: the
/// data names the speed, not the filter (it is Unreal's `RInterpTo`, whose step is the speed times the frame time; the exact
/// form here does not depend on the frame rate).
pub const LOOK_FILTER_SPEED: f32 = 4.0;

/// Where the rider's head looks from straight ahead, degrees: right and up are positive.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct HeadLook {
    right_deg: f32,
    up_deg: f32,
}

impl HeadLook {
    /// Moves the look `dt` seconds toward where a stick pushed by `stick` (x right, y up, each -1 to 1; a number out of that range
    /// is held to it and one that is not a number is the stick at rest) asks it to be: the stick's share of the limits.
    pub fn update(&mut self, stick: [f32; 2], dt: f32) {
        // A frame of no time, or of a time that is not one, does not move it.
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let [right, up] = Self::target(stick);
        let step = 1.0 - (-LOOK_FILTER_SPEED * dt).exp();
        self.right_deg += (right - self.right_deg) * step;
        self.up_deg += (up - self.up_deg) * step;
    }

    /// Where a stick pushed by `stick` (x right, y up, each -1 to 1; a number out of that range is held to it and one that is not a
    /// number is the stick at rest) asks the head to be, `[right, up]` degrees: the stick's share of the limits, linear on
    /// either side of the middle. Right is [`LOOK_YAW_LIMIT_DEG`] and left the same; up is [`LOOK_UP_LIMIT_DEG`] and down
    /// [`LOOK_DOWN_LIMIT_DEG`]. A pure function of the input, the part of the look that does not depend on time.
    pub fn target(stick: [f32; 2]) -> [f32; 2] {
        let held = |value: f32| match value.is_finite() {
            true => value.clamp(-1.0, 1.0),
            false => 0.0,
        };
        let up = held(stick[1]);
        let pitch = match up >= 0.0 {
            true => up * LOOK_UP_LIMIT_DEG,
            false => up * LOOK_DOWN_LIMIT_DEG,
        };
        [held(stick[0]) * LOOK_YAW_LIMIT_DEG, pitch]
    }

    /// How far the head is turned to the right, degrees (negative: to the left).
    pub fn right_deg(&self) -> f32 {
        self.right_deg
    }

    /// How far the head is tipped up, degrees (negative: down).
    pub fn up_deg(&self) -> f32 {
        self.up_deg
    }
}

#[cfg(test)]
#[path = "head_look_tests.rs"]
mod tests;
