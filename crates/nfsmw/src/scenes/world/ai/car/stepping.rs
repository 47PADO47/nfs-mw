//! One physics step of a computer-driven car, in the three parts a traffic step can run apart: the start (the think,
//! the driver and the start of the body's step) and the end (props, walls, damage, the trailer) run in car order on
//! the simulation thread; the integration between them reads only the car's own body and the static collision, so the
//! cars' integrations may run on several threads at once (`ai/parallel.rs`).

use blackbox_collision::CollisionWorld;
use blackbox_driver::{AiControls, DriveFlags, DriveRequest};
use blackbox_roads::RandomSource;
use nfsmw_data::car::physics::SurfaceTable;

use super::super::trailer::Brakes;
use super::super::{cop, racer, traffic};
use super::{AiCar, Ctx, IDLE_SPEED, RECOVERY_SPEED, ROADBLOCK_GOAL, Role, SHOCK_TIME, shift};
use crate::scenes::world::drive::{DriveInput, Integration, STEP, StepStart, WorldGround};
use crate::scenes::world::props::PropWorld;

impl AiCar {
    /// One physics step: think when it is this car's turn, then drive.
    pub fn step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let Some(start) = self.begin_step(ctx, props, collision) else { return };
        self.sim.integration().run(&start, &WorldGround { collision, surfaces }, Some(collision));
        self.finish_step(&start, collision, props, surfaces);
    }

    /// The start of [`Self::step`]: the think, the driver and the start of the body's step. `None` when the car does
    /// not step (a racer on its stand-in moves by `stand_in.rs` instead).
    pub(in crate::scenes::world::ai) fn begin_step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        props: &mut PropWorld,
        collision: &CollisionWorld,
    ) -> Option<StepStart> {
        if self.stand_in.active {
            return None;
        }
        self.sim.collision_goal =
            if self.roadblock && !self.is_wrecked() { Some(ROADBLOCK_GOAL) } else { self.goal_drive().collision_goal };
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
        self.shock = (self.shock - STEP / SHOCK_TIME).max(0.0);
        let reversing = self.driver.reverse_override_left() > 0.0;
        let flags = match (&self.role, reversing, self.searching) {
            (Role::Cop(_) | Role::Racer(_), false, false) => DriveFlags::FULL,
            _ => DriveFlags::SIMPLE,
        };
        let wanted = if reversing { RECOVERY_SPEED } else { self.speed };
        let target = self.target + self.avoid.steer_offset(&self.nav, view.position);
        let request = DriveRequest { target, speed: wanted, flags };
        let clock = self.steps as f32 * STEP;
        let held = self.controls.steer;
        if human.is_none() {
            self.controls = self.driver.step(&view, &request, STEP, clock);
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
        if airborne_action && !reversing && self.airborne > 1.5 {
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
        Some(self.sim.begin_step(&input, true, Some(props)))
    }

    /// The integration of [`Self::step`]: the part that may run on another thread.
    pub(in crate::scenes::world::ai) fn integration(&mut self) -> Integration<'_> {
        self.sim.integration()
    }

    /// The end of [`Self::step`], after the integration: props, walls, damage, the trailer and the car's clocks.
    pub(in crate::scenes::world::ai) fn finish_step(
        &mut self,
        start: &StepStart,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let impact = self.sim.end_step(start, &WorldGround { collision, surfaces }, Some((collision, props)));
        for hit in impact.damage {
            self.wreck_hit(hit, self.damage_distance);
        }
        self.avoid.advance(self.speed().abs() * STEP);
        if let Some(trailer) = self.trailer.as_mut() {
            let brakes = Brakes { brake: self.controls.brake, handbrake: self.controls.handbrake > 0.5 };
            trailer.step(&mut self.sim, brakes, collision, props, surfaces);
        }
        self.previous = self.current;
        self.current = self.sim.pose();
        self.wreck_step(STEP);
        self.steps += 1;
        self.airborne = match self.sim.telemetry().wheels_on_ground == 0 {
            true => self.airborne + STEP,
            false => 0.0,
        };
        self.idle = match self.sim.telemetry().speed_mps.abs() < IDLE_SPEED && !self.waiting {
            true => self.idle + STEP,
            false => 0.0,
        };
    }
}
