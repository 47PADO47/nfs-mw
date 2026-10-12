//! The light bars of cop cars: the particle emitters of their `fxcar_coplight*` groups, run at the `COPLIGHT*` markers
//! of each car with the on and off cycles the data gives, so the bars flash the way the original's do. A cop model's
//! textures are uploaded once; each car keeps its own emitters.

use std::rc::Rc;

use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle};
use blackbox_particles::{Emitter, EmitterSpec, Frame};
use blackbox_render::{EffectLayer, TexturedEffect};
use blackbox_tpk::Texture;
use glam::{Mat4, Vec3};
use nfsmw_data::car::cop_lights::CopLightFx;
use nfsmw_data::car::exhaust::ADDITIVE_BLEND;

use crate::scenes::world::exhaust::ADDITIVE_GAIN;

/// Most live particles of one emitter at one lamp.
const LIMIT: usize = 24;

/// One emitter of a light bar at one lamp: what it spawns, where (in the car's space) and its texture (an index into
/// the set's textures).
struct Template {
    texture: usize,
    spec: EmitterSpec,
    frame: Mat4,
}

/// A cop model's light bars, uploaded: the textures and the emitters of every lamp.
pub struct CopLightSet {
    textures: Vec<(TextureHandle, BlendMode)>,
    templates: Vec<Template>,
}

impl CopLightSet {
    /// Upload the textures the light bars use (`found` are the particle textures read from the install) and gather
    /// one template per emitter and lamp. `None` when nothing can be drawn.
    pub fn build(renderer: &mut dyn RenderBackend, fx: &CopLightFx, found: &[Texture]) -> Option<Rc<Self>> {
        let mut hashes: Vec<u32> = Vec::new();
        let mut textures = Vec::new();
        let mut templates = Vec::new();
        for lane in &fx.lanes {
            for emitter in &lane.emitters {
                let Some(texture) = found.iter().find(|t| t.name_hash == emitter.texture) else { continue };
                let slot = match hashes.iter().position(|&h| h == emitter.texture) {
                    Some(slot) => slot,
                    None => {
                        let Some(handle) = blackbox_scene::upload_texture(renderer, texture) else { continue };
                        let blend = match texture.alpha_blend == ADDITIVE_BLEND {
                            true => BlendMode::Additive,
                            false => BlendMode::AlphaBlend,
                        };
                        hashes.push(emitter.texture);
                        textures.push((handle, blend));
                        textures.len() - 1
                    }
                };
                for &frame in &lane.frames {
                    templates.push(Template { texture: slot, spec: emitter.spec.clone(), frame });
                }
            }
        }
        (!templates.is_empty()).then(|| Rc::new(Self { textures, templates }))
    }
}

/// One car's light bars: its emitters, one per template of the set.
pub struct CopLightRun {
    set: Rc<CopLightSet>,
    emitters: Vec<Emitter>,
}

impl CopLightRun {
    pub fn new(set: Rc<CopLightSet>) -> Self {
        let emitters = set
            .templates
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut emitter = Emitter::new(t.spec.clone(), i as u64 + 1);
                emitter.set_limit(LIMIT);
                emitter
            })
            .collect();
        Self { set, emitters }
    }

    /// Advance every emitter by `dt` seconds with the car's frame `to_world` (the emitters switch on and off by
    /// themselves).
    pub fn step(&mut self, dt: f32, to_world: Mat4) {
        for (emitter, template) in self.emitters.iter_mut().zip(&self.set.templates) {
            let frame = Frame { to_world: to_world * template.frame, inherit_velocity: Vec3::ZERO, intensity: 1.0 };
            emitter.step(dt, &frame);
        }
    }

    /// Append one textured batch per texture that has live particles, the sprites turned to face the camera.
    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for (slot, &(handle, blend)) in self.set.textures.iter().enumerate() {
            let gain = match blend {
                BlendMode::Additive => ADDITIVE_GAIN,
                _ => 1.0,
            };
            let mut vertices = Vec::new();
            for (emitter, template) in self.emitters.iter().zip(&self.set.templates) {
                if template.texture != slot {
                    continue;
                }
                for sprite in emitter.sprites() {
                    let corners = sprite.corners(right, up, forward);
                    if (sprite.position - camera).dot(forward) <= 0.0 {
                        continue;
                    }
                    let mut color = sprite.color;
                    color[3] = (f32::from(color[3]) * gain).min(255.0) as u8;
                    // The texture's top edge is v = 0: start from the upper-left corner.
                    EffectLayer::particle_quad(
                        &mut vertices,
                        [corners[3], corners[2], corners[1], corners[0]],
                        color,
                        [0.0; 2],
                    );
                }
            }
            if !vertices.is_empty() {
                out.push(TexturedEffect { texture: handle, blend, vertices });
            }
        }
    }
}
