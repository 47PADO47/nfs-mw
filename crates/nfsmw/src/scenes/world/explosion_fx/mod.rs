//! The petrol station's explosion: its authored particle group (`effects/exp_gasstation`, read from the install) run
//! at the blast point for a while, drawn as textured billboards the way the exhaust flames are. The blasts start
//! where the scenery physics reports a petrol-station explosion, and step with the fixed physics tick.

use blackbox_attrib::Database;
use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle};
use blackbox_particles::{Emitter, EmitterSpec, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, particle_textures};
use nfsmw_data::world::explosion_fx::petrol_station_emitters;

use super::exhaust::ADDITIVE_GAIN;

#[cfg(test)]
mod tests;

/// Seconds a blast keeps its emitters spawning. The install does not say how long the original runs them; this is
/// the rewrite's choice. Particles outlive it by their own life.
pub const BLAST_SECONDS: f32 = 2.0;
/// Most live particles of one emitter in one blast.
const LIMIT: usize = 64;
/// Most blasts alive at once; a new one drops the oldest.
const MAX_BLASTS: usize = 4;

/// The GPU side of the effect: its textures and, per emitter, the texture it is drawn with.
struct Set {
    textures: Vec<(TextureHandle, BlendMode)>,
    templates: Vec<(usize, EmitterSpec)>,
}

enum Load {
    Unloaded,
    Missing,
    Ready(Set),
}

/// One blast: its emitters at one point, and how long it has been running.
struct Blast {
    position: Vec3,
    age: f32,
    emitters: Vec<Emitter>,
}

impl Blast {
    fn new(templates: &[(usize, EmitterSpec)], position: Vec3, seed: u64) -> Self {
        let emitters = templates
            .iter()
            .enumerate()
            .map(|(i, (_, spec))| {
                let mut emitter = Emitter::new(spec.clone(), seed * 64 + i as u64 + 1);
                emitter.set_limit(LIMIT);
                emitter
            })
            .collect();
        Self { position, age: 0.0, emitters }
    }

    fn step(&mut self, dt: f32) {
        self.age += dt;
        let spawning = self.age < BLAST_SECONDS;
        let frame =
            Frame { to_world: Mat4::from_translation(self.position), inherit_velocity: Vec3::ZERO, intensity: 1.0 };
        for emitter in &mut self.emitters {
            emitter.set_enabled(spawning);
            emitter.step(dt, &frame);
        }
    }

    fn alive(&self) -> bool {
        self.age < BLAST_SECONDS || self.emitters.iter().map(Emitter::len).sum::<usize>() > 0
    }
}

pub struct ExplosionFx {
    load: Load,
    /// Points waiting for a blast, until the textures are loaded.
    queued: Vec<Vec3>,
    blasts: Vec<Blast>,
    /// How many blasts have started, for their random seeds.
    started: u64,
    /// Blasts started since the audio last asked (see [`ExplosionFx::take_started`]).
    unannounced: u32,
}

impl Default for ExplosionFx {
    fn default() -> Self {
        Self { load: Load::Unloaded, queued: Vec::new(), blasts: Vec::new(), started: 0, unannounced: 0 }
    }
}

impl ExplosionFx {
    /// Queue blasts at these points. Without the effect's art there is nothing to show, so they are dropped.
    pub fn request(&mut self, points: impl IntoIterator<Item = Vec3>) {
        if !matches!(self.load, Load::Missing) {
            self.queued.extend(points);
        }
    }

    /// Upload the effect's textures once; with no art (or no effect in the install) it stays off.
    pub fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database) {
        if !matches!(self.load, Load::Unloaded) {
            return;
        }
        self.load = match build(renderer, dir, db) {
            Some(set) => Load::Ready(set),
            None => Load::Missing,
        };
    }

    /// One fixed physics tick: start the queued blasts, advance the live ones and drop the spent ones.
    pub fn step(&mut self, dt: f32) {
        let set = match &self.load {
            Load::Ready(set) => set,
            _ => {
                self.queued.clear();
                return;
            }
        };
        for position in self.queued.drain(..) {
            if self.blasts.len() == MAX_BLASTS {
                self.blasts.remove(0);
            }
            self.started += 1;
            self.unannounced += 1;
            self.blasts.push(Blast::new(&set.templates, position, self.started));
        }
        for blast in &mut self.blasts {
            blast.step(dt);
        }
        self.blasts.retain(Blast::alive);
    }

    /// How many blasts started since the last call (the audio plays one explosion sound for each).
    pub fn take_started(&mut self) -> u32 {
        std::mem::take(&mut self.unannounced)
    }

    /// Append one textured batch per texture with live particles, the sprites turned to face the camera.
    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let Load::Ready(set) = &self.load else { return };
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for (slot, &(handle, blend)) in set.textures.iter().enumerate() {
            let gain = match blend {
                BlendMode::Additive => ADDITIVE_GAIN,
                _ => 1.0,
            };
            let mut vertices = Vec::new();
            for blast in &self.blasts {
                for (emitter, &(texture, _)) in blast.emitters.iter().zip(&set.templates) {
                    if texture != slot {
                        continue;
                    }
                    for sprite in emitter.sprites() {
                        if (sprite.position - camera).dot(forward) <= 0.0 {
                            continue;
                        }
                        let corners = sprite.corners(right, up, forward);
                        let mut color = sprite.color;
                        color[3] = (f32::from(color[3]) * gain).min(255.0) as u8;
                        // The texture's top edge is v = 0: start from the upper-left corner.
                        blackbox_render::EffectLayer::particle_quad(
                            &mut vertices,
                            [corners[3], corners[2], corners[1], corners[0]],
                            color,
                            [0.0; 2],
                        );
                    }
                }
            }
            if !vertices.is_empty() {
                out.push(TexturedEffect { texture: handle, blend, vertices });
            }
        }
    }
}

/// The effect's emitters with their textures uploaded. `None` when the install has no effect or no art for it.
fn build(renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database) -> Option<Set> {
    let emitters = petrol_station_emitters(db);
    if emitters.is_empty() {
        log::info!("no petrol station explosion effect in the install");
        return None;
    }
    let wanted: Vec<u32> = emitters.iter().map(|e| e.texture).collect();
    let found = particle_textures(dir, &wanted).unwrap_or_else(|e| {
        log::warn!("explosion textures: {e:#}");
        Vec::new()
    });
    let mut slots: Vec<u32> = Vec::new();
    let mut set = Set { textures: Vec::new(), templates: Vec::new() };
    for emitter in emitters {
        let Some(texture) = found.iter().find(|t| t.name_hash == emitter.texture) else {
            log::warn!("explosion particle texture {:08X} is not in the pack", emitter.texture);
            continue;
        };
        let slot = match slots.iter().position(|&h| h == emitter.texture) {
            Some(slot) => slot,
            None => {
                let Some(handle) = blackbox_scene::upload_texture(renderer, texture) else { continue };
                let blend = match texture.alpha_blend == ADDITIVE_BLEND {
                    true => BlendMode::Additive,
                    false => BlendMode::AlphaBlend,
                };
                slots.push(emitter.texture);
                set.textures.push((handle, blend));
                set.textures.len() - 1
            }
        };
        set.templates.push((slot, emitter.spec));
    }
    (!set.templates.is_empty()).then_some(set)
}
