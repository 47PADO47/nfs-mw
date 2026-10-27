//! AI actors use the shared authored vehicle flares; optional indicators stay separate.

use super::AiCar;
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::car_lights::LampState;
use blackbox_render::{EffectVertex, TexturedEffect};
use glam::Vec3;

impl AiCar {
    pub(in crate::scenes::world::ai) fn lamp_state(&self) -> LampState {
        LampState {
            broken: self.sim.lamp_damage(),
            emergency: match self.emergency_lights() {
                true => 0x7000,
                false => 0,
            },
            time: self.lamp_time(1.0),
            brake: self.sim.lamp_brake(),
            reverse: self.sim.telemetry().gear < 0,
            headlights: true,
            enhanced: self.rig.enhanced_lamps(),
            ..Default::default()
        }
    }

    pub fn flares(&self, alpha: f32, view: FlareView, out: &mut Vec<TexturedEffect>) {
        let state = LampState { time: self.lamp_time(alpha), ..self.lamp_state() };
        self.rig.flares().draw(self.rig.lights(), self.pose(alpha).transform(), state, view, out);
        if let Some(trailer) = &self.trailer {
            trailer.flares(alpha, view, out);
        }
    }

    pub(in crate::scenes::world::ai) fn lamp_time(&self, alpha: f32) -> f32 {
        if self.steps == 0 {
            return self.light_phase;
        }
        (self.elapsed - self.step_delta + alpha.clamp(0.0, 1.0) * self.step_delta) + self.light_phase
    }

    pub fn glows(
        &self,
        alpha: f32,
        time: f32,
        turn_signals: bool,
        camera: Vec3,
        forward: Vec3,
        out: &mut Vec<EffectVertex>,
    ) {
        let state = LampState { steer: self.controls.steer, turn_signals, time, ..self.lamp_state() };
        self.rig.lights().signal_glows(self.pose(alpha).transform(), state, camera, forward, out);
    }
}
