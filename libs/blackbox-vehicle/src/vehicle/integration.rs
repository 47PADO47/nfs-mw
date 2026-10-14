//! Tire integration policy retained over placement and hardware replacement.

use super::Vehicle;
use crate::tires::TireIntegration;

impl Vehicle {
    pub fn tire_integration(&self) -> TireIntegration {
        self.tire_integration
    }

    pub fn drift_strength(&self) -> f32 {
        self.drift.value
    }

    pub fn set_tire_integration(&mut self, integration: TireIntegration) {
        self.tire_integration = integration;
        self.burnout = Default::default();
        for tire in &mut self.tires {
            tire.integration = integration;
        }
    }
}
