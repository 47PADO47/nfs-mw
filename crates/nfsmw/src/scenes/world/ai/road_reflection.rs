//! Ground vehicles share the actor feed; helicopter capture is a separate contract.

use blackbox_gfx::Instance;

use super::TrafficWorld;

impl TrafficWorld {
    pub(in crate::scenes::world) fn road_reflection_instances(&self, out: &mut Vec<Instance>) {
        // Traffic, patrol/pursuit/roadblock cops and their trailers share this collection.
        for car in &self.cars {
            car.road_reflection_instances(self.alpha, out);
        }
    }
}
