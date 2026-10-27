//! The tire smoke of a mod's variant (`docs/modding/effect-variants.md`): the `[[layer]]` emitters of its profile, one for each
//! layer at each wheel that smokes, drawn with the mod's own pictures in place of the game's procedural puffs.
//!
//! The game's particle system (`blackbox-particles`) and its textured quads are used as they are; the profile gives the
//! textures and the numbers (rate, life, size, colour and alpha over the life, rise, spread, drag). A wheel's emitter sits on
//! the road under it with its axis along the road's normal, and its spawn rate is the profile's times the wheel's smoke (the
//! physics' own slip intensity, 0 to 1), so the physics still decides how much smoke there is. The colour mode of the game
//! (`tint.rs`) recolours the sprites while the quads are built. With no variant chosen nothing here is loaded or drawn.

use std::sync::Arc;

use blackbox_gfx::RenderBackend;
use blackbox_modding::content::TireSmokeProfile;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use super::tint;
use super::{Contact, WHEELS};
use crate::scenes::world::exhaust::variant::{emitter_spec, spawn_intensity, upload_png};
use crate::settings::SmokeTint;

/// The seed of the variant's emitters.
const SEED: u64 = 0x5449_5245;
/// Most live particles of one emitter (one layer at one wheel).
const EMITTER_LIMIT: usize = 128;
/// How far above the road the emitters sit, metres (the game's own smoke starts 0.12 m up).
const LIFT: f32 = 0.12;

/// One layer of the profile at every wheel.
struct Layer {
    /// The slot of the layer's picture in `Loaded::textures`.
    texture: usize,
    rate: f32,
    rate_per_speed: f32,
    emitters: [Emitter; WHEELS],
    /// The vertices of the layer's batch; they travel to the layer and come back after the upload.
    vertices: Vec<EffectVertex>,
}

/// What was loaded for one profile.
struct Loaded {
    profile: Arc<TireSmokeProfile>,
    /// A picture on the GPU and its blend; `None` for a picture that could not be read (its layers draw nothing).
    textures: Vec<Option<(TextureHandle, BlendMode)>>,
    layers: Vec<Layer>,
    /// The batches handed to the layer by the latest `geometry`: the layer, the texture and the position in the layer's list.
    handed: Vec<(usize, TextureHandle, usize)>,
}

/// The tire smoke variant of one car's wheels.
#[derive(Default)]
pub(super) struct SmokeVariant {
    /// The profile the scene says is in effect (`None`: the game's own smoke).
    wanted: Option<Arc<TireSmokeProfile>>,
    loaded: Option<Loaded>,
    /// GPU textures to free at the next chance.
    stale: Vec<TextureHandle>,
    /// Seconds of smoke time, which turns the rainbow.
    clock: f32,
    /// What the colour mode reads of the car now (`tint::level_of`), for the sprites of every layer.
    level: f32,
    /// The sprites of a layer, far to near (kept between frames so a frame allocates nothing).
    order: Vec<Sprite>,
}

/// A sprite ready to draw: its depth, corners, colour and texture window.
type Sprite = (f32, [Vec3; 4], [u8; 4], [[f32; 2]; 2]);

impl SmokeVariant {
    /// The profile in effect, or `None`.
    pub(super) fn set_wanted(&mut self, profile: Option<Arc<TireSmokeProfile>>) {
        self.wanted = profile;
    }

    /// Whether the profile in effect is not the one that is loaded.
    pub(super) fn needs_load(&self) -> bool {
        match (&self.wanted, &self.loaded) {
            (None, None) => false,
            (Some(want), Some(have)) => !Arc::ptr_eq(want, &have.profile) && **want != *have.profile,
            _ => true,
        }
    }

    /// Whether the variant is what draws the smoke now: a profile is in effect and it is loaded.
    pub(super) fn active(&self) -> bool {
        self.wanted.is_some() && self.loaded.is_some() && !self.needs_load()
    }

    /// Live particles of every layer.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        let layers = self.loaded.iter().flat_map(|loaded| &loaded.layers);
        layers.flat_map(|layer| &layer.emitters).map(Emitter::len).sum()
    }

    /// Remove every particle.
    pub(super) fn clear(&mut self) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.layers.iter_mut().flat_map(|layer| layer.emitters.iter_mut()).for_each(Emitter::clear);
    }

    /// Drop what is loaded; its textures are freed by [`SmokeVariant::release`].
    pub(super) fn unload(&mut self) {
        let Some(loaded) = self.loaded.take() else { return };
        self.stale.extend(loaded.textures.iter().flatten().map(|(handle, _)| *handle));
    }

    /// Free the GPU textures that were dropped.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    /// Load the profile in effect: its pictures, from the install's layers, and one emitter per wheel for each layer. A layer
    /// whose picture cannot be read is kept without one, with a warning, and draws nothing.
    pub(super) fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        self.unload();
        let Some(profile) = self.wanted.clone() else { return };
        let mut pictures: Vec<(String, bool)> = Vec::new();
        let mut textures = Vec::new();
        let mut slots = Vec::new();
        for layer in &profile.layers {
            let key = (layer.texture.clone(), layer.additive);
            let slot = match pictures.iter().position(|known| *known == key) {
                Some(slot) => slot,
                None => {
                    pictures.push(key);
                    textures.push(upload_png(renderer, dir, &layer.texture, layer.additive, "tire smoke variant"));
                    textures.len() - 1
                }
            };
            slots.push(slot);
        }
        let layers = make_layers(&profile, &slots);
        self.loaded = Some(Loaded { profile, textures, layers, handed: Vec::new() });
    }

    /// One physics step. A wheel with a contact that smokes (`emit`) spawns; the rest only age.
    pub(super) fn step(
        &mut self,
        contacts: &[Option<Contact>; WHEELS],
        emit: bool,
        velocity: Vec3,
        tint: SmokeTint,
        dt: f32,
    ) {
        self.clock = (self.clock + dt) % 3600.0;
        let speed = velocity.length();
        let smoke = contacts.iter().flatten().map(|contact| contact.smoke).fold(0.0, f32::max);
        self.level = tint::level_of(tint.mode, speed, smoke);
        let Some(loaded) = self.loaded.as_mut() else { return };
        for layer in &mut loaded.layers {
            let spawn = spawn_intensity(layer.rate, layer.rate_per_speed, speed);
            for (emitter, contact) in layer.emitters.iter_mut().zip(contacts) {
                let wheel = contact.filter(|contact| emit && contact.smoke > 0.0);
                let Some(contact) = wheel else {
                    emitter.set_enabled(false);
                    emitter.step(dt, &Frame::default());
                    continue;
                };
                emitter.set_enabled(true);
                let frame = Frame {
                    to_world: road_frame(&contact),
                    inherit_velocity: velocity,
                    intensity: contact.smoke * spawn,
                };
                emitter.step(dt, &frame);
            }
        }
    }

    /// Let what is alive finish its life, with nothing new (the physics is parked).
    pub(super) fn age(&mut self, dt: f32) {
        self.clock = (self.clock + dt) % 3600.0;
        let Some(loaded) = self.loaded.as_mut() else { return };
        for emitter in loaded.layers.iter_mut().flat_map(|layer| layer.emitters.iter_mut()) {
            if emitter.is_empty() {
                continue;
            }
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
    }

    /// Append one textured batch per layer with particles to `layer`, each far to near. Call [`SmokeVariant::reclaim`] once the
    /// layer has been uploaded.
    pub(super) fn geometry(&mut self, camera: Vec3, forward: Vec3, tint: SmokeTint, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.handed.clear();
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for index in 0..loaded.layers.len() {
            let Some((handle, blend)) = loaded.textures[loaded.layers[index].texture] else { continue };
            let smoke = &mut loaded.layers[index];
            self.order.clear();
            for sprite in smoke.emitters.iter().flat_map(Emitter::sprites) {
                let mut color = sprite.color;
                let phase = tint::rainbow_phase(self.clock, sprite.position);
                let [r, g, b] = tint::smoke_rgb(tint, [color[0], color[1], color[2]], phase, self.level);
                color[..3].copy_from_slice(&[r, g, b]);
                let corners = sprite.corners(right, up, forward);
                self.order.push(((sprite.position - camera).dot(forward), corners, color, sprite.uv));
            }
            smoke.vertices.clear();
            if self.order.is_empty() {
                continue;
            }
            self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
            for &(_, c, color, uv) in &self.order {
                // The texture's top edge is v = 0: start from the upper-left corner.
                EffectLayer::particle_quad_uv(&mut smoke.vertices, [c[3], c[2], c[1], c[0]], color, [0.0; 2], uv);
            }
            let vertices = std::mem::take(&mut smoke.vertices);
            loaded.handed.push((index, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend, vertices });
        }
    }

    /// Take the vertex buffers back from `layer`.
    pub(super) fn reclaim(&mut self, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        for (index, handle, at) in loaded.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(at).filter(|batch| batch.texture == handle) else { continue };
            loaded.layers[index].vertices = std::mem::take(&mut batch.vertices);
        }
    }
}

/// One layer for each of the profile's, with an emitter at every wheel; `slots` says which picture each one draws.
fn make_layers(profile: &TireSmokeProfile, slots: &[usize]) -> Vec<Layer> {
    let mut layers = Vec::with_capacity(profile.layers.len());
    for (index, (layer, &texture)) in profile.layers.iter().zip(slots).enumerate() {
        let spec = emitter_spec(layer);
        let emitters = std::array::from_fn(|wheel| {
            let mut emitter = Emitter::new(spec.clone(), SEED + (index * WHEELS + wheel) as u64);
            emitter.set_limit(EMITTER_LIMIT);
            emitter
        });
        layers.push(Layer {
            texture,
            rate: layer.rate,
            rate_per_speed: layer.rate_per_speed,
            emitters,
            vertices: Vec::new(),
        });
    }
    layers
}

/// The frame of the road under a wheel: its origin a little above the contact, its `z` axis the road's normal and its `x` axis
/// the way the wheel rolls.
fn road_frame(contact: &Contact) -> Mat4 {
    let z = contact.normal;
    let x = contact.forward;
    let y = z.cross(x);
    Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), (contact.point + z * LIFT).extend(1.0))
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
