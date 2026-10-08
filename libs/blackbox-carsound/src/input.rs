//! The per-frame telemetry the controllers read: what the physics knows about the car.

/// Gear ids as the transmission reports them.
pub const GEAR_REVERSE: i32 = 0;
pub const GEAR_NEUTRAL: i32 = 1;
pub const GEAR_FIRST: i32 = 2;

/// One wheel. The wheel order everywhere is front left, front right, rear right, rear left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelInput {
    /// The tire touches the ground.
    pub on_ground: bool,
    /// Forward slip as the physics reports it (signed).
    pub slip: f32,
    /// The slip the tire tolerates before it skids; a dead zone of 0.2 times this is cut from `slip`.
    pub tolerated_slip: f32,
    /// Lateral skid as the physics reports it (signed; the sound uses its negative).
    pub skid: f32,
    /// Load on the wheel (newtons), ramped between 4000 and 10000 for the skid sound.
    pub load: f32,
    /// Suspension compression (the "Z force"), used for landings.
    pub compression: f32,
    /// Magnitude of the traction in use (adds to the road noise).
    pub traction_usage: f32,
    /// `Aud_Skid_Type` of the surface under the tire.
    pub skid_surface: u8,
    /// `Aud_Roadnoise_LOOP` of the surface under the tire (0 = none).
    pub road_noise_loop: u8,
    /// The tire is blown.
    pub blown: bool,
}

impl Default for WheelInput {
    fn default() -> Self {
        Self {
            on_ground: true,
            slip: 0.0,
            tolerated_slip: 0.0,
            skid: 0.0,
            load: 0.0,
            compression: 0.0,
            traction_usage: 0.0,
            skid_surface: 0,
            road_noise_loop: 0,
            blown: false,
        }
    }
}

/// Everything the controllers read each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarInput {
    /// `(rpm - idle) / (redline - idle)`, 0 to 1 (0 when the redline is not above the idle).
    pub rpm_pct: f32,
    /// Throttle pedal, 0 to 1.
    pub throttle: f32,
    /// Brake pedal, 0 to 1.
    pub brake: f32,
    /// Gear id: [`GEAR_REVERSE`] 0, [`GEAR_NEUTRAL`] 1, first 2, second 3, ...
    pub gear: i32,
    /// Speed in metres per second (the sign is ignored).
    pub speed: f32,
    /// Nitrous is engaged.
    pub nos_active: bool,
    /// The nitrous tank ran dry (true for as long as the physics holds the flag).
    pub nos_empty: bool,
    /// 0 none, 1 blown, 2 sabotaged: plays a moment sound once.
    pub engine_blown: u8,
    pub wheels: [WheelInput; 4],
    /// Dot product of the car's up vector with the world up (1 = level, below 0.8 = heavily leaning).
    pub up_dot: f32,
    /// The race has not started: the tachometer shows the physics RPM.
    pub pre_race: bool,
    /// The dynamic mixer's pitch multiplier for the engine (1 = unchanged).
    pub pitch_multiplier: f32,
}

impl Default for CarInput {
    fn default() -> Self {
        Self {
            rpm_pct: 0.0,
            throttle: 0.0,
            brake: 0.0,
            gear: GEAR_NEUTRAL,
            speed: 0.0,
            nos_active: false,
            nos_empty: false,
            engine_blown: 0,
            wheels: [WheelInput::default(); 4],
            up_dot: 1.0,
            pre_race: false,
            pitch_multiplier: 1.0,
        }
    }
}

impl CarInput {
    /// Number of wheels on the ground.
    pub fn wheels_on_ground(&self) -> usize {
        self.wheels.iter().filter(|w| w.on_ground).count()
    }

    /// Speed in miles per hour.
    pub fn speed_mph(&self) -> f32 {
        let v = self.speed.abs();
        if v.is_finite() { v * 2.2369 } else { 0.0 }
    }
}
