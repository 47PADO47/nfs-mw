//! A car in a race: what the race reads of it and tells it (`ai/racer.rs`).

use glam::Vec3;

use super::{AiCar, Role};
use crate::scenes::world::ai::racer::RacerState;

impl AiCar {
    /// The race index of the car, when it is a racer.
    pub fn racer_id(&self) -> Option<usize> {
        match &self.role {
            Role::Racer(racer) => Some(racer.id),
            _ => None,
        }
    }

    /// The car is a racer.
    pub fn is_racer(&self) -> bool {
        matches!(self.role, Role::Racer(_))
    }

    /// The racer's state, for the race to give it a goal, a skill or a release.
    pub fn racer_mut(&mut self) -> Option<&mut RacerState> {
        match &mut self.role {
            Role::Racer(racer) => Some(racer),
            _ => None,
        }
    }

    /// The racer is emulated (see [`AiCar::racer_emulated`]).
    pub fn is_racer_emulated(&self) -> bool {
        self.racer_emulated()
    }

    /// The racer is emulated: the race moves it, and the car does not step.
    pub(super) fn racer_emulated(&self) -> bool {
        matches!(&self.role, Role::Racer(racer) if racer.emulated)
    }

    /// Puts the racer back on the road at `at` (physics space) facing `forward`, rolling at `speed`: it stops being
    /// emulated. `false` when there is no road or ground there.
    pub fn relocate_racer(
        &mut self,
        net: &blackbox_roads::RoadNetwork,
        index: &blackbox_roads::SegmentIndex,
        at: Vec3,
        forward: Vec3,
        speed: f32,
        ground: &crate::scenes::world::drive::WorldGround<'_>,
    ) -> bool {
        let mut nav = blackbox_roads::RoadNav::new(
            blackbox_roads::NavKind::Direction,
            blackbox_roads::LaneType::Racing,
            blackbox_roads::SegmentFilter::default(),
        );
        nav.half_width = self.half.0;
        if !nav.init_at_point(net, index, at, forward, false) {
            return false;
        }
        nav.enable_trail(net);
        let render = crate::scenes::world::space::to_render(nav.position.to_array());
        let facing = crate::scenes::world::space::to_render(nav.forward.to_array());
        let spawn = crate::scenes::world::road::Spawn { position: render, heading: facing.y.atan2(facing.x) };
        if !self.sim.place_traffic_moving(ground, spawn, speed) {
            return false;
        }
        self.nav = nav;
        let pose = self.sim.pose();
        (self.previous, self.current) = (pose, pose);
        self.speed = speed;
        if let Role::Racer(racer) = &mut self.role {
            racer.emulated = false;
        }
        true
    }

    /// A racer held on the grid before GO.
    pub(super) fn racer_held(&self) -> bool {
        matches!(&self.role, Role::Racer(racer) if racer.held)
    }

    /// The car's place in physics space, for the race's gate tests.
    pub fn race_position(&self) -> Vec3 {
        self.physics_position()
    }

    /// The racer's speed, m/s.
    pub fn race_speed(&self) -> f32 {
        self.speed().abs()
    }
}
