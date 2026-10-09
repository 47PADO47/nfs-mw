//! What a driver writes: the same controls a human produces.

/// A gear the driver asks the gearbox for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearRequest {
    Reverse,
    /// The first forward gear (the automatic box takes over from there).
    First,
    /// Coasting, with the drive disengaged (a traffic car in shock).
    Neutral,
}

/// Gas, brake, handbrake and steering for one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AiControls {
    pub gas: f32,
    pub brake: f32,
    pub handbrake: f32,
    /// -1 (full left) … 1 (full right), normalised by the car's maximum steering angle.
    pub steer: f32,
    pub nos: bool,
    pub gear: Option<GearRequest>,
}

/// Which controllers run (`drive flags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveFlags(pub u8);

impl DriveFlags {
    pub const NONE: Self = Self(0);
    pub const STEER: u8 = 1;
    pub const GAS_BRAKE: u8 = 2;
    pub const REVERSE: u8 = 4;
    /// Traffic and stuck recovery: steering and pedals, no automatic reverse.
    pub const SIMPLE: Self = Self(Self::STEER | Self::GAS_BRAKE);
    /// Racers, cops and rams: everything.
    pub const FULL: Self = Self(Self::STEER | Self::GAS_BRAKE | Self::REVERSE);

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}
