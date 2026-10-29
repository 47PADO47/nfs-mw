//! Reading a car's flames from the install: the pipes, the particle group and its textures.

use blackbox_attrib::Database;
use blackbox_gfx::RenderBackend;
use blackbox_particles::Emitter;
use blackbox_render::BlendMode;
use game_install::GameDir;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, ExhaustFx, particle_textures};

use super::trigger::{BIKE_FLAME_SCALE, BIKE_SHIFT_TIMING, ShiftTiming};
use super::{Active, EMITTER_LIMIT, ExhaustFlames, Lane, MAX_PIPES, SpriteTexture, State};

/// The first random seed of the blue lanes, apart from the orange ones.
const BLUE_SEED: u64 = 0x424c_5545;

impl ExhaustFlames {
    /// Load the flames of `model` if the setting is on and they are not loaded: its pipes, particle group and
    /// textures, uploaded to `renderer`. A car without pipes or data gets none (nothing is drawn).
    pub fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database, model: &CarModel) {
        if self.wants_load() {
            self.state = match read(renderer, dir, db, model) {
                Some(active) => State::Ready(Box::new(active)),
                None => State::Missing,
            };
        }
        if let State::Ready(active) = &mut self.state {
            active.enhanced.enabled = self.enhanced;
            active.enhanced.load(renderer);
        }
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
    let bike = nfsmw_data::car::mods::bike::is_bike(&model.name);
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
        // A bike's flame is the group's at a fraction of its size (`scale::compact_bike`).
        let spec = match bike {
            true => super::scale::compact_bike(&emitter.spec, BIKE_FLAME_SCALE),
            false => emitter.spec.clone(),
        };
        lanes.push(lane(&spec, i, pipes.len(), slot));
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
    let mod_car = nfsmw_data::car::mods::is_mod_car(&model.name);
    let timing = shift_timing(bike, mod_car, (fx.shift_speed, fx.shift_angle));
    // A mod car has no `pvehicle` record to say how many engine levels it has: its blow-off is always allowed.
    let engine_upgrades = match mod_car {
        true => 0,
        false => fx.engine_upgrades,
    };
    let mut active = Active::new(pipes, lanes, textures, timing, engine_upgrades);
    active.mod_car = mod_car;
    active.bike = bike;
    active.enhanced.bike = bike;
    Some(active)
}

/// The shift timing a car flames with: a bike's own short one, a mod car's from its record or the racers', else the record's.
pub(super) fn shift_timing(bike: bool, mod_car: bool, (speed, angle): (f32, f32)) -> ShiftTiming {
    if bike {
        return BIKE_SHIFT_TIMING;
    }
    match mod_car {
        true => mod_timing(speed, angle),
        false => ShiftTiming { speed, angle },
    }
}

/// The shift timing of a mod car: the one its `ecar` record gives, or the racers' when that has none (the class-wide
/// `default` record, which every mod car falls back on, has no shift timing, so the blow-off would never show).
pub(super) fn mod_timing(speed: f32, angle: f32) -> ShiftTiming {
    let timing = ShiftTiming { speed, angle };
    match timing.duration() > 0.0 {
        true => timing,
        false => super::trigger::MOD_SHIFT_TIMING,
    }
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

impl Lane {
    /// The same emitters with a state of their own, for the nitrous flame that is drawn in blue.
    fn blue_copy(&self) -> Lane {
        let emitters = self
            .emitters
            .iter()
            .enumerate()
            .map(|(pipe, source)| {
                let spec = super::scale::compact_nitrous(source.spec());
                let mut emitter = Emitter::new(spec, BLUE_SEED + (self.texture * 16 + pipe) as u64);
                emitter.set_limit(EMITTER_LIMIT);
                emitter
            })
            .collect();
        Lane { texture: self.texture, emitters, vertices: Vec::new() }
    }
}

impl Active {
    pub(super) fn new(
        pipes: Vec<glam::Mat4>,
        lanes: Vec<Lane>,
        textures: Vec<SpriteTexture>,
        timing: ShiftTiming,
        engine_upgrades: i32,
    ) -> Self {
        let blue = lanes.iter().map(Lane::blue_copy).collect();
        Self {
            enhanced: Default::default(),
            pipes,
            lanes,
            blue,
            textures,
            timing,
            engine_upgrades,
            mod_car: false,
            bike: false,
            shift: Default::default(),
            backfire: Default::default(),
            intensity: 0.0,
            nitrous: false,
            live: 0,
            order: Vec::new(),
            handed: Vec::new(),
        }
    }
}
