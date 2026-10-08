//! Flames at the tail pipes: the nitrous and the blow-off after a gear change (spec: exhaust-flames.md).
//!
//! The car's `ecar` record names a particle group; its emitters (read from the install) are run at every pipe
//! marker of the car model with `blackbox-particles` and drawn as textured billboards, additive or alpha-blended
//! as the particle textures say. Nothing here is stored in the repository: definitions and textures come from
//! the install when a car is loaded.

mod trigger;

use blackbox_attrib::Database;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, Renderer, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, ExhaustFx, particle_textures};

use trigger::{ShiftEvent, ShiftTiming, blowoff_allowed, pipes_flame};

/// A particle texture on the GPU and how it blends.
struct SpriteTexture {
    handle: Option<TextureHandle>,
    blend: BlendMode,
}

/// All the pipes' emitters of one group emitter, drawn with one texture.
struct Lane {
    texture: usize,
    /// One emitter per tail pipe, in the pipe order.
    emitters: Vec<Emitter>,
}

/// What the effects need of the car for one physics step.
pub struct CarState {
    /// The car model's frame in the world.
    pub to_world: Mat4,
    pub velocity: Vec3,
    /// -1 reverse, 0 neutral, 1 and up forward.
    pub gear: i32,
    /// The nitrous is burning.
    pub nitrous: bool,
}

#[derive(Default)]
pub struct ExhaustFlames {
    /// Each pipe's frame in the car model's space.
    pipes: Vec<Mat4>,
    lanes: Vec<Lane>,
    textures: Vec<SpriteTexture>,
    timing: ShiftTiming,
    engine_upgrades: i32,
    /// The installed engine upgrade level (the career will supply it; for now a console setting).
    engine_level: i32,
    shift: ShiftEvent,
    off: bool,
    /// Whether the pipes flamed in the latest step.
    flaming: bool,
    order: Vec<(f32, [Vec3; 4], [u8; 4])>,
    /// The vertices of the lane being built.
    vertices: Vec<EffectVertex>,
}

impl ExhaustFlames {
    /// The effects of `model`: its pipes, particle group and textures, uploaded to `renderer`. A car without
    /// pipes or data gets none (nothing is drawn).
    pub fn load(renderer: &mut Renderer, dir: &GameDir, db: &Database, model: &CarModel) -> Self {
        let Some(fx) = ExhaustFx::read(db, model) else {
            log::info!("{}: no ecar record, so no exhaust effects", model.name);
            return Self::default();
        };
        if fx.pipes.is_empty() || fx.nitrous.is_empty() {
            log::info!("{}: {} tail pipes, {} flame emitters", model.name, fx.pipes.len(), fx.nitrous.len());
            return Self::default();
        }
        let wanted: Vec<u32> = fx.nitrous.iter().map(|e| e.texture).collect();
        let found = particle_textures(dir, &wanted).unwrap_or_else(|e| {
            log::warn!("particle textures: {e:#}");
            Vec::new()
        });
        // One GPU texture per distinct name hash; the emitters whose texture is missing are left out.
        let mut hashes: Vec<u32> = Vec::new();
        let mut textures = Vec::new();
        let mut kept = Vec::new();
        let mut emitters = Vec::new();
        for emitter in &fx.nitrous {
            let Some(texture) = found.iter().find(|t| t.name_hash == emitter.texture) else {
                log::warn!("{}: particle texture {:08X} is not in the pack", model.name, emitter.texture);
                continue;
            };
            let slot = match hashes.iter().position(|&h| h == emitter.texture) {
                Some(slot) => slot,
                None => {
                    hashes.push(emitter.texture);
                    textures.push(SpriteTexture {
                        handle: blackbox_scene::upload_texture(renderer, texture),
                        blend: match texture.alpha_blend == ADDITIVE_BLEND {
                            true => BlendMode::Additive,
                            false => BlendMode::AlphaBlend,
                        },
                    });
                    textures.len() - 1
                }
            };
            kept.push(slot);
            emitters.push(emitter.clone());
        }
        log::info!(
            "{}: {} tail pipes, {} flame emitters, shift timing {}/{}, {} engine upgrades",
            model.name,
            fx.pipes.len(),
            emitters.len(),
            fx.shift_speed,
            fx.shift_angle,
            fx.engine_upgrades
        );
        Self::new(ExhaustFx { nitrous: emitters, ..fx }, &kept, textures)
    }

    /// The effects of `fx`: emitter `i` of its nitrous group is drawn with `textures[texture_of[i]]`.
    fn new(fx: ExhaustFx, texture_of: &[usize], textures: Vec<SpriteTexture>) -> Self {
        let lanes = fx
            .nitrous
            .iter()
            .zip(texture_of)
            .enumerate()
            .map(|(i, (emitter, &texture))| {
                let seed = |pipe: usize| (i * 16 + pipe) as u64 + 1;
                Lane {
                    texture,
                    emitters: (0..fx.pipes.len()).map(|p| Emitter::new(emitter.spec.clone(), seed(p))).collect(),
                }
            })
            .collect();
        Self {
            pipes: fx.pipes,
            lanes,
            textures,
            timing: ShiftTiming { speed: fx.shift_speed, angle: fx.shift_angle },
            engine_upgrades: fx.engine_upgrades,
            ..Self::default()
        }
    }

    /// Free the GPU textures (before the effects are replaced by another car's).
    pub fn release(&mut self, renderer: &mut Renderer) {
        for texture in self.textures.drain(..) {
            if let Some(handle) = texture.handle {
                renderer.destroy_texture(handle);
            }
        }
    }

    /// One physics step: the shift event, then every pipe's emitters (spawning while the pipes flame).
    pub fn step(&mut self, dt: f32, car: &CarState) {
        self.shift.note_gear(car.gear);
        self.shift.advance(dt, car.gear, car.velocity.length(), self.timing);
        let blowoff = blowoff_allowed(self.engine_level, self.engine_upgrades);
        self.flaming = !self.off && pipes_flame(car.nitrous, blowoff, &self.shift);
        for lane in &mut self.lanes {
            for (emitter, pipe) in lane.emitters.iter_mut().zip(&self.pipes) {
                emitter.set_enabled(self.flaming);
                let frame = Frame { to_world: car.to_world * *pipe, inherit_velocity: car.velocity, intensity: 1.0 };
                emitter.step(dt, &frame);
            }
        }
    }

    /// Let the particles alive finish their life without spawning any (the physics is parked).
    pub fn age(&mut self, dt: f32) {
        self.flaming = false;
        for emitter in self.lanes.iter_mut().flat_map(|l| l.emitters.iter_mut()) {
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
    }

    /// The car was put somewhere else: forget the shift event; the particles alive finish their short life.
    pub fn disconnect(&mut self) {
        self.shift.reset();
        self.flaming = false;
    }

    /// Remove every particle and forget the shift event.
    pub fn clear(&mut self) {
        self.disconnect();
        for emitter in self.lanes.iter_mut().flat_map(|l| l.emitters.iter_mut()) {
            emitter.clear();
        }
    }

    fn live(&self) -> usize {
        self.lanes.iter().flat_map(|l| &l.emitters).map(Emitter::len).sum()
    }

    /// Append one textured batch per particle texture to `layer`, each sorted far to near.
    pub fn geometry(&mut self, camera: Vec3, forward: Vec3, layer: &mut EffectLayer) {
        for lane in 0..self.lanes.len() {
            if !self.quads(lane, camera, forward) {
                continue;
            }
            let texture = &self.textures[self.lanes[lane].texture];
            let Some(handle) = texture.handle else { continue };
            let vertices = std::mem::take(&mut self.vertices);
            layer.textured.push(TexturedEffect { texture: handle, blend: texture.blend, vertices });
        }
    }

    /// Fill `self.vertices` with the quads of lane `lane`, far to near; false when it has no particles.
    fn quads(&mut self, lane: usize, camera: Vec3, forward: Vec3) -> bool {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        self.order.clear();
        self.vertices.clear();
        for sprite in self.lanes[lane].emitters.iter().flat_map(Emitter::sprites) {
            let corners = sprite.corners(right, up, forward);
            self.order.push(((sprite.position - camera).dot(forward), corners, sprite.color));
        }
        if self.order.is_empty() {
            return false;
        }
        self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, c, color) in &self.order {
            // The texture's top edge is v = 0: start from the upper-left corner.
            EffectLayer::particle_quad(&mut self.vertices, [c[3], c[2], c[1], c[0]], color, [0.0; 2]);
        }
        true
    }

    /// `exhaust-flames [status|on|off|engine <level>]`.
    pub fn command(&mut self, args: &[&str]) -> Result<String, String> {
        const USAGE: &str = "usage: exhaust-flames [status|on|off|engine <level>]";
        match args {
            [] | ["status"] => {}
            ["on"] => self.off = false,
            ["off"] => {
                self.off = true;
                self.clear();
            }
            ["engine", level] => {
                self.engine_level = level.parse::<i32>().ok().filter(|l| *l >= 0).ok_or(USAGE)?;
            }
            _ => return Err(USAGE.into()),
        }
        let word = |on: bool| match on {
            true => "on",
            false => "off",
        };
        Ok(format!(
            "exhaust flames {}: {} pipes, engine level {} of {} (shift blow-off {}), {} particles, {}",
            word(!self.off),
            self.pipes.len(),
            self.engine_level,
            self.engine_upgrades,
            word(blowoff_allowed(self.engine_level, self.engine_upgrades)),
            self.live(),
            match self.flaming {
                true => "flaming",
                false => "quiet",
            }
        ))
    }
}

#[cfg(test)]
mod tests;
