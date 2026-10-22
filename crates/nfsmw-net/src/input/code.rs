//! One tick of input at wire precision.

use blackbox_vehicle::InputState;

/// Largest throttle or brake code: 0..=127 is 0..1.
pub const PEDAL_MAX: u8 = 127;
/// Steer code of a centred wheel; 0 is full left (-1), [`STEER_MAX`] full right (+1).
pub const STEER_CENTRE: u16 = 255;
/// Largest steer code.
pub const STEER_MAX: u16 = 510;
/// Largest handbrake code: 0..=15 is 0..1.
pub const HANDBRAKE_MAX: u8 = 15;
/// Largest clutch code: 0..=15 is 0..1.
pub const CLUTCH_MAX: u8 = 15;
/// Largest gear id a frame carries.
pub const GEAR_MAX: u8 = 15;

/// One tick of input in wire steps. Build it with [`InputCode::quantise`]; a decoded one is always valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputCode {
    /// 0..=[`PEDAL_MAX`].
    pub throttle: u8,
    /// 0..=[`PEDAL_MAX`].
    pub brake: u8,
    /// 0..=[`STEER_MAX`], centred at [`STEER_CENTRE`].
    pub steer: u16,
    /// 0..=[`HANDBRAKE_MAX`].
    pub handbrake: u8,
    pub nos: bool,
    /// Edges: set on the tick the shift happens only, never both.
    pub shift_up: bool,
    pub shift_down: bool,
    /// A gear asked for by id, 0..=[`GEAR_MAX`].
    pub gear_select: Option<u8>,
    /// 0..=[`CLUTCH_MAX`]; sent only in a session that uses the clutch.
    pub clutch: u8,
}

impl Default for InputCode {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

impl InputCode {
    /// No pedal, wheel centred, no button.
    pub const NEUTRAL: Self = Self {
        throttle: 0,
        brake: 0,
        steer: STEER_CENTRE,
        handbrake: 0,
        nos: false,
        shift_up: false,
        shift_down: false,
        gear_select: None,
        clutch: 0,
    };

    /// `input` at wire precision: non-finite axes read as zero, ranges are clamped, both shift edges at once become
    /// neither, and a gear above [`GEAR_MAX`] is dropped.
    pub fn quantise(input: &InputState) -> Self {
        let both = input.shift_up && input.shift_down;
        let gear_select = input.gear_select.and_then(|gear| u8::try_from(gear).ok()).filter(|gear| *gear <= GEAR_MAX);
        let steer = (unit(input.steer, -1.0) * f32::from(STEER_CENTRE)).round() as i32 + i32::from(STEER_CENTRE);
        Self {
            throttle: scale(input.throttle, PEDAL_MAX),
            brake: scale(input.brake, PEDAL_MAX),
            steer: steer.clamp(0, i32::from(STEER_MAX)) as u16,
            handbrake: scale(input.handbrake, HANDBRAKE_MAX),
            nos: input.nos,
            shift_up: input.shift_up && !both,
            shift_down: input.shift_down && !both,
            gear_select,
            clutch: scale(input.clutch, CLUTCH_MAX),
        }
    }

    /// The input this code stands for.
    pub fn to_input(&self) -> InputState {
        InputState {
            throttle: f32::from(self.throttle) / f32::from(PEDAL_MAX),
            brake: f32::from(self.brake) / f32::from(PEDAL_MAX),
            steer: (f32::from(self.steer) - f32::from(STEER_CENTRE)) / f32::from(STEER_CENTRE),
            handbrake: f32::from(self.handbrake) / f32::from(HANDBRAKE_MAX),
            nos: self.nos,
            shift_up: self.shift_up,
            shift_down: self.shift_down,
            gear_select: self.gear_select.map(usize::from),
            clutch: f32::from(self.clutch) / f32::from(CLUTCH_MAX),
        }
    }

    /// Whether every field is within its range and the shift edges are not both set.
    pub fn is_valid(&self) -> bool {
        self.throttle <= PEDAL_MAX
            && self.brake <= PEDAL_MAX
            && self.steer <= STEER_MAX
            && self.handbrake <= HANDBRAKE_MAX
            && self.clutch <= CLUTCH_MAX
            && self.gear_select.is_none_or(|gear| gear <= GEAR_MAX)
            && !(self.shift_up && self.shift_down)
    }
}

/// `x` clamped to `min..=1`, zero when not finite.
fn unit(x: f32, min: f32) -> f32 {
    match x.is_finite() {
        true => x.clamp(min, 1.0),
        false => 0.0,
    }
}

/// `x` in 0..1 as a code of 0..=`max`.
fn scale(x: f32, max: u8) -> u8 {
    (unit(x, 0.0) * f32::from(max)).round() as u8
}
