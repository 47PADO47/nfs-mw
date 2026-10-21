//! A car in a race: what the race reads of it and tells it (`ai/racer.rs`).

use glam::Vec3;

use super::{AiCar, Role};
use crate::scenes::world::ai::racer::RacerState;

impl AiCar {
    /// What a HUD shows of this car: the gauges and its place on the map (split screen's second player).
    pub fn hud_state(&self) -> crate::hud::HudState {
        let t = self.sim.telemetry();
        let pose = self.current;
        let nose = pose.rotation * Vec3::X;
        crate::hud::HudState {
            minimap: Some(crate::scenes::world::split::map_position(pose.position, nose)),
            speed: t.speed_mps.abs(),
            rpm: t.rpm,
            max_rpm: t.max_rpm,
            red_line: t.red_line,
            gear: t.gear,
            shifting: t.shifting,
            shift_light: t.shift_up,
            has_nos: t.has_nos,
            nos: t.nos,
            has_turbo: t.has_induction,
            boost_psi: t.boost_psi,
            ..Default::default()
        }
    }

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

    /// A person drives this racer from now on (split screen): the car takes the player's control set-up, with an
    /// automatic gearbox when `automatic`.
    pub fn seat_person(&mut self, automatic: bool) {
        self.sim.configure_human(automatic);
        if let Role::Racer(racer) = &mut self.role {
            racer.human = Some(crate::scenes::world::drive::DriveInput::default());
        }
    }

    /// The pedals the person holds this frame; `false` when this car has no person.
    pub fn set_pedals(&mut self, pedals: crate::scenes::world::drive::DriveInput) -> bool {
        match &mut self.role {
            Role::Racer(racer) if racer.human.is_some() => {
                racer.human = Some(pedals);
                true
            }
            _ => false,
        }
    }

    /// The pedals of the person driving this racer, when one does (split screen).
    pub(super) fn human_pedals(&self) -> Option<crate::scenes::world::drive::DriveInput> {
        match &self.role {
            Role::Racer(racer) => racer.human,
            _ => None,
        }
    }

    /// What a person's pedals ask of the car this step: the grid holds it before GO, a wreck is not driven.
    pub(super) fn human_input(
        &mut self,
        pedals: crate::scenes::world::drive::DriveInput,
    ) -> crate::scenes::world::drive::DriveInput {
        use crate::scenes::world::drive::DriveInput;
        if self.is_wrecked() {
            return DriveInput { brake: self.wreck_brake(), ..DriveInput::default() };
        }
        if self.racer_held() {
            return DriveInput { brake: 1.0, handbrake: true, ..DriveInput::default() };
        }
        // A shift or reset the person asked for is carried out once, by the first step that sees it.
        if let Role::Racer(racer) = &mut self.role
            && let Some(held) = racer.human.as_mut()
        {
            held.shift_up = false;
            held.shift_down = false;
            held.gear_select = None;
            held.reset = false;
        }
        pedals
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
