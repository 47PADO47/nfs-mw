use super::*;
use crate::scenes::world::vehicle_effects::WaterContact;
use blackbox_collision::CollisionWorld;
impl AiCar {
    pub(in crate::scenes::world::ai) fn rain_contacts(
        &self,
        collision: &CollisionWorld,
    ) -> ([Option<WaterContact>; 4], CarPose, Vec3) {
        (self.sim.water_contacts(self.sim.tire_contacts(collision)), self.sim.pose(), self.sim.effect_velocity())
    }
}
