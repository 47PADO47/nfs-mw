//! Authored scenery particles, independent of the selected prop solver.
mod cache;
#[cfg(test)]
mod installed_tests;
mod instance;
#[cfg(test)]
mod tests;

use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle};
use blackbox_particles::{Emitter, EmitterSpec, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::world::sequences::ObjectEffect;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Weak,
};

pub struct Source {
    pub id: u32,
    pub owner: Weak<()>,
    pub effect: ObjectEffect,
    pub frame: Frame,
}

pub enum Command {
    Play(Source),
    Stop { source: u32, owner: Weak<()>, instance: Option<u32> },
}

const MAX_INSTANCES: usize = 64;
const MAX_PENDING: usize = 256;
const MAX_PARTICLES: usize = 16384;
const PER_EMITTER: usize = 128;

struct Set {
    parameters: nfsmw_data::world::object_fx::Parameters,
    templates: Vec<(u32, EmitterSpec)>,
}
struct Instance {
    parameters: nfsmw_data::world::object_fx::Parameters,
    source: Source,
    emitters: Vec<(u32, Emitter)>,
    stopping: bool,
}

#[derive(Default)]
pub struct ObjectFx {
    pending: VecDeque<Command>,
    sets: BTreeMap<u32, Option<Set>>,
    textures: BTreeMap<u32, (TextureHandle, BlendMode)>,
    missing_textures: BTreeSet<u32>,
    live: Vec<Instance>,
    started: u64,
    rejected: u64,
}

impl ObjectFx {
    pub fn request(&mut self, commands: impl IntoIterator<Item = Command>) {
        for command in commands {
            if self.pending.len() >= MAX_PENDING {
                self.rejected += 1;
                continue;
            }
            self.pending.push_back(command);
        }
    }

    pub fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &blackbox_attrib::Database) {
        self.load_pending(renderer, dir, db);
        self.process_pending();
    }

    fn process_pending(&mut self) {
        while let Some(command) = self.pending.pop_front() {
            match command {
                Command::Stop { source, owner, instance } => {
                    for live in &mut self.live {
                        if live.source.id == source
                            && live.source.owner.ptr_eq(&owner)
                            && live.source.effect.instance != 0
                            && instance.is_none_or(|id| id == live.source.effect.instance)
                        {
                            live.stop();
                        }
                    }
                }
                Command::Play(source) => self.start(source),
            }
        }
    }

    fn start(&mut self, source: Source) {
        if source.effect.instance != 0 {
            for live in &mut self.live {
                if live.stopping
                    || live.source.id != source.id
                    || !live.source.owner.ptr_eq(&source.owner)
                    || live.source.effect.instance != source.effect.instance
                {
                    continue;
                }
                if live.source.effect.effect == source.effect.effect {
                    live.source = source;
                    return;
                }
                live.stop();
            }
        }
        let Some(Some(set)) = self.sets.get(&source.effect.effect) else { return };
        let reserved: usize = self.live.iter().flat_map(|i| &i.emitters).map(|(_, e)| e.limit()).sum();
        let needed = set.templates.len() * PER_EMITTER;
        if self.live.len() >= MAX_INSTANCES || reserved + needed > MAX_PARTICLES {
            self.rejected += 1;
            return;
        }
        self.started += 1;
        let emitters = set
            .templates
            .iter()
            .enumerate()
            .map(|(index, (texture, spec))| {
                let mut emitter = Emitter::new(spec.clone(), self.started * 64 + index as u64);
                emitter.set_limit(PER_EMITTER);
                if source.effect.instance == 0 {
                    emitter.force_one_shot();
                }
                (*texture, emitter)
            })
            .collect();
        self.live.push(Instance { source, emitters, parameters: set.parameters, stopping: false });
    }

    pub fn step(&mut self, props: &super::props::PropWorld, dt: f32) {
        for live in &mut self.live {
            live.step(props, dt);
        }
        self.live.retain(|live| live.emitters.iter().any(|(_, e)| !e.is_empty() || !e.finished_spawning()));
    }

    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.pending.clear();
        self.live.clear();
        self.sets.clear();
        self.missing_textures.clear();
        for (_, (texture, _)) in std::mem::take(&mut self.textures) {
            renderer.destroy_texture(texture);
        }
    }

    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for (&texture, &(handle, blend)) in &self.textures {
            let mut vertices = Vec::new();
            let gain = match blend {
                BlendMode::Additive => super::exhaust::ADDITIVE_GAIN,
                _ => 1.0,
            };
            for live in &self.live {
                for (_, emitter) in live.emitters.iter().filter(|(hash, _)| *hash == texture) {
                    for sprite in emitter.sprites() {
                        if (sprite.position - camera).dot(forward) <= 0.0 {
                            continue;
                        }
                        let corners = sprite.corners(right, up, forward);
                        let mut color = sprite.color;
                        color[3] = (f32::from(color[3]) * gain).min(255.0) as u8;
                        blackbox_render::EffectLayer::particle_quad_uv(
                            &mut vertices,
                            [corners[3], corners[2], corners[1], corners[0]],
                            color,
                            [0.0; 2],
                            sprite.uv,
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
