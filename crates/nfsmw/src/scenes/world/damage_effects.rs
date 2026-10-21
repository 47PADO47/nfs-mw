//! Persistent health-driven engine particles, shared by both rendering backends.
//! Spec: `docs/specs/cop-visible-damage.md`.
use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle, TexturedEffect};
use blackbox_particles::{Emitter, EmitterSpec, Frame};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::car::{
    damage_effects::DamageFx,
    exhaust::{ADDITIVE_BLEND, GroupEmitter, particle_textures},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Weak,
};

pub(super) struct Source<'a> {
    pub id: u32,
    pub owner: Weak<()>,
    pub fx: &'a DamageFx,
    pub health: f32,
    pub frame: Frame,
}

struct Run {
    id: u32,
    owner: Weak<()>,
    group: u32,
    frame: Frame,
    emitters: Vec<(u32, Emitter)>,
    active: bool,
}

const MAX_RUNS: usize = 64;
const MAX_PARTICLES: usize = 16_384;
const LIMIT: usize = 128;

#[derive(Default)]
pub(super) struct DamageEffects {
    pending: BTreeMap<u32, Vec<GroupEmitter>>,
    groups: BTreeMap<u32, Vec<(u32, EmitterSpec)>>,
    textures: BTreeMap<u32, (TextureHandle, BlendMode)>,
    missing: BTreeSet<u32>,
    runs: Vec<Run>,
    serial: u64,
}

impl DamageEffects {
    pub fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        let wanted: BTreeSet<_> = self
            .pending
            .values()
            .flatten()
            .map(|e| e.texture)
            .filter(|key| !self.textures.contains_key(key) && !self.missing.contains(key))
            .collect();
        if !wanted.is_empty() {
            let hashes: Vec<_> = wanted.iter().copied().collect();
            match particle_textures(dir, &hashes) {
                Ok(found) => {
                    for texture in found {
                        let Some(handle) = blackbox_scene::upload_texture(renderer, &texture) else { continue };
                        let blend = if texture.alpha_blend == ADDITIVE_BLEND {
                            BlendMode::Additive
                        } else {
                            BlendMode::AlphaBlend
                        };
                        self.textures.insert(texture.name_hash, (handle, blend));
                    }
                }
                Err(error) => log::warn!("vehicle damage particle textures: {error:#}"),
            }
            self.missing.extend(wanted.into_iter().filter(|key| !self.textures.contains_key(key)));
        }
        for (key, emitters) in std::mem::take(&mut self.pending) {
            let templates = emitters
                .into_iter()
                .filter(|e| self.textures.contains_key(&e.texture))
                .map(|e| (e.texture, e.spec))
                .collect();
            self.groups.insert(key, templates);
        }
    }

    pub fn step<'a>(&mut self, sources: impl IntoIterator<Item = Source<'a>>, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        for run in &mut self.runs {
            run.active = false;
        }
        for source in sources {
            let Some(group) = source.fx.select(source.health) else { continue };
            if !source.frame.to_world.is_finite() || !source.frame.inherit_velocity.is_finite() {
                continue;
            }
            if !self.groups.contains_key(&group.key) {
                if self.groups.len() + self.pending.len() < 128 {
                    self.pending.entry(group.key).or_insert_with(|| group.emitters.iter().take(16).cloned().collect());
                }
                continue;
            }
            let frame = Frame {
                to_world: source.frame.to_world * source.fx.engine.unwrap(),
                intensity: 1.0,
                inherit_velocity: source.frame.inherit_velocity,
            };
            if let Some(run) = self
                .runs
                .iter_mut()
                .find(|run| run.id == source.id && run.group == group.key && run.owner.ptr_eq(&source.owner))
            {
                run.active = true;
                run.frame = frame;
                continue;
            }
            let templates = &self.groups[&group.key];
            let reserved: usize = self.runs.iter().flat_map(|run| &run.emitters).map(|(_, e)| e.limit()).sum();
            if templates.is_empty() || self.runs.len() >= MAX_RUNS || reserved + templates.len() * LIMIT > MAX_PARTICLES
            {
                continue;
            }
            self.serial += 1;
            let emitters = templates
                .iter()
                .enumerate()
                .map(|(index, (texture, spec))| {
                    let mut spec = spec.clone();
                    spec.one_shot = false;
                    let mut emitter = Emitter::new(spec, self.serial * 32 + index as u64);
                    emitter.set_limit(LIMIT);
                    (*texture, emitter)
                })
                .collect();
            self.runs.push(Run { id: source.id, owner: source.owner, group: group.key, frame, emitters, active: true });
        }
        for run in &mut self.runs {
            let enabled = run.active && run.owner.upgrade().is_some();
            for (_, emitter) in &mut run.emitters {
                emitter.set_enabled(enabled);
                emitter.step(dt, &run.frame);
            }
        }
        self.runs.retain(|run| run.active || run.emitters.iter().any(|(_, e)| !e.is_empty()));
    }

    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for (&hash, &(texture, blend)) in &self.textures {
            let mut vertices = Vec::new();
            for run in &self.runs {
                for (_, emitter) in run.emitters.iter().filter(|(key, _)| *key == hash) {
                    for sprite in emitter.sprites() {
                        if (sprite.position - camera).dot(forward) <= 0.0 {
                            continue;
                        }
                        let corners = sprite.corners(right, up, forward);
                        blackbox_gfx::EffectLayer::particle_quad_uv(
                            &mut vertices,
                            [corners[3], corners[2], corners[1], corners[0]],
                            sprite.color,
                            [0.0; 2],
                            sprite.uv,
                        );
                    }
                }
            }
            if !vertices.is_empty() {
                out.push(TexturedEffect { texture, blend, vertices });
            }
        }
    }

    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.runs.clear();
        self.pending.clear();
        self.groups.clear();
        self.missing.clear();
        for (_, (texture, _)) in std::mem::take(&mut self.textures) {
            renderer.destroy_texture(texture);
        }
    }
}

#[cfg(test)]
mod tests;
