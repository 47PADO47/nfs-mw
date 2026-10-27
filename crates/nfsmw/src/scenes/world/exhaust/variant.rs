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
use blackbox_particles::{Animation, Emitter, EmitterSpec, Frame, TrackKey};
use blackbox_render::{BlendMode, EffectLayer, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use super::{CarState, EMITTER_LIMIT, Lane, SpriteTexture};

/// The seed of the variant's emitters, apart from the game's own.
const SEED: u64 = 0x4e49_5452;
/// The throttle above which the pedal is down. The 2012 game sets the pop's light-state bit (14) only when the pedal bit (8)
/// and the nitrous start bit (9) are clear, so a pop effect never plays with the pedal pressed.
const PEDAL_DOWN: f32 = 0.05;

/// One effect of the profile at every pipe.
struct VariantLane {
    on: EffectOn,
    duration: f32,
    /// Particles a second at rest, and for each metre a second of the car's speed.
    rate: f32,
    rate_per_speed: f32,
    /// The share of the car's velocity a new particle carries on average: it is born that much of a step behind the pipe, so that
    /// the step that moves it brings it to the pipe and not past it.
    carried: f32,
    lane: Lane,
}

/// What was loaded for one profile.
struct Loaded {
    profile: Arc<NitrousProfile>,
    textures: Vec<SpriteTexture>,
    lanes: Vec<VariantLane>,
    /// Seconds left of the start effects.
    start_t: f32,
    /// Seconds since the game's own pop flame began, `f32::INFINITY` while there is none. It keeps counting after the game's
    /// short flame has gone, as long as the pedal stays up and the nitrous off: a pop effect plays out its whole duration.
    pop_t: f32,
    /// The game's own pop flame was lit in the latest step.
    pop_lit: bool,
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
    /// Whether any particle is alive.
    #[cfg(test)]
    pub(super) fn live(&self) -> bool {
        self.loaded
            .as_ref()
            .is_some_and(|loaded| loaded.lanes.iter().any(|lane| lane.lane.emitters.iter().any(|e| !e.is_empty())))
    }

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
            let known =
                textures.iter().position(|(path, additive, _)| *path == effect.texture && *additive == effect.additive);
            let slot = match known {
                Some(slot) => slot,
                None => {
                    let Some(texture) = upload(renderer, dir, effect) else { continue };
                    textures.push((effect.texture.clone(), effect.additive, texture));
                    textures.len() - 1
                }
            };
            lanes.push(VariantLane {
                on: effect.on,
                duration: effect.duration,
                rate: effect.rate,
                rate_per_speed: effect.rate_per_speed,
                carried: carried_share(effect),
                lane: lane(effect, index, pipes, slot),
            });
        }
        let textures = textures.into_iter().map(|(_, _, texture)| texture).collect();
        self.loaded = Some(Loaded {
            profile,
            textures,
            lanes,
            start_t: f32::INFINITY,
            pop_t: f32::INFINITY,
            pop_lit: false,
            handed: Vec::new(),
        });
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
    ///
    /// Each moment follows the 2012 state bits: the start (bit 9) is the instant the nitrous lights and the burn (bit 20) is the
    /// time it burns, so a tank that is empty while the button is held (the car's nitrous does not burn) has neither; the pop
    /// (bit 14) needs the pedal up and the nitrous off.
    pub(super) fn step(&mut self, dt: f32, car: &CarState, pipes: &[Mat4], pop: f32) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        let allowed = !car.nitrous && car.throttle <= PEDAL_DOWN;
        let lit = pop > 0.0 && allowed;
        // Time since each moment began; it restarts at the edge that begins it.
        loaded.start_t = match (car.nitrous, loaded.start_t.is_finite()) {
            (false, _) => f32::INFINITY,
            (true, false) => 0.0,
            (true, true) => loaded.start_t + dt,
        };
        // A pop begins when the game's flame lights and goes on after it, until the pedal or the nitrous ends it.
        loaded.pop_t = match (allowed, lit && !loaded.pop_lit, loaded.pop_t.is_finite()) {
            (false, _, _) => f32::INFINITY,
            (true, true, _) => 0.0,
            (true, false, true) => loaded.pop_t + dt,
            (true, false, false) => f32::INFINITY,
        };
        loaded.pop_lit = lit;
        let (start_t, pop_t) = (loaded.start_t, loaded.pop_t);
        let speed = car.velocity.length();
        for lane in &mut loaded.lanes {
            // A duration of 0 is for as long as the moment lasts.
            let within = |t: f32| t.is_finite() && (lane.duration <= 0.0 || t < lane.duration);
            let on = match lane.on {
                EffectOn::Start => within(start_t),
                EffectOn::Burn => car.nitrous,
                // An effect with a duration plays it out from the pop's start; one without lasts as long as the flame does.
                EffectOn::Pop if lane.duration <= 0.0 => lit,
                EffectOn::Pop => within(pop_t),
            };
            let intensity = spawn_intensity(lane.rate, lane.rate_per_speed, speed);
            let behind = car.velocity * (dt * lane.carried);
            for (emitter, pipe) in lane.lane.emitters.iter_mut().zip(pipes) {
                if !on && emitter.is_empty() {
                    continue;
                }
                emitter.set_enabled(on);
                let mut to_world = car.to_world * *pipe;
                to_world.w_axis -= behind.extend(0.0);
                emitter.step(dt, &Frame { to_world, inherit_velocity: car.velocity, intensity });
            }
        }
    }

    /// Let what is alive finish its life, with nothing new (the physics is parked).
    pub(super) fn age(&mut self, dt: f32) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.start_t = f32::INFINITY;
        loaded.pop_t = f32::INFINITY;
        loaded.pop_lit = false;
        for emitter in loaded.lanes.iter_mut().flat_map(|lane| lane.lane.emitters.iter_mut()) {
            if emitter.is_empty() {
                continue;
            }
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
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

/// The share of the car's velocity the particles of `effect` carry on average at birth.
fn carried_share(effect: &EffectProfile) -> f32 {
    effect.inherit * (1.0 - 0.5 * effect.inherit_variance)
}

/// The emitter's intensity (its rate is multiplied by it) that makes the spawn rate `rate + rate_per_speed * speed`: the
/// emitter's own rate is `rate`, or `rate_per_speed` itself when the effect has no rate at rest.
pub(in crate::scenes::world) fn spawn_intensity(rate: f32, rate_per_speed: f32, speed: f32) -> f32 {
    match rate > 0.0 {
        true => 1.0 + rate_per_speed * speed / rate,
        false => speed,
    }
}

/// The emitter parameters of an effect: the profile's numbers on the game's own emitter model.
pub(in crate::scenes::world) fn emitter_spec(effect: &EffectProfile) -> EmitterSpec {
    EmitterSpec {
        animation: Animation { grid: effect.grid, fps: effect.fps, random_start: effect.random_start },
        // With no rate at rest the emitter's rate is the one per metre a second, and the intensity is the speed.
        rate: if effect.rate > 0.0 { effect.rate } else { effect.rate_per_speed },
        life: effect.life,
        life_variance: effect.life_variance,
        speed: effect.speed,
        speed_variance: effect.speed_variance,
        spread: effect.spread,
        // The emitter's gravity is subtracted from the vertical speed: a lift is its opposite.
        gravity: -effect.lift,
        drag: effect.drag,
        damping: effect.damping,
        inherit: effect.inherit,
        inherit_variance: effect.inherit_variance,
        initial_angle_range: effect.angle_range,
        // The look is the profile's keys as straight lines between them. A particle turns either way at a rate up to the
        // profile's spin, so half of it on average.
        track: effect
            .keys
            .iter()
            .zip(&effect.size)
            .zip(&effect.colors)
            .map(|((&t, &size), &color)| TrackKey { t, size, angle: 0.5 * effect.spin * effect.life * t, color })
            .collect(),
        random_rotation_direction: effect.spin > 0.0,
        kill_alpha: None,
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
    let (handle, blend) = upload_png(renderer, dir, &effect.texture, effect.additive, "nitrous variant")?;
    Some(SpriteTexture { handle: Some(handle), blend })
}

/// Read the PNG at `path` of the install's layers and upload it with the blend a particle layer asks for, or `None` with a
/// warning that starts with `what` (the effect variant it is for). Shared by the effect variants that draw particles.
pub(in crate::scenes::world) fn upload_png(
    renderer: &mut dyn RenderBackend,
    dir: &GameDir,
    path: &str,
    additive: bool,
    what: &str,
) -> Option<(TextureHandle, BlendMode)> {
    let bytes = match dir.read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("{what}: {path} is not read: {error}");
            return None;
        }
    };
    let image = match crate::ui::decode_png_rgba(&bytes) {
        Ok(image) => image,
        Err(error) => {
            log::warn!("{what}: {path} is not a usable PNG: {error:#}");
            return None;
        }
    };
    let handle = renderer.create_texture(&TextureDesc {
        label: path,
        width: image.width,
        height: image.height,
        format: PixelFormat::Rgba8,
        mips: vec![&image.rgba[..]],
    });
    let blend = match additive {
        true => BlendMode::Additive,
        false => BlendMode::AlphaBlend,
    };
    Some((handle, blend))
}

/// A sprite ready to draw: its depth, corners, colour and texture window.
type DrawnSprite = (f32, [Vec3; 4], [u8; 4], [[f32; 2]; 2]);

/// Fill the lane's vertices with its quads, far to near; false when it has no particles.
fn quads(lane: &mut Lane, camera: Vec3, forward: Vec3) -> bool {
    let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
    let up = right.cross(forward).normalize_or(Vec3::Z);
    let mut order: Vec<DrawnSprite> = lane
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
