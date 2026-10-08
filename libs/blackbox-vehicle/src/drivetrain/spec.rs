/// Gear id of reverse.
pub const GEAR_REVERSE: usize = 0;
/// Gear id of neutral.
pub const GEAR_NEUTRAL: usize = 1;
/// Gear id of first gear.
pub const GEAR_FIRST: usize = 2;

/// Transmission parameters (the attribute class `transmission`).
///
/// `gear_ratio` and `gear_efficiency` are indexed by gear id: entry 0 is reverse, entry 1 neutral (ratio 0),
/// entry 2 first gear. Ratios are used as magnitudes; reverse is applied with a sign flip.
#[derive(Clone, Debug)]
pub struct TransmissionSpec {
    pub gear_ratio: Vec<f32>,
    pub gear_efficiency: Vec<f32>,
    pub final_gear: f32,
    /// Fraction of drive torque sent to the front axle (0 = rear drive, 1 = front drive).
    pub torque_split: f32,
    /// Lock factors of the front, rear and centre differentials (0 open .. 1 locked).
    pub differential: [f32; 3],
    /// Launch torque multiplication of an automatic, 0 for none.
    pub torque_converter: f32,
    /// Clutch play and launch drag factor, 0..1.
    pub clutch_slip: f32,
    /// Seconds of shift delay per unit of gear ratio.
    pub shift_speed: f32,
    /// Perfect-shift window in rpm per unit of ratio (drag races; unused here).
    pub optimal_shift: f32,
}

impl TransmissionSpec {
    /// Highest gear id.
    pub fn top_gear(&self) -> usize {
        self.gear_ratio.len().saturating_sub(1)
    }

    /// Ratio magnitude of a gear id (0 if out of range).
    pub fn ratio(&self, gear: usize) -> f32 {
        self.gear_ratio.get(gear).copied().unwrap_or(0.0).abs()
    }

    /// Driveline efficiency of a gear id (1 if the table is shorter).
    pub fn efficiency(&self, gear: usize) -> f32 {
        self.gear_efficiency.get(gear).copied().unwrap_or(1.0)
    }

    /// True when the front axle receives torque.
    pub fn front_driven(&self) -> bool {
        self.torque_split > 0.0
    }

    /// True when the rear axle receives torque.
    pub fn rear_driven(&self) -> bool {
        self.torque_split < 1.0
    }
}
