//! The helicopter's rotor wash: the `heliwash` particle group of the install (three emitters) run on the ground under
//! the helicopter. Spec: `docs/specs/ai-helicopter.md` section 7.
//!
//! Every tick the effect is placed on the ground at the helicopter's horizontal position, its `z` axis along the
//! ground's normal, with the intensity `1 - ramp(altitude, 5, 30)`. Above 30 m, or with no ground under it, it is
//! paused. The three emitters are discs: they blow out across the ground, not up along the normal.

use blackbox_attrib::Database;
use blackbox_collision::CollisionWorld;
use blackbox_gfx::RenderBackend;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::{Mat4, Quat, Vec3};
use nfsmw_data::world::explosion_fx::heli_wash_emitters;

use super::explosion_fx::{Set, build_set, draw};
use super::space;

/// The altitude (metres above the ground) where the wash starts to fade, and where it is gone.
const FULL: f32 = 5.0;
const GONE: f32 = 30.0;
/// Most live particles of one emitter.
const LIMIT: usize = 64;
/// How far down the ground is looked for, metres.
const REACH: f32 = 300.0;

/// The wash's intensity at `altitude` metres above the ground: 1 up to 5 m, fading to 0 at 30 m. `None` above 30 m:
/// the effect is paused there.
pub fn intensity(altitude: f32) -> Option<f32> {
    if altitude.is_nan() || altitude > GONE {
        return None;
    }
    let ramp = ((altitude - FULL) / (GONE - FULL)).clamp(0.0, 1.0);
    Some(1.0 - ramp)
}

/// The effect's frame (render space) at `ground` (physics space) on ground with `normal` (physics space): the shortest
/// turn from the up axis to the upward normal, so on flat ground it is unturned and its `x`/`y` plane is the ground.
fn wash_frame(ground: Vec3, normal: Vec3) -> Mat4 {
    let up = match normal.y < 0.0 {
        true => -normal,
        false => normal,
    };
    let tilt = Quat::from_rotation_arc(Vec3::Z, space::to_render(up.to_array()).normalize_or(Vec3::Z));
    Mat4::from_rotation_translation(tilt, space::to_render(ground.to_array()))
}

enum Load {
    Unloaded,
    Missing,
    Ready(Set),
}

pub struct HeliWash {
    load: Load,
    emitters: Vec<Emitter>,
    /// Where the effect stands (render space, z axis along the ground normal), and the intensity of its last update.
    frame: Mat4,
    intensity: f32,
}

impl Default for HeliWash {
    fn default() -> Self {
        Self { load: Load::Unloaded, emitters: Vec::new(), frame: Mat4::IDENTITY, intensity: 0.0 }
    }
}

impl HeliWash {
    /// Uploads the effect's textures once; with no art (or no effect in the install) it stays off.
    pub fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database) {
        if !matches!(self.load, Load::Unloaded) {
            return;
        }
        let emitters = heli_wash_emitters(db);
        match build_set(renderer, dir, emitters) {
            Some(set) => {
                self.emitters = set
                    .templates
                    .iter()
                    .enumerate()
                    .map(|(i, (_, spec))| {
                        let mut emitter = Emitter::new(spec.clone(), 0x4845_4C49 + i as u64);
                        emitter.set_limit(LIMIT);
                        emitter
                    })
                    .collect();
                self.load = Load::Ready(set);
            }
            None => {
                log::info!("no helicopter wash effect in the install");
                self.load = Load::Missing;
            }
        }
    }

    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.emitters.clear();
        if let Load::Ready(set) = std::mem::replace(&mut self.load, Load::Missing) {
            set.destroy(renderer);
        }
    }

    /// One fixed physics tick. `helicopter` is the helicopter's position (physics space) while one flies.
    pub fn step(&mut self, dt: f32, helicopter: Option<Vec3>, collision: &CollisionWorld) {
        if !matches!(self.load, Load::Ready(_)) {
            return;
        }
        match helicopter {
            Some(at) => self.update(at, collision),
            None => self.intensity = 0.0,
        }
        let frame = Frame { to_world: self.frame, inherit_velocity: Vec3::ZERO, intensity: self.intensity };
        for emitter in &mut self.emitters {
            emitter.set_enabled(self.intensity > 0.0);
            emitter.step(dt, &frame);
        }
    }

    /// Place the effect on the ground under `at` and set its intensity.
    fn update(&mut self, at: Vec3, collision: &CollisionWorld) {
        let Some(hit) = collision.ground(at.x, at.z, at.y, at.y - REACH) else {
            self.intensity = 0.0;
            return;
        };
        let Some(strength) = intensity(at.y - hit.point[1]) else {
            self.intensity = 0.0;
            return;
        };
        self.frame = wash_frame(Vec3::new(at.x, hit.point[1], at.z), Vec3::from_array(hit.normal));
        self.intensity = strength;
    }

    /// Appends one textured batch per texture with live particles.
    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let Load::Ready(set) = &self.load else { return };
        draw(set, std::iter::once(self.emitters.as_slice()), camera, forward, out);
    }

    #[cfg(test)]
    pub fn current_intensity(&self) -> f32 {
        self.intensity
    }
}

#[cfg(test)]
#[path = "heli_wash_tests.rs"]
mod tests;
