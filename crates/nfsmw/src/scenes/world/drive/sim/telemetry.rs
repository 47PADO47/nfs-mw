//! What the readout shows.

#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry {
    /// Speed along the car in metres per second (negative in reverse).
    pub speed_mps: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// Nitrous tank, 0..1.
    pub nos: f32,
    /// The nitrous is burning (held, in gear, on the throttle, fast enough), not just asked for.
    pub nos_burning: bool,
    /// The car has a nitrous system.
    pub has_nos: bool,
    pub wheels_on_ground: usize,
    /// The engine's red line, rpm.
    pub red_line: f32,
    /// The engine's `MAX_RPM`: it picks the tachometer's scale.
    pub max_rpm: f32,
    /// The engine's idle speed, rpm.
    pub idle: f32,
    /// A gear change is in progress.
    pub shifting: bool,
    /// The gearbox wants the next gear (the shift light).
    pub shift_up: bool,
    /// The car has forced induction (a turbo or a supercharger).
    pub has_induction: bool,
    /// The boost gauge, psi.
    pub boost_psi: f32,
}

impl Telemetry {
    /// The nitrous as the high-speed effects (blur, field-of-view kick) see it: 1 while it burns, else 0. A full
    /// tank that is not burning adds nothing.
    pub fn nitrous_burn(&self) -> f32 {
        match self.has_nos && self.nos_burning {
            true => 1.0,
            false => 0.0,
        }
    }
}
