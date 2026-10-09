use super::Drive;

impl Drive {
    pub fn spark_body(&self) -> (super::CarPose, nfsmw_data::car::physics::CarBounds) {
        (self.pose(), self.physics.bounds)
    }

    /// Age parked effects without emitting. Completed screenshot batches stay deterministic.
    pub fn age_effects(&mut self, dt: f32) {
        self.effects.disconnect();
        self.vehicle_effects.disconnect();
        if !self.batch_run {
            self.effects.age(dt);
            self.vehicle_effects.age(dt);
        }
    }
}
