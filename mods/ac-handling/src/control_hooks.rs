//! The three hook entry points of [`Control`]: one wheel of the tyre hook, one axle of the suspension hook, one tick of the
//! controls hook. Each answers "not handled" for every role but the player's, while its hook is not registered, with no model
//! yet, and on any value the model may not answer for (the model's own checks), and never sends the host a number it would
//! refuse.

use blackbox_modding::sdk::{
    ControlsQuery, ControlsResult, Role, SuspensionQuery, SuspensionResult, TyreQuery, TyreResult,
};

use crate::bridge::{to_axle_input, to_axle_result, to_input, to_result};
use crate::control::{Control, DEFAULT_AMBIENT_C};
use crate::controls::{ControlsCfg, Feed, controls_step};
use crate::grip::{GripInputs, sanitize};
use crate::settings::Unit;
use crate::suspension::axle_call;
use crate::thermal::ThermalTick;
use crate::wheel::{WheelState, wheel_tick};

impl Control {
    /// Note a car change seen in a query: keep the old parameters for this tick and ask the next frame start to read the spec.
    fn check_revision(&mut self, revision: u32) {
        let Some(model) = self.model.as_ref() else {
            return;
        };
        if revision != model.params.revision {
            self.needs_spec = true;
        }
    }

    /// The tyre hook: one wheel, one tick. `None` is "not handled" (every other role, no model yet, a refused answer).
    pub fn tyre(&mut self, q: &TyreQuery) -> Option<TyreResult> {
        if q.role != Some(Role::Player) || !self.applied.tyre {
            return None;
        }
        if self.model.is_none() {
            self.needs_spec = true;
            return None;
        }
        self.check_revision(q.vehicle_revision);
        let wheel = q.wheel_index as usize;
        if wheel > 3 {
            return None;
        }
        if q.ambient_temperature_c.is_finite() {
            self.ambient_c = f64::from(q.ambient_temperature_c);
        }
        let road = if q.road_temperature_c.is_finite() { f64::from(q.road_temperature_c) } else { 25.0 };
        let (ambient, grip_gain, substeps) = (self.ambient_c, self.grip_gain, self.substeps);
        let model = self.model.as_mut()?;
        let camber = if q.camber_rad.is_finite() { f64::from(q.camber_rad) } else { 0.0 };
        model.camber[wheel] = camber;
        let thermal = sanitize(model.thermal.grip_scale(wheel, &GripInputs { camber_rad: camber }));
        let condition = f64::from(q.tyre_condition).clamp(0.0, 1.0);
        let input = to_input(q, grip_gain * condition * thermal);
        let state = &mut model.wheels[wheel];
        let output = wheel_tick(model.params.axle(wheel), state, &input, substeps)?;
        let result = to_result(&output);
        if result.is_none() {
            *state = WheelState::fresh();
            return None;
        }
        model.thermal.end_tick(
            wheel,
            &ThermalTick {
                dt: f64::from(q.dt),
                p_slide: output.p_slide,
                v_slide: output.v_slide,
                fz: f64::from(q.vertical_load_n),
                v: f64::from(q.body_speed).abs(),
                gamma: camber,
                t_amb: if q.ambient_temperature_c.is_finite() { ambient } else { DEFAULT_AMBIENT_C },
                t_road: road,
                loaded: q.loaded,
            },
        );
        if wheel < 2 {
            self.last_mz[wheel] = output.mz;
            self.last_omega[wheel] = output.spin;
        }
        if wheel == 0 {
            self.tick_clock(f64::from(q.dt));
        }
        result
    }

    /// The suspension hook: one axle, one tick. `None` is "not handled": the host's own spring runs for both wheels.
    pub fn suspension(&mut self, q: &SuspensionQuery) -> Option<SuspensionResult> {
        if q.role != Some(Role::Player) || !self.applied.suspension {
            return None;
        }
        if self.model.is_none() {
            self.needs_spec = true;
            return None;
        }
        self.check_revision(q.vehicle_revision);
        let axle = q.axle as usize;
        if axle > 1 {
            return None;
        }
        let input = to_axle_input(q);
        if axle == 0 {
            self.front_steer = [input.wheels[0].steer, input.wheels[1].steer];
        }
        let model = self.model.as_mut()?;
        let out = axle_call(&model.axle_cfg[axle], &mut model.axles[axle], &input)?;
        model.camber[2 * axle] = out[0].camber;
        model.camber[2 * axle + 1] = out[1].camber;
        Some(to_axle_result(&out))
    }

    /// The controls hook: one tick. `None` is "not handled": the controls stay as the host shaped them.
    pub fn controls(&mut self, q: &ControlsQuery) -> Option<ControlsResult> {
        if q.role != Some(Role::Player) || !self.applied.controls {
            return None;
        }
        if self.model.is_none() {
            self.needs_spec = true;
            return None;
        }
        self.check_revision(q.vehicle_revision);
        let aids = self.unit_on(Unit::Aids);
        let steering = self.unit_on(Unit::Steering);
        let feed = Feed { mz: self.last_mz, omega: self.last_omega, seed_delta: self.seed_delta() };
        let device = self.device;
        let (sens, gain, curve) = (self.steer_sens, self.ffb_gain, self.ffb_curve);
        let levels = self.levels;
        let model = self.model.as_mut()?;
        let mut keys = model.params.steering.clone();
        keys.s_sens = sens.map_or(keys.s_sens, |value| value.clamp(0.0, 1.0));
        keys.g_ffb = gain.map_or(keys.g_ffb, |value| value.clamp(0.0, 3.0));
        keys.p_curve = curve.map_or(keys.p_curve, |value| value.clamp(0.4, 1.5));
        let cfg = ControlsCfg {
            aids,
            steering,
            levels: levels.over(model.params.units.levels),
            car: model.aid_car,
            steer_keys: keys,
            device,
        };
        controls_step(&cfg, &mut model.controls, q, &feed)
    }

    /// The angle the steering model starts from: the last suspension call's inside front wheel, 0 without one.
    fn seed_delta(&self) -> f64 {
        if !self.applied.suspension {
            return 0.0;
        }
        let [left, right] = self.front_steer;
        if left.abs() >= right.abs() { left } else { right }
    }
}
