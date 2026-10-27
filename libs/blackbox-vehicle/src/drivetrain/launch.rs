use super::Powertrain;

impl Powertrain {
    /// Sets the race context; zero strength preserves ordinary engine behavior.
    pub fn set_launch_context(&mut self, staging: bool, strength: f32) {
        self.staging = staging;
        self.launch_strength = match strength.is_finite() && strength > 0.0 && !staging {
            true => strength,
            false => 0.0,
        };
    }
}
