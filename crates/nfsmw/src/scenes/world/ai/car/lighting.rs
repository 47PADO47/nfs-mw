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

    pub(in crate::scenes::world::ai) fn has_local_headlights(&self) -> bool {
        let state = self.lamp_state();
        state.headlights
            && !state.dark
            && self
                .rig
                .lights()
                .lamps()
                .iter()
                .any(|lamp| lamp.kind == crate::scenes::world::car_lights::Kind::Head && lamp.flags & state.broken == 0)
    }

    pub(in crate::scenes::world::ai) fn local_headlights(
        &self,
        alpha: f32,
        strength: f32,
        out: &mut Vec<blackbox_gfx::LocalLight>,
    ) {
        let state = self.lamp_state();
        if !state.headlights || state.dark {
            return;
        }
        let transform = self.pose(alpha).transform();
        for lamp in self
            .rig
            .lights()
            .lamps()
            .iter()
            .filter(|lamp| lamp.kind == crate::scenes::world::car_lights::Kind::Head && lamp.flags & state.broken == 0)
            .take(2)
        {
            let direction = transform.transform_vector3(lamp.direction).normalize_or_zero();
            out.push(blackbox_gfx::LocalLight {
                position: (transform.transform_point3(lamp.at) + direction * 0.16).to_array(),
                direction: direction.to_array(),
                casts_shadow: true,
                shadow_exclude: None,
                upper_plane: Some(transform.transform_vector3(Vec3::Z).normalize_or_zero().to_array()),
                range: 45.0,
                outer_cos: 0.80,
                inner_cos: 0.94,
                color: (Vec3::new(1.0, 0.96, 0.86) * strength * 1.3).to_array(),
            });
        }
    }

    pub fn flares(&self, alpha: f32, view: FlareView, out: &mut Vec<TexturedEffect>) {
        let state = LampState { time: self.lamp_time(alpha), ..self.lamp_state() };
        let world = self.pose(alpha).transform();
        self.rig.flares().draw(self.rig.lights(), world, state, view, out);
        // A cop's nitrous smears its brake flares along its last poses, as the player's does.
        self.rig.flares().draw_trails(self.rig.lights(), &self.tail_trails, world, state, view, out);
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
        self.rig.lights().signal_glows(self.render_pose(alpha).transform(), state, camera, forward, out);
    }
}
