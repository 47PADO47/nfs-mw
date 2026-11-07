//! Temporary player ownership during a world moment. Reuses the race action and PID driver.
//! Spec: `docs/specs/pursuit-cinematic-control.md`.

use blackbox_driver::{ControllerKind, DriveFlags, DriveRequest, Driver, GearRequest, VehicleView};
use blackbox_roads::{Body, LaneType, NavKind, RoadNav, SegmentFilter};
use blackbox_vehicle::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};
use blackbox_vehicle::performance::Performance;
use nfsmw_data::pursuit::AiVehicle;

use super::TrafficWorld;
use super::car::Ctx;
use super::racer::{RACER_THINK_STEPS, RacerState};
use super::traffic::CarState;
use crate::scenes::world::drive::DriveInput;

pub(in crate::scenes::world) struct MomentDriver {
    nav: RoadNav,
    racer: RacerState,
    driver: Driver,
    ticks: u32,
    elapsed: f32,
    think_elapsed: f32,
    request: DriveRequest,
}

impl TrafficWorld {
    #[cfg(test)]
    pub(in crate::scenes::world) fn moment_input(
        &mut self,
        state: &mut Option<MomentDriver>,
        body: Body,
        view: VehicleView,
        performance: Performance,
        ai: AiVehicle,
        mass: f32,
    ) -> DriveInput {
        self.moment_input_with_delta(crate::scenes::world::drive::STEP, state, body, view, performance, ai, mass)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::scenes::world) fn moment_input_with_delta(
        &mut self,
        dt: f32,
        state: &mut Option<MomentDriver>,
        body: Body,
        view: VehicleView,
        performance: Performance,
        ai: AiVehicle,
        mass: f32,
    ) -> DriveInput {
        if state.is_none() {
            let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
            nav.half_width = body.half_width;
            if !nav.init_at_point(&self.network, &self.index, body.position, view.forward, false) {
                return DriveInput::default();
            }
            nav.enable_trail(&self.network);
            let mut racer = RacerState::new(0, performance, ai, performance, 1.0, 0.5, view.forward_speed);
            racer.held = false;
            // Keep the entry speed until the original non-race autopilot speed rule is established.
            racer.set_cruise_speed(view.planar_speed);
            *state = Some(MomentDriver {
                nav,
                racer,
                driver: Driver::new(ControllerKind::Pid, false),
                ticks: 0,
                elapsed: 0.0,
                think_elapsed: RACER_THINK_STEPS as f32 * dt,
                request: DriveRequest { target: body.position, speed: view.planar_speed, flags: DriveFlags::FULL },
            });
        }
        let state = state.as_mut().expect("the road navigation was acquired");
        if state.ticks.is_multiple_of(RACER_THINK_STEPS) {
            let bodies: Vec<(Body, bool)> = self.cars.iter().map(|car| (car.avoidable(), car.is_cop())).collect();
            let mut ctx = Ctx {
                net: &self.network,
                index: &self.index,
                rng: &mut self.rng.nav,
                bodies: &bodies,
                radii: &[],
                me: usize::MAX,
                player: None,
                aims: &[],
                slotted: &[],
                lights: None,
            };
            let car = CarState {
                body,
                forward_speed: view.forward_speed,
                radius: body.half_width.hypot(body.half_length),
                mass,
                tractor: false,
                stop: None,
            };
            state.racer.set_think_delta(state.think_elapsed);
            state.think_elapsed = 0.0;
            let radii: Vec<f32> = self.cars.iter().map(|car| car.avoid_radius()).collect();
            let neighbours = super::traffic::neighbours(&bodies, &radii, usize::MAX, None);
            let think = state.racer.think(
                &mut ctx,
                &mut state.nav,
                &car,
                neighbours,
                state.ticks.is_multiple_of(4),
                view.forward.y,
            );
            state.request = DriveRequest { target: think.target, speed: think.speed, flags: DriveFlags::FULL };
        }
        let controls = state.driver.step(&view, &state.request, dt, state.elapsed);
        state.ticks += 1;
        state.elapsed += dt;
        state.think_elapsed += dt;
        let gear_select = controls.gear.map(|gear| match gear {
            GearRequest::Reverse => GEAR_REVERSE,
            GearRequest::First => GEAR_FIRST,
            GearRequest::Neutral => GEAR_NEUTRAL,
        });
        DriveInput {
            throttle: controls.gas,
            brake: controls.brake,
            steer: controls.steer,
            handbrake: controls.handbrake > 0.5,
            gear_select,
            ..DriveInput::default()
        }
    }
}
