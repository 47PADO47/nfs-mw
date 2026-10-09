//! Reading a car's flames from the install: the pipes, the particle group and its textures.

use blackbox_attrib::Database;
use blackbox_gfx::RenderBackend;
use blackbox_particles::Emitter;
use blackbox_render::BlendMode;
use game_install::GameDir;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, ExhaustFx, particle_textures};

use super::trigger::ShiftTiming;
use super::{Active, EMITTER_LIMIT, ExhaustFlames, Lane, MAX_PIPES, SpriteTexture, State};

impl ExhaustFlames {
    /// Load the flames of `model` if the setting is on and they are not loaded: its pipes, particle group and
    /// textures, uploaded to `renderer`. A car without pipes or data gets none (nothing is drawn).
    pub fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database, model: &CarModel) {
        if !self.wants_load() {
            return;
        }
        self.state = match read(renderer, dir, db, model) {
            Some(active) => State::Ready(Box::new(active)),
            None => State::Missing,
        };
    }
}

fn read(renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database, model: &CarModel) -> Option<Active> {
    let Some(fx) = ExhaustFx::read(db, model) else {
        log::info!("{}: no ecar record, so no exhaust effects", model.name);
        return None;
    };
    if fx.pipes.is_empty() || fx.nitrous.is_empty() {
        log::info!("{}: {} tail pipes, {} flame emitters", model.name, fx.pipes.len(), fx.nitrous.len());
        return None;
    }
    let wanted: Vec<u32> = fx.nitrous.iter().map(|e| e.texture).collect();
    let found = particle_textures(dir, &wanted).unwrap_or_else(|e| {
        log::warn!("particle textures: {e:#}");
        Vec::new()
    });
    // One GPU texture per distinct name hash; the emitters whose texture is missing are left out.
    let mut hashes: Vec<u32> = Vec::new();
    let mut textures = Vec::new();
    let mut lanes = Vec::new();
    let mut pipes = fx.pipes;
    pipes.truncate(MAX_PIPES);
    for (i, emitter) in fx.nitrous.iter().enumerate() {
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
        lanes.push(lane(&emitter.spec, i, pipes.len(), slot));
    }
    if lanes.is_empty() {
        return None;
    }
    log::info!(
        "{}: {} tail pipes, {} flame emitters, shift timing {}/{}, {} engine upgrades",
        model.name,
        pipes.len(),
        lanes.len(),
        fx.shift_speed,
        fx.shift_angle,
        fx.engine_upgrades
    );
    Some(Active::new(
        pipes,
        lanes,
        textures,
        ShiftTiming { speed: fx.shift_speed, angle: fx.shift_angle },
        fx.engine_upgrades,
    ))
}

/// The emitters of group emitter `index` at each of `pipes` pipes, drawn with texture `texture`.
pub(super) fn lane(spec: &blackbox_particles::EmitterSpec, index: usize, pipes: usize, texture: usize) -> Lane {
    let emitters = (0..pipes)
        .map(|pipe| {
            let mut emitter = Emitter::new(spec.clone(), (index * 16 + pipe) as u64 + 1);
            emitter.set_limit(EMITTER_LIMIT);
            emitter
        })
        .collect();
    Lane { texture, emitters, vertices: Vec::new() }
}

impl Active {
    pub(super) fn new(
        pipes: Vec<glam::Mat4>,
        lanes: Vec<Lane>,
        textures: Vec<SpriteTexture>,
        timing: ShiftTiming,
        engine_upgrades: i32,
    ) -> Self {
        Self {
            pipes,
            lanes,
            textures,
            timing,
            engine_upgrades,
            shift: Default::default(),
            backfire: Default::default(),
            intensity: 0.0,
            live: 0,
            order: Vec::new(),
            handed: Vec::new(),
        }
    }
}
