//! What the cinematic moment of a pursuit breaker asks of the police: the cops chasing the player, and a hold on the
//! busted meter while the cut plays. Spec: `docs/specs/pursuit-breaker-moment.md`.

use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, SplitMix};
use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::space;

impl TrafficWorld {
    /// A world moment holds new placements while allowing existing cars to retire and timers to advance.
    pub(in crate::scenes::world) fn hold_world_moment(&mut self, active: bool) {
        self.world_moment = active;
    }

    /// The ground cops chasing the player (not wrecked), render space.
    pub fn pursuer_positions(&self) -> Vec<Vec3> {
        self.cars.iter().filter(|car| car.is_cop()).map(|car| car.position()).collect()
    }

    /// The game camera the helicopter keeps out of (`AvoidCamera`), render space; `None` clears it.
    pub fn set_heli_camera(&mut self, camera: Option<(Vec3, Vec3)>) {
        self.police.set_heli_camera(camera.map(|(position, forward)| blackbox_pursuit::helicopter::camera::Camera {
            position: Vec3::from_array(crate::scenes::world::space::to_physics(position)),
            forward: Vec3::from_array(crate::scenes::world::space::to_physics(forward)),
            velocity: Vec3::ZERO,
        }));
    }

    /// Holds the busted meter where it is while a cinematic moment plays: it neither fills nor drains.
    pub fn hold_busted(&mut self, hold: bool) {
        self.police.hold_busted(hold);
    }

    /// The cops chasing the player that see the player now, render space: the ones the pursuit-start shot may look at
    /// (`docs/specs/game-moment-camera.md` section 4).
    pub fn cops_seeing_player(&self) -> Vec<Vec3> {
        self.cars.iter().filter(|car| car.is_cop() && car.sees_player()).map(|car| car.position()).collect()
    }

    /// Every other car on the road that is not wrecked, render space: the cars a crash shot looks out for.
    pub fn live_car_positions(&self) -> Vec<Vec3> {
        self.cars.iter().filter(|car| !car.is_wrecked()).map(|car| car.position()).collect()
    }

    /// How far the nearest roadblock car is from `at` (render space), if a roadblock stands.
    pub fn nearest_roadblock_car(&self, at: Vec3) -> Option<f32> {
        let cars = self.cars.iter().filter(|car| car.is_roadblock() && !car.is_wrecked());
        cars.map(|car| car.position().distance(at)).min_by(f32::total_cmp)
    }

    /// The road `ahead` metres along from `at` heading `forward` (render space): the point on it and its drivable left
    /// and right edges, for the car shot's eye. Junctions are taken by a fixed draw.
    pub fn road_ahead(&self, at: Vec3, forward: Vec3, ahead: f32) -> Option<(Vec3, [Vec3; 2])> {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        let position = Vec3::from_array(space::to_physics(at));
        let heading = Vec3::from_array(space::to_physics(forward));
        if !nav.init_at_point(&self.network, &self.index, position, heading, true) {
            return None;
        }
        nav.advance(&self.network, ahead, Vec3::ZERO, &mut SplitMix(0));
        let render = |p: Vec3| space::to_render(p.to_array());
        let [left, right] = nav.road_edges(&self.network);
        Some((render(nav.position), [render(left), render(right)]))
    }
}
