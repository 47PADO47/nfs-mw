//! Cop placement, kept separate from the pursuit lifecycle.

use super::*;

impl TrafficWorld {
    pub(in crate::scenes::world::ai) fn spawn_cop(
        &mut self,
        name: &str,
        focus: Focus,
        target: Vec3,
        player: Performance,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let Some(body) = focus.body else { return false };
        if !self.has_cop_model(name) {
            return false;
        }
        let band = Some((MIN_DISTANCE, MAX_DISTANCE));
        let spot = |nav: &RoadNav| CopSpot { at: nav.position, target, speed: REINFORCEMENT_START_SPEED, player, band };
        if let Some(nav) = self.seek_spawn_cursor(&body)
            && self.spawn_cop_on_road(name, spot(&nav), nav, collision, surfaces)
        {
            return true;
        }
        let Some(nav) = self.fallback_spawn_cursor(&body) else { return false };
        self.spawn_cop_on_road(name, spot(&nav), nav, collision, surfaces)
    }

    /// Puts a cop car of `name` on the cop road at `spot.at`, rolling at `spot.speed` and facing `spot.target`. The spawn path
    /// of every cop, the wave's and the support cars' (`ai/support/spawn.rs`). Returns whether the car stands.
    pub(in crate::scenes::world::ai) fn spawn_cop_at(
        &mut self,
        name: &str,
        spot: CopSpot,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let toward = Vec3::new(spot.target.x - spot.at.x, 0.0, spot.target.z - spot.at.z);
        let filter = SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
        if !nav.init_at_point(&self.network, &self.index, spot.at, toward, false) {
            return false;
        }
        self.spawn_cop_on_road(name, spot, nav, collision, surfaces)
    }

    /// Accepts the selected road cursor intact: locating its point again can change its road or undo its reversal.
    pub(super) fn spawn_cop_on_road(
        &mut self,
        name: &str,
        spot: CopSpot,
        mut nav: RoadNav,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let CopSpot { target, speed, player, band, .. } = spot;
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        nav.half_width = model.model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        if self.cops.bars_spawn(nav.position) {
            return false;
        }
        // The window is measured from the camera (`AISpawnManager::GetBasePosition`), the player until the world gives it.
        let gap = self.cops.window_gap(nav.position, target);
        if band.is_some_and(|(near, far)| !(near..=far).contains(&gap)) {
            return false;
        }
        if self.cars.iter().any(|c| c.physics_position().distance(nav.position) < MIN_GAP) {
            return false;
        }
        let render = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        let state = CopState::new(model.car, player, speed);
        let start = Start {
            stagger: (self.rng.police.next_f32() * 8.0) as u32,
            speed,
            cruise: Cruise::default(),
            patrol: false,
        };
        let Some(car) = AiCar::place(
            &model.model,
            Role::Cop(Box::new(state)),
            nav,
            spawn,
            start,
            &WorldGround { collision, surfaces },
        ) else {
            return false;
        };
        log::info!("pursuit: {} at ({:.0}, {:.0}), {:.0} m from the camera", car.name, render.x, render.y, gap);
        self.admit(car);
        self.police.ground_deployed(name);
        true
    }
}
