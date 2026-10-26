//! The tail pipe effects of a mod's nitrous variant (`docs/modding/nitrous-variants.md`): the `[[effect]]` emitters of its
//! profile, played at every pipe of the car with the mod's own textures.
//!
//! The effects play on top of the game's own flames, or in their place when the profile says so (`replace_burn`,
//! `replace_pop`): a burn effect for as long as the nitrous burns, a start effect for a few seconds after it lights, a pop
//! effect at a blow-off or a backfire. The game's emitters (`blackbox-particles`) and its quad drawing are used as they
//! are; only the numbers and the textures are the profile's. With no variant chosen nothing here is loaded or drawn.

use std::sync::Arc;

use blackbox_gfx::{PixelFormat, RenderBackend, TextureDesc};
use blackbox_modding::content::{EffectOn, EffectProfile, NitrousProfile};
use blackbox_particles::{Animation, Emitter, EmitterSpec, Frame};
use blackbox_render::{BlendMode, EffectLayer, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use super::{CarState, EMITTER_LIMIT, Lane, SpriteTexture};

/// The seed of the variant's emitters, apart from the game's own.
const SEED: u64 = 0x4e49_5452;

/// One effect of the profile at every pipe.
struct VariantLane {
    on: EffectOn,
    duration: f32,
    lane: Lane,
}

/// What was loaded for one profile.
struct Loaded {
    profile: Arc<NitrousProfile>,
    textures: Vec<SpriteTexture>,
    lanes: Vec<VariantLane>,
    /// Seconds left of the start effects.
    start_left: f32,
    burning: bool,
    /// The batches handed to the layer by the latest `geometry`: the lane, the texture and the layer's index.
    handed: Vec<(usize, TextureHandle, usize)>,
}

/// The variant's effects of one car's flames.
#[derive(Default)]
pub(super) struct Variant {
    /// The profile the scene says is in effect (`None`: the game's own flames only).
    wanted: Option<Arc<NitrousProfile>>,
    loaded: Option<Loaded>,
    /// GPU textures to free at the next chance.
    stale: Vec<TextureHandle>,
}

impl Variant {
    /// The profile in effect, or `None`. A profile with no effect is the same as none.
    pub(super) fn set_wanted(&mut self, profile: Option<Arc<NitrousProfile>>) {
        self.wanted = profile.filter(|profile| !profile.effects.is_empty());
    }

    /// Whether the profile in effect is not the one that is loaded.
    pub(super) fn needs_load(&self) -> bool {
        match (&self.wanted, &self.loaded) {
            (None, None) => false,
            (Some(want), Some(have)) => !Arc::ptr_eq(want, &have.profile) && **want != *have.profile,
            _ => true,
        }
    }

    /// Drop what is loaded; its textures are freed by [`Variant::release`].
    pub(super) fn unload(&mut self) {
        let Some(loaded) = self.loaded.take() else { return };
        self.stale.extend(loaded.textures.iter().filter_map(|texture| texture.handle));
    }

    /// Free the GPU textures that were dropped.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    /// Load the profile in effect: its textures, from the install's layers, and one emitter per pipe for each effect. An
    /// effect whose picture cannot be read is left out, with a warning.
    pub(super) fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, pipes: usize) {
        self.unload();
        let Some(profile) = self.wanted.clone() else { return };
        let mut textures: Vec<(String, bool, SpriteTexture)> = Vec::new();
        let mut lanes = Vec::new();
        for (index, effect) in profile.effects.iter().enumerate() {
            let known = textures.iter().position(|(path, additive, _)| *path == effect.texture && *additive == effect.additive);
            let slot = match known {
                Some(slot) => slot,
                None => {
                    let Some(texture) = upload(renderer, dir, effect) else { continue };
                    textures.push((effect.texture.clone(), effect.additive, texture));
                    textures.len() - 1
                }
            };
            lanes.push(VariantLane { on: effect.on, duration: effect.duration, lane: lane(effect, index, pipes, slot) });
        }
        let textures = textures.into_iter().map(|(_, _, texture)| texture).collect();
        self.loaded = Some(Loaded { profile, textures, lanes, start_left: 0.0, burning: false, handed: Vec::new() });
    }

    /// Whether the game's own blue nitrous flame is not drawn: a loaded burn effect stands in for it.
    pub(super) fn replaces_burn(&self) -> bool {
        self.replaces(EffectOn::Burn, |flame| flame.replace_burn)
    }

    /// Whether the game's own blow-off and backfire flame is not drawn.
    pub(super) fn replaces_pop(&self) -> bool {
        self.replaces(EffectOn::Pop, |flame| flame.replace_pop)
    }

    fn replaces(&self, on: EffectOn, flag: impl Fn(&blackbox_modding::content::FlameProfile) -> bool) -> bool {
        let Some(loaded) = &self.loaded else { return false };
        let asked = loaded.profile.flame.as_ref().is_some_and(flag);
        asked && loaded.lanes.iter().any(|lane| lane.on == on)
    }

    /// One physics step. `pop` is how strongly the game's own blow-off or backfire would flame now (0 to 1).
    pub(super) fn step(&mut self, dt: f32, car: &CarState, pipes: &[Mat4], pop: f32) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        if car.nitrous && !loaded.burning {
            loaded.start_left = loaded.lanes.iter().filter(|lane| lane.on == EffectOn::Start).map(|lane| lane.duration).fold(0.0, f32::max);
        }
        loaded.burning = car.nitrous;
        loaded.start_left = (loaded.start_left - dt).max(0.0);
        let (start, burn, pop) = (loaded.start_left > 0.0, car.nitrous, pop > 0.0 && !car.nitrous);
        for lane in &mut loaded.lanes {
            let on = match lane.on {
                EffectOn::Start => start,
                EffectOn::Burn => burn,
                EffectOn::Pop => pop,
            };
            for (emitter, pipe) in lane.lane.emitters.iter_mut().zip(pipes) {
                if !on && emitter.is_empty() {
                    continue;
                }
                emitter.set_enabled(on);
                emitter.step(dt, &Frame { to_world: car.to_world * *pipe, inherit_velocity: car.velocity, intensity: 1.0 });
            }
        }
    }

    /// Let what is alive finish its life, with nothing new (the physics is parked).
    pub(super) fn age(&mut self, dt: f32) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.burning = false;
        loaded.start_left = 0.0;
        for emitter in loaded.lanes.iter_mut().flat_map(|lane| lane.lane.emitters.iter_mut()) {
            if emitter.is_empty() {
                continue;
            }
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
    }

    /// Whether any particle is alive.
    pub(super) fn live(&self) -> bool {
        self.loaded.as_ref().is_some_and(|loaded| loaded.lanes.iter().any(|lane| lane.lane.emitters.iter().any(|e| !e.is_empty())))
    }

    /// Append one textured batch per effect with particles to `layer`, far to near. Call [`Variant::reclaim`] once the layer has
    /// been uploaded.
    pub(super) fn geometry(&mut self, camera: Vec3, forward: Vec3, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.handed.clear();
        for index in 0..loaded.lanes.len() {
            let texture = &loaded.textures[loaded.lanes[index].lane.texture];
            let (blend, Some(handle)) = (texture.blend, texture.handle) else { continue };
            if !quads(&mut loaded.lanes[index].lane, camera, forward) {
                continue;
            }
            let vertices = std::mem::take(&mut loaded.lanes[index].lane.vertices);
            loaded.handed.push((index, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend, vertices });
        }
    }

    /// Take the vertex buffers back from `layer`.
    pub(super) fn reclaim(&mut self, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        for (index, handle, at) in loaded.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(at).filter(|batch| batch.texture == handle) else { continue };
            loaded.lanes[index].lane.vertices = std::mem::take(&mut batch.vertices);
        }
    }
}

/// The emitter parameters of an effect: the profile's numbers on the game's own emitter model.
pub(super) fn emitter_spec(effect: &EffectProfile) -> EmitterSpec {
    EmitterSpec {
        animation: Animation { grid: effect.grid, fps: effect.fps, random_start: effect.random_start },
        rate: effect.rate,
        life: effect.life,
        life_variance: effect.life_variance,
        speed: effect.speed,
        speed_variance: effect.speed_variance,
        spread: effect.spread,
        // The emitter's gravity is subtracted from the vertical speed: a lift is its opposite.
        gravity: -effect.lift,
        drag: effect.drag,
        inherit: effect.inherit,
        size: effect.size,
        colors: effect.colors,
        ..EmitterSpec::default()
    }
}

/// The emitters of effect `index`, one per pipe, drawn with texture `texture`.
fn lane(effect: &EffectProfile, index: usize, pipes: usize, texture: usize) -> Lane {
    let spec = emitter_spec(effect);
    let emitters = (0..pipes)
        .map(|pipe| {
            let mut emitter = Emitter::new(spec.clone(), SEED + (index * 16 + pipe) as u64);
            emitter.set_limit(EMITTER_LIMIT);
            emitter
        })
        .collect();
    Lane { texture, emitters, vertices: Vec::new() }
}

/// Read the effect's picture and upload it, or `None` with a warning.
fn upload(renderer: &mut dyn RenderBackend, dir: &GameDir, effect: &EffectProfile) -> Option<SpriteTexture> {
    let bytes = match dir.read(&effect.texture) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("nitrous variant: {} is not read: {error}", effect.texture);
            return None;
        }
    };
    let image = match crate::ui::decode_png_rgba(&bytes) {
        Ok(image) => image,
        Err(error) => {
            log::warn!("nitrous variant: {} is not a usable PNG: {error:#}", effect.texture);
            return None;
        }
    };
    let handle = renderer.create_texture(&TextureDesc {
        label: &effect.texture,
        width: image.width,
        height: image.height,
        format: PixelFormat::Rgba8,
        mips: vec![&image.rgba[..]],
    });
    let blend = match effect.additive {
        true => BlendMode::Additive,
        false => BlendMode::AlphaBlend,
    };
    Some(SpriteTexture { handle: Some(handle), blend })
}

/// Fill the lane's vertices with its quads, far to near; false when it has no particles.
fn quads(lane: &mut Lane, camera: Vec3, forward: Vec3) -> bool {
    let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
    let up = right.cross(forward).normalize_or(Vec3::Z);
    let mut order: Vec<(f32, [Vec3; 4], [u8; 4], [[f32; 2]; 2])> = lane
        .emitters
        .iter()
        .flat_map(Emitter::sprites)
        .map(|sprite| {
            let corners = sprite.corners(right, up, forward);
            ((sprite.position - camera).dot(forward), corners, sprite.color, sprite.uv)
        })
        .collect();
    lane.vertices.clear();
    if order.is_empty() {
        return false;
    }
    order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    for (_, c, color, uv) in order {
        // The texture's top edge is v = 0: start from the upper-left corner.
        EffectLayer::particle_quad_uv(&mut lane.vertices, [c[3], c[2], c[1], c[0]], color, [0.0; 2], uv);
    }
    true
}

#[cfg(test)]
#[path = "variant_tests.rs"]
mod tests;
