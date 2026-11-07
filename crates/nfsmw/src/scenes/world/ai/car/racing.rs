//! A car in a race: what the race reads of it and tells it (`ai/racer.rs`).

use glam::Vec3;

use super::{AiCar, Role};
use crate::scenes::world::ai::racer::RacerState;

impl AiCar {
    pub(in crate::scenes::world::ai) fn race_uses_physics(&self) -> bool {
        !self.stand_in.active
    }

    /// The elapsed interval only on a tick where this car made a racer decision.
    pub(in crate::scenes::world::ai) fn racer_decision_interval(&self) -> Option<f32> {
        (self.has_thought && self.think_elapsed == 0.0 && self.is_racer()).then_some(self.think_delta)
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn race_power_diagnostic(&self) -> String {
        self.sim.race_power_diagnostic()
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn race_diagnostic(&self) -> String {
        format!(
            "position={:?} speed={} target={:?} wanted={} stand_in={:?} segment={} direction={} nav_position={:?} curvature={} dead_end={} path={:?} controls={:?} think_delta={}",
            self.race_position(),
            self.race_speed(),
            self.target,
            self.speed,
            self.stand_in,
            self.nav.segment,
            self.nav.node_ind,
            self.nav.position,
            self.nav.curvature,
            self.nav.dead_end,
            self.nav.path,
            self.controls,
            self.think_delta
        )
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn race_terrain_diagnostic(&self, collision: &blackbox_collision::CollisionWorld) {
        use crate::scenes::world::drive::drivable_ground_below;
        let ground = |at: Vec3| drivable_ground_below(collision, at.x, at.z, at.y + 20.0, at.y - 60.0);
        eprintln!(
            "RACER_TERRAIN car={:?} target={:?} cursor={:?}",
            ground(self.race_position()),
            ground(self.target),
            ground(self.nav.position)
        );
        let from = self.race_position();
        let to = Vec3::new(self.target.x, from.y, self.target.z);
        let options = blackbox_collision::RayOptions { skip_groups: true, ..Default::default() };
        let wall = collision.ray_cast_filtered(from.to_array(), to.to_array(), &options, |hit| {
            hit.kind == blackbox_collision::HitKind::Barrier || hit.normal[1].abs() < 0.65
        });
        eprintln!("RACER_WALL {wall:?}");
        if let Some(trail) = self.nav.trail() {
            for cookie in trail.cookies() {
                eprintln!("RACER_COOKIE {cookie:?}");
            }
        }
    }

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
            telemetry: self.sim.hud_telemetry(),
            condition: Some(self.sim.hud_condition()),
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
