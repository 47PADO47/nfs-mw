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

    /// The traffic density now: none in a world moment or while the cop manager has a request it has not fulfilled.
    pub(super) fn density(&self) -> f32 {
        if self.world_moment || self.cop_request_pending() {
            return 0.0;
        }
        let context = super::manager::density(self.target > 0, self.pursuit.is_some());
        self.policy.density(context * self.event_density as f32 / 100.0)
    }

    /// Whether the cop manager has a request open (`IsCopRequestPending`, spec `ai-traffic-spawning.md` section 4): a
    /// scripted spawn request waiting, a latched roadblock request, or a wave cop due and not yet placed. Only while cops
    /// may spawn.
    pub(super) fn cop_request_pending(&self) -> bool {
        if !self.cops.spawning_enabled() {
            return false;
        }
        !self.cops.requests.is_empty()
            || self.roadblocks.dispatcher_latched()
            || self.pursuit.as_ref().is_some_and(|pursuit| pursuit.cop_due)
    }

    /// The traffic budget: every car but the racers, which the race puts on its own grid. Counting them would leave a race of
    /// sixteen no traffic at the low settings **[ours]**. The helicopter counts as a vehicle.
    pub(super) fn population_slots(&self, focus: Focus) -> usize {
        let traffic = self.cars.iter().filter(|c| !c.is_racer()).count();
        traffic
            + self.cars.iter().filter(|c| c.trailer().is_some()).count()
            + self.fixed_slots(focus)
            + self.parked_police()
    }

    pub(super) fn other_slots(&self, focus: Focus) -> usize {
        self.cars.iter().filter(|c| c.is_cop() || c.is_patrol()).count()
            + self.cars.iter().filter(|c| c.trailer().is_some()).count()
            + self.fixed_slots(focus)
            + self.parked_police()
    }

    /// Police vehicles of the chase that are off the road: the most it has had on the road at once, less those on it
    /// now. The original parks a police car that leaves and reuses it for the next cop, so it keeps its slot in the
    /// shared budget (spec `ai-traffic-spawning.md` section 5). Released when the chase ends **[ours]**.
    pub(super) fn parked_police(&self) -> usize {
        let Some(pursuit) = &self.pursuit else { return 0 };
        (pursuit.fleet as usize).saturating_sub(self.police_on_road())
    }

    /// Follows the chase's fleet: the most police vehicles (chase, patrol, roadblock, support, wrecks) on the road at once.
    pub(super) fn note_police_fleet(&mut self) {
        let on_road = self.police_on_road() as u32;
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        pursuit.fleet = pursuit.fleet.max(on_road);
    }

    fn police_on_road(&self) -> usize {
        self.cars.iter().filter(|c| c.is_police()).count()
    }

    /// The player and the helicopter, when there are.
    fn fixed_slots(&self, focus: Focus) -> usize {
        usize::from(focus.body.is_some()) + usize::from(self.police.helicopter_motion().is_some())
    }

    /// Whether a wave cop may take a slot of the vehicle budget it shares with traffic and the player: a parked police
    /// vehicle's, or a free one. Active traffic is never removed for it: it waits, with traffic paused, until cars retire.
    /// The budget is never under the reference 20, so a low traffic setting does not starve the chase **[ours]**.
    pub(super) fn vehicle_budget_has_room(&self, focus: Focus) -> bool {
        if self.parked_police() > 0 {
            return true;
        }
        let budget = self.target.max(crate::settings::DEFAULT_TRAFFIC as usize);
        self.population_slots(focus) < budget
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
