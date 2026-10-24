//! Shared last observation, used by every pursuit driver when visual contact is lost.

use blackbox_roads::Body;
use glam::Vec3;

use super::TrafficWorld;

impl TrafficWorld {
    pub(super) fn pursuit_target(&mut self, actual: Option<Body>) -> Option<Body> {
        let pursuit = self.pursuit.as_mut()?;
        if self.police.has_contact() {
            pursuit.last_seen = actual;
        }
        if self.police.is_perp_in_sight() {
            return actual;
        }
        let mut remembered = pursuit.last_seen?;
        remembered.velocity = Vec3::ZERO;
        Some(remembered)
    }
}

impl TrafficWorld {
    /// Cooldown replacements enter on traffic lanes and already use the search action. The annular sampler is a
    /// rewrite approximation of the original camera-based gathered-segment cache.
    pub(super) fn spawn_search_cop(
        &mut self,
        name: &str,
        focus: super::Focus,
        performance: blackbox_vehicle::performance::Performance,
        collision: &blackbox_collision::CollisionWorld,
        surfaces: &nfsmw_data::car::physics::SurfaceTable,
    ) -> bool {
        use super::car::{AiCar, Role, Start};
        use super::cop::CopState;
        use crate::scenes::world::{drive::WorldGround, road::Spawn, space};
        use blackbox_roads::{RandomSource, RoadNav};

        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        let player = focus.body.map_or(Vec3::ZERO, |body| body.position);
        let centre = self.cops.base(player);
        let distance = 150.0 + 250.0 * self.rng.police.next_f32();
        let angle = std::f32::consts::TAU * self.rng.police.next_f32();
        let at = centre + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance);
        let direction = Vec3::new(angle.sin(), 0.0, -angle.cos());
        let mut nav = RoadNav::traffic();
        if !nav.init_at_point(&self.network, &self.index, at, direction, false)
            || !nav.can_traffic_spawn(&self.network, &mut self.rng.police)
        {
            return false;
        }
        nav.half_width = model.model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        let gap = self.cops.window_gap(nav.position, player);
        if self.cops.bars_spawn(nav.position)
            || !(150.0..=400.0).contains(&gap)
            || !super::spawn::cop_lane_clear(&self.cars, &nav)
        {
            return false;
        }
        let position = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position, heading: facing.y.atan2(facing.x) };
        let cruise = self.search_cruise();
        let speed = cruise.street.min(cruise.highway);
        let start = Start { stagger: (self.rng.police.next_f32() * 8.0) as u32, speed, cruise, patrol: false };
        let Some(mut car) = AiCar::place(
            &model.model,
            Role::Cop(Box::new(CopState::new(model.car, performance, speed))),
            nav,
            spawn,
            start,
            &WorldGround { collision, surfaces },
        ) else {
            return false;
        };
        car.search_mode(true, cruise, &self.network, &self.index);
        if !car.is_searching() {
            return false;
        }
        log::info!("pursuit: {name} joins road search, {gap:.0} m from the player");
        self.admit(car);
        true
    }
}
