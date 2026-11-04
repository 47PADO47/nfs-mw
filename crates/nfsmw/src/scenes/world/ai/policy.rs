//! Population budget and collision support of the selected traffic policy.
use super::{Focus, TrafficWorld};
use crate::settings::TrafficPolicy;
use glam::Vec2;

impl TrafficWorld {
    /// Apply the event's density and pattern without changing the configured actor budget.
    pub fn set_event_traffic(&mut self, percent: u32, pattern: Option<&str>) {
        self.event_density = percent.min(100);
        let pattern = pattern.filter(|name| !name.is_empty());
        if self.event_pattern.as_deref() != pattern {
            self.event_pattern = pattern.map(str::to_owned);
        }
    }

    pub fn set_policy(&mut self, policy: TrafficPolicy) {
        self.policy = policy.sanitized();
    }

    pub(super) fn density(&self) -> f32 {
        if self.world_moment {
            return 0.0;
        }
        let context = super::manager::density(self.target > 0, self.pursuit.is_some());
        self.policy.density(context * self.event_density as f32 / 100.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_roads::RoadNetwork;

    #[test]
    fn traffic_parity_event_density_scales_replenishment_without_rescaling_actor_slots() {
        let net = RoadNetwork { nodes: vec![], segments: vec![], profiles: vec![], roads: vec![] };
        let mut world = TrafficWorld::new(net);
        world.configure(20, 0);
        for percent in [0, 10, 30, 50, 100] {
            world.set_event_traffic(percent, None);
            assert_eq!(world.target, 20, "event percentage {percent} changed the actor cap");
            assert!((world.density() - percent as f32 / 100.0).abs() < 1e-6);
        }
        let policy = TrafficPolicy { density_percent: 50, ..Default::default() };
        world.set_policy(policy);
        world.set_event_traffic(30, Some("cityhighway"));
        assert!((world.density() - 0.15).abs() < 1e-6);
        assert_eq!(world.event_pattern.as_deref(), Some("cityhighway"));
        world.set_event_traffic(100, None);
        assert!((world.density() - 0.5).abs() < 1e-6, "free roam restores context without overwriting user density");
        assert_eq!(world.event_pattern, None);
        world.world_moment = true;
        assert_eq!(world.density(), 0.0, "cinematics still suppress ambient spawns");
    }
}
