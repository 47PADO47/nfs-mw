//! The rain splash of a mod's variant (`docs/modding/effect-variants-marks-rain.md`): splashes where the rain meets the road.
//!
//! The game's own rain makes no splash on the road. With a variant chosen, while it rains and the player is not under cover,
//! splashes are made at random places of the road around the player (a ground query for each, a few a frame at most), at the
//! rate the profile's layers say times how hard it rains, and each lives for its short life drawn with the layer's picture: a ring
//! that lies on the road and grows, or a sprite that stands up from the road facing the camera. Nothing is allocated per splash
//! once the pool is full; with no variant chosen nothing here is loaded, made or drawn.

use std::sync::Arc;

use blackbox_collision::CollisionWorld;
use blackbox_gfx::RenderBackend;
use blackbox_modding::content::{RainSplashProfile, SplashKind};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::Vec3;

use crate::scenes::world::exhaust::variant::upload_png;
use crate::scenes::world::space;

/// Most live splashes.
const POOL: usize = 384;
/// Most splashes made in one frame (each costs a ground query).
const PER_FRAME: usize = 8;
/// How far above and below the player the ground is looked for, metres.
const REACH: [f32; 2] = [3.0, 6.0];
/// How far above the road a ring lies, metres.
const LIFT: f32 = 0.03;

#[derive(Clone, Copy)]
struct Splash {
    layer: u8,
    at: Vec3,
    age: f32,
    life: f32,
    cell: u8,
    /// A turn of the ring about the road's normal, radians.
    turn: f32,
}

struct Loaded {
    profile: Arc<RainSplashProfile>,
    textures: Vec<Option<(TextureHandle, BlendMode)>>,
    batches: Vec<Vec<EffectVertex>>,
    handed: Vec<(usize, TextureHandle, usize)>,
}

/// The rain splash of the scene.
#[derive(Default)]
pub(in crate::scenes::world) struct RainSplash {
    wanted: Option<Arc<RainSplashProfile>>,
    loaded: Option<Loaded>,
    stale: Vec<TextureHandle>,
    splashes: Vec<Splash>,
    /// The splashes each layer is owed, fractions carried over.
    credit: Vec<f32>,
    seed: u32,
}

impl RainSplash {
    pub(in crate::scenes::world) fn set_wanted(&mut self, profile: Option<Arc<RainSplashProfile>>) {
        self.wanted = profile;
    }

    fn needs_load(&self) -> bool {
        match (&self.wanted, &self.loaded) {
            (None, None) => false,
            (Some(want), Some(have)) => !Arc::ptr_eq(want, &have.profile) && **want != *have.profile,
            _ => true,
        }
    }

    /// Load the pictures when the variant changed, or drop them when it went (and the splashes with them).
    pub(in crate::scenes::world) fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
        if !self.needs_load() {
            return;
        }
        if let Some(loaded) = self.loaded.take() {
            self.stale.extend(loaded.textures.iter().flatten().map(|(handle, _)| *handle));
        }
        self.splashes.clear();
        let Some(profile) = self.wanted.clone() else { return };
        let textures: Vec<_> = profile
            .layers
            .iter()
            .map(|layer| upload_png(renderer, dir, &layer.texture, layer.additive, "rain splash variant"))
            .collect();
        let count = textures.len();
        self.credit = vec![0.0; count];
        self.loaded = Some(Loaded { profile, textures, batches: vec![Vec::new(); count], handed: Vec::new() });
    }

    /// Free the GPU textures when the scene goes.
    pub(in crate::scenes::world) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.wanted = None;
        if let Some(loaded) = self.loaded.take() {
            self.stale.extend(loaded.textures.iter().flatten().map(|(handle, _)| *handle));
        }
        self.splashes.clear();
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / 16_777_216.0
    }

    /// Live splashes.
    pub(in crate::scenes::world) fn len(&self) -> usize {
        self.splashes.len()
    }

    /// One step. `rain` is how hard it rains where the player is (0 to 1, 0 when under cover), `centre` the player, `ground`
    /// gives the height of the road below a point.
    pub(in crate::scenes::world) fn step(
        &mut self,
        dt: f32,
        rain: f32,
        centre: Vec3,
        ground: impl Fn(f32, f32, f32, f32) -> Option<f32>,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        for splash in &mut self.splashes {
            splash.age += dt;
        }
        self.splashes.retain(|splash| splash.age < splash.life);
        let Some(profile) = self.loaded.as_ref().filter(|_| !self.needs_load()).map(|loaded| loaded.profile.clone())
        else {
            return;
        };
        let mut made = 0;
        for (index, layer) in profile.layers.iter().enumerate() {
            let owed = layer.rate * profile.density * rain.clamp(0.0, 1.0) * dt.min(0.1);
            self.credit[index] = (self.credit[index] + owed).min(PER_FRAME as f32);
            while self.credit[index] >= 1.0 && made < PER_FRAME {
                self.credit[index] -= 1.0;
                made += 1;
                let angle = self.random() * std::f32::consts::TAU;
                let distance = self.random().sqrt() * profile.radius;
                let (life_cut, cell, turn) = (self.random(), self.random(), self.random());
                let (x, y) = (centre.x + angle.cos() * distance, centre.y + angle.sin() * distance);
                let Some(z) = ground(x, y, centre.z + REACH[0], centre.z - REACH[1]) else { continue };
                if self.splashes.len() == POOL {
                    self.splashes.remove(0);
                }
                let cells = f32::from(layer.grid) * f32::from(layer.grid);
                self.splashes.push(Splash {
                    layer: index as u8,
                    at: Vec3::new(x, y, z),
                    age: 0.0,
                    life: layer.life * (1.0 - layer.life_variance * life_cut),
                    cell: (cell * cells) as u8,
                    turn: turn * std::f32::consts::TAU,
                });
            }
        }
    }

    /// Append one textured batch per layer with splashes to `layer`; call [`Self::reclaim`] after the upload.
    pub(in crate::scenes::world) fn geometry(&mut self, forward: Vec3, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.handed.clear();
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        for batch in &mut loaded.batches {
            batch.clear();
        }
        for splash in &self.splashes {
            let index = usize::from(splash.layer);
            let Some(spec) = loaded.profile.layers.get(index) else { continue };
            let t = (splash.age / splash.life).clamp(0.0, 1.0);
            let width = spec.size[0] + (spec.size[1] - spec.size[0]) * t;
            // Rising over the first fifth of the life, then falling to nothing.
            let level = match t < 0.2 {
                true => t / 0.2,
                false => (1.0 - t) / 0.8,
            };
            let alpha = (level * spec.opacity * 255.0) as u8;
            let color = [spec.color[0], spec.color[1], spec.color[2], alpha];
            let grid = f32::from(spec.grid);
            let (column, row) = (f32::from(splash.cell % spec.grid.max(1)), f32::from(splash.cell / spec.grid.max(1)));
            let rect = [[column / grid, row / grid], [(column + 1.0) / grid, (row + 1.0) / grid]];
            let corners = match spec.kind {
                SplashKind::Ring => {
                    let (sin, cos) = splash.turn.sin_cos();
                    let (x, y) = (Vec3::new(cos, sin, 0.0) * (width * 0.5), Vec3::new(-sin, cos, 0.0) * (width * 0.5));
                    let at = splash.at + Vec3::Z * LIFT;
                    [at - x - y, at + x - y, at + x + y, at - x + y]
                }
                SplashKind::Burst => {
                    let side = right * (width * 0.5);
                    let up = Vec3::Z * width;
                    [splash.at - side + up, splash.at + side + up, splash.at + side, splash.at - side]
                }
            };
            EffectLayer::particle_quad_uv(&mut loaded.batches[index], corners, color, [0.0; 2], rect);
        }
        for (index, batch) in loaded.batches.iter_mut().enumerate() {
            let (Some((handle, blend)), false) = (loaded.textures[index], batch.is_empty()) else { continue };
            let vertices = std::mem::take(batch);
            loaded.handed.push((index, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend, vertices });
        }
    }

    /// Take the vertex buffers back from `layer`.
    pub(in crate::scenes::world) fn reclaim(&mut self, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        for (index, handle, at) in loaded.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(at).filter(|batch| batch.texture == handle) else { continue };
            loaded.batches[index] = std::mem::take(&mut batch.vertices);
        }
    }
}

/// The height of the road below `(x, y)`: the scene's ground query, from `from_z` down to `to_z`.
pub(in crate::scenes::world) fn ground_query(
    collision: &CollisionWorld,
) -> impl Fn(f32, f32, f32, f32) -> Option<f32> + '_ {
    move |x, y, from_z, to_z| space::ground_below(collision, x, y, from_z, to_z)
}

#[cfg(test)]
#[path = "rain_splash_tests.rs"]
mod tests;
