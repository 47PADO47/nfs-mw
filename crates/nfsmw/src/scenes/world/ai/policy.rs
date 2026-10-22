//! Population budget and collision support of the selected traffic policy.
use super::{Focus, TrafficWorld};
use crate::settings::TrafficPolicy;
use glam::Vec2;

impl TrafficWorld {
    pub fn set_policy(&mut self, policy: TrafficPolicy) {
        self.policy = policy.sanitized();
    }

    pub(super) fn density(&self) -> f32 {
        self.policy.density(super::manager::density(self.target > 0, self.pursuit.is_some()))
    }

    /// The traffic budget: every car but the racers, which the race puts on its own grid. Counting them would leave a race of
    /// sixteen no traffic at the low settings **[ours]**.
    pub(super) fn population_slots(&self, focus: Focus) -> usize {
        let traffic = self.cars.iter().filter(|c| !c.is_racer()).count();
        traffic + self.cars.iter().filter(|c| c.trailer().is_some()).count() + usize::from(focus.body.is_some())
    }

    pub(super) fn other_slots(&self, focus: Focus) -> usize {
        self.cars.iter().filter(|c| c.is_cop() || c.is_patrol()).count()
            + self.cars.iter().filter(|c| c.trailer().is_some()).count()
            + usize::from(focus.body.is_some())
    }

    /// Actor support is independent of whether its scenery remains in the player's PVS.
    pub(super) fn collision_anchors(&self) -> Vec<(Vec2, f32)> {
        let mut anchors = Vec::with_capacity(self.cars.len() * 3);
        for car in &self.cars {
            if !car.is_finite() {
                continue;
            }
            let p = car.position().truncate();
            let velocity = super::super::space::to_render(car.avoidable().velocity.to_array()).truncate();
            anchors.push((p, 60.0));
            anchors.push((p + velocity * 2.0, 60.0));
            if let Some(trailer) = car.trailer() {
                anchors.push((trailer.position().truncate(), 60.0));
            }
        }
        anchors
    }
}
