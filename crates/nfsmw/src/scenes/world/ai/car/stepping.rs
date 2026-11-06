//! AI thinking and body integration precede shared pairs. All body contacts finish before the response pass,
//! which reads only each car and static terrain and may run on worker threads (`ai/parallel.rs`).

use blackbox_collision::CollisionWorld;
use blackbox_driver::{AiControls, DriveFlags, DriveRequest};
use blackbox_roads::RandomSource;
use nfsmw_data::car::physics::SurfaceTable;

use super::super::trailer::Brakes;
use super::super::{cop, racer, traffic};
use super::{AiCar, Ctx, IDLE_SPEED, RECOVERY_SPEED, ROADBLOCK_GOAL, Role, SHOCK_TIME, shift};
use crate::scenes::world::drive::{DriveInput, Integration, StepStart, WorldGround};
use crate::scenes::world::props::PropWorld;

/// Prepared inputs retained across the world's shared body and response barriers.
#[derive(Clone, Copy)]
pub(in crate::scenes::world::ai) struct AiStep {
    pub car: StepStart,
    pub trailer: Option<StepStart>,
}

impl AiCar {
    /// One physics step: think when it is this car's turn, then drive.
    #[cfg(test)]
    pub fn step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let Some(start) = self.begin_step(ctx, props, collision) else { return };
        self.finish_body(&start, collision, props, surfaces);
        self.resolve_articulation();
        self.sim.integration().run(&start.car, &WorldGround { collision, surfaces }, Some(collision));
        self.finish_step(&start, collision, props, surfaces);
    }

    /// The start of [`Self::step`]: the think, the driver and the start of the body's step. `None` when the car does
    /// not step (a racer on its stand-in moves by `stand_in.rs` instead).
    pub(in crate::scenes::world::ai) fn begin_step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        props: &mut PropWorld,
        collision: &CollisionWorld,
    ) -> Option<AiStep> {
        if self.stand_in.active {
            return None;
        }
        self.think_elapsed += self.step_delta;
        self.sim.collision_goal =
            if self.roadblock && !self.is_wrecked() { Some(ROADBLOCK_GOAL) } else { self.goal_drive().collision_goal };
        // Distant cars are drawn with their front wheels straight (`drive/sim/wheel_visuals.rs`).
        self.sim.set_steer_visible(self.damage_distance <= crate::scenes::world::drive::STEER_DRAW_DISTANCE);
        let think_steps = match self.role {
            Role::Traffic => traffic::THINK_STEPS,
            Role::Cop(_) => cop::THINK_STEPS,
            Role::Racer(_) => racer::RACER_THINK_STEPS,
        };
        let human = self.human_pedals();
        if human.is_none() && !self.is_wrecked() && (self.steps + self.stagger).is_multiple_of(think_steps) {
            self.think(ctx, collision);
            if matches!(self.role, Role::Cop(_)) {
                self.queue_sight_task();
            }
        }
        let mut view = self.sim.driver_view();
        view.in_shock = self.shock > 0.0;
        self.shock = (self.shock - self.step_delta / SHOCK_TIME).max(0.0);
        let reversing = self.driver.reverse_override_left() > 0.0;
        let flags = match (&self.role, reversing, self.searching) {
            (Role::Cop(_) | Role::Racer(_), false, false) => DriveFlags::FULL,
            _ => DriveFlags::SIMPLE,
        };
        let wanted = if reversing { RECOVERY_SPEED } else { self.speed };
        if human.is_none() {
            self.refresh_traffic_target(ctx);
        }
        let target = self.target;
        let request = DriveRequest { target, speed: wanted, flags };
        let clock = self.elapsed;
        let held = self.controls.steer;
        if human.is_none() {
            self.controls = self.driver.step(&view, &request, self.step_delta, clock);
            if let super::Role::Cop(cop) = &self.role {
                self.controls.nos = cop.nitrous()
                    && !self.searching
                    && !self.fleeing
                    && self.goal_drive().command().is_none()
                    && !reversing
                    && self.sim.telemetry().has_nos;
            }
        }
        if let Some(controls) = self.accident.controls(self.tractor, held) {
            self.controls = controls;
        }
        self.hold_goal_brake();
        let airborne_action = matches!(&self.role, super::Role::Cop(cop) if !cop.close_active()
            && !(cop.ram_allowed && self.goal_drive().command().is_some()));
        if airborne_releases(airborne_action, reversing, self.airborne, self.sim.body_on_ground()) {
            self.controls = AiControls::default();
        }
        if self.breaker_stop {
            self.controls = AiControls { brake: 1.0, ..AiControls::default() };
        }
        if self.is_wrecked() {
            self.arrived = false;
            self.controls = AiControls { brake: self.wreck_brake(), ..AiControls::default() };
        }
        if (self.arrested || self.racer_held()) && !self.is_wrecked() {
            self.controls = AiControls { brake: 1.0, handbrake: 1.0, ..AiControls::default() };
        }
        shift(&mut self.sim, self.controls.gear);
        let input = match human {
            Some(pedals) => self.human_input(pedals),
            None => DriveInput {
                throttle: self.controls.gas,
                brake: self.controls.brake,
                steer: self.controls.steer,
                handbrake: self.controls.handbrake > 0.5 || self.arrived,
                nos: self.controls.nos,
                ..DriveInput::default()
            },
        };
        let destroyed = self.is_wrecked();
        self.sim.set_destroyed_non_player(destroyed);
        let car = self.sim.begin_step_with_delta(self.step_delta, &input, true, Some(props));
        self.sim.begin_body(&car, Some(collision));
        let brakes = Brakes { brake: self.controls.brake, handbrake: self.controls.handbrake > 0.5 };
        let trailer =
            self.trailer.as_mut().map(|trailer| trailer.begin_step(self.step_delta, brakes, collision, props));
        Some(AiStep { car, trailer })
    }

    /// Finish every body's contacts before any vehicle produces the next tick's forces.
    pub(in crate::scenes::world::ai) fn finish_body(
        &mut self,
        start: &AiStep,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let ground = WorldGround { collision, surfaces };
        self.sim.finish_body(&start.car, &ground, Some((collision, &mut *props)));
        if let (Some(trailer), Some(start)) = (self.trailer.as_mut(), start.trailer.as_ref()) {
            trailer.sim_mut().finish_body(start, &ground, Some((collision, props)));
        }
    }

    /// The integration of [`Self::step`]: the part that may run on another thread.
    pub(in crate::scenes::world::ai) fn integration(&mut self) -> Integration<'_> {
        self.sim.integration()
    }

    /// Constraints consume completed body collisions before engine and suspension response.
    pub(in crate::scenes::world::ai) fn resolve_articulation(&mut self) {
        if let Some(trailer) = self.trailer.as_mut() {
            trailer.resolve_articulation(&mut self.sim);
        }
    }

    /// The end of [`Self::step`], after the integration: props, walls, damage, the trailer and the car's clocks.
    pub(in crate::scenes::world::ai) fn finish_step(
        &mut self,
        start: &AiStep,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let impact =
            self.sim.end_step(&start.car, &WorldGround { collision, surfaces }, Some((collision, &mut *props)));
        for hit in impact.damage {
            self.wreck_hit(hit, self.damage_distance);
        }
        if let (Some(trailer), Some(start)) = (self.trailer.as_mut(), start.trailer.as_ref()) {
            trailer.complete_step(start, &mut self.sim, collision, props, surfaces);
        }
        self.previous = self.current;
        self.current = self.sim.pose();
        let nitrous = self.sim.telemetry().nos_burning;
        self.tail_trails.step(self.current.transform(), nitrous, self.sim.velocity().length());
        self.wreck_step(self.step_delta);
        self.steps += 1;
        self.elapsed += self.step_delta;
        self.airborne = match self.sim.telemetry().wheels_on_ground == 0 {
            true => self.airborne + self.step_delta,
            false => 0.0,
        };
        self.idle = match self.sim.telemetry().speed_mps.abs() < IDLE_SPEED && !self.waiting {
            true => self.idle + self.step_delta,
            false => 0.0,
        };
    }
}

/// Seconds with no wheel on the ground before the airborne action releases a cop's controls (`ai-driver-control.md`
/// section 7.2).
const AIRBORNE_SECS: f32 = 1.5;

/// Whether the airborne action holds a cop's controls at zero this step: the cop may take it (`eligible`), no stuck
/// reverse runs, and it has had no wheel down for [`AIRBORNE_SECS`]. **[ours]** Not while its body rests on the
/// ground: a cop beached on a kerb or a bank with its wheels hanging would sit there for good, as its body sleeps and
/// only the throttle wakes it, and the stuck test only counts while the gas is pressed.
fn airborne_releases(eligible: bool, reversing: bool, airborne: f32, body_on_ground: bool) -> bool {
    eligible && !reversing && !body_on_ground && airborne > AIRBORNE_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_airborne_action_releases_a_flying_cop_after_a_second_and_a_half() {
        assert!(!airborne_releases(true, false, 1.4, false));
        assert!(airborne_releases(true, false, 1.6, false));
        assert!(!airborne_releases(true, true, 5.0, false), "a stuck reverse keeps its controls");
        assert!(!airborne_releases(false, false, 5.0, false), "a closing or ramming cop keeps its controls");
    }

    #[test]
    fn a_cop_beached_on_its_body_keeps_its_controls() {
        assert!(!airborne_releases(true, false, 60.0, true), "it keeps pressing the gas, so it wakes and can reverse");
    }
}
