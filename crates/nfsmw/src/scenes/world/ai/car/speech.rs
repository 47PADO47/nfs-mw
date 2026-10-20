//! What the police radio asks of a car: its identity, what it was doing when it was hit, and what its last step did.

use super::*;
use crate::audio::Vehicle;
use crate::scenes::world::drive::SurfaceHit;

impl AiCar {
    /// The identity the radio knows the car by: the same number its [`CopFeed`](crate::audio::CopFeed) carries.
    pub(in crate::scenes::world::ai) fn speech_id(&self) -> u32 {
        self.cop_health_sample().map_or_else(|| self.render_key(), |sample| sample.key)
    }

    /// The car is a cop of any kind: chasing, patrolling or standing in a roadblock.
    pub(in crate::scenes::world::ai) fn is_cop_class(&self) -> bool {
        self.driver_class() == super::super::collide::DriverClass::Cop
    }

    /// The car brakes or has its handbrake on.
    pub(in crate::scenes::world::ai) fn is_braking(&self) -> bool {
        self.controls.brake > 0.0 || self.controls.handbrake > 0.0
    }

    /// The car is one of the SUVs (a ram of one is no deliberate hit on a cop).
    pub(in crate::scenes::world::ai) fn is_suv(&self) -> bool {
        crate::audio::radio_tables::vehicle_of(&self.name) == Vehicle::Suv
    }

    /// The car is a tractor, or tows a trailer: a hit on it is a hit on a semi.
    pub(in crate::scenes::world::ai) fn is_semi(&self) -> bool {
        self.tractor
    }

    /// Pretends the car's last step reacted against `hit`.
    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn force_surface_hit(&mut self, hit: SurfaceHit) {
        self.sim.force_step_hit(hit);
    }

    /// The surface the car's last step reacted against, once.
    pub(in crate::scenes::world::ai) fn take_surface_hit(&mut self) -> Option<SurfaceHit> {
        self.sim.take_step_hit()
    }
}
