//! The flares at a race's finish line: the finish trigger's `ParticleEffect` (`fxgame_flare_red`, an `emittergroup` of the
//! install) run at each of the trigger's flare points, from the start of the race until the race is taken down. Spec:
//! `docs/specs/race-finish-effects.md` section 1.
//!
//! The effect's emitters are the original's data; they stand still (render space, the effect's z axis up) and step with
//! the fixed physics tick, so the flares are frozen with the world behind the post-race screens.

use blackbox_attrib::Database;
use blackbox_gfx::RenderBackend;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::finish_fx::trigger_effect_emitters;

use super::explosion_fx::{Set, build_set, draw};

/// Most live particles of one emitter of one flare (the smoke lives 4 s at 8 a second).
const LIMIT: usize = 64;

/// The effect's textures, once uploaded, and the name of the effect they belong to. `set` is `None` when the install has
/// no such effect or no art for it.
struct Loaded {
    effect: String,
    set: Option<Set>,
}

/// The flares of the running race.
#[derive(Default)]
pub struct FinishFlares {
    loaded: Option<Loaded>,
    /// The effect the race wants (the finish trigger's `ParticleEffect`) and where it stands, render space.
    wanted: Option<(String, Vec<Vec3>)>,
    /// One group of emitters per flare point; empty until the next step builds them.
    flares: Vec<Vec<Emitter>>,
    /// How many times the flares were started, for the emitters' random seeds.
    started: u64,
}

impl FinishFlares {
    /// The race wants `effect` at these points (render space). Replaces what was shown before; the particles start over.
    pub fn show(&mut self, effect: &str, points: Vec<Vec3>) {
        self.wanted = Some((effect.to_owned(), points));
        self.flares.clear();
        self.started += 1;
    }

    /// The race is over or gone: the flares go out. The textures stay for the next race.
    pub fn clear(&mut self) {
        self.wanted = None;
        self.flares.clear();
    }

    /// Whether the flares are wanted now.
    #[cfg(test)]
    pub fn is_shown(&self) -> bool {
        self.wanted.is_some()
    }

    /// Where the flares stand (render space).
    #[cfg(test)]
    pub fn points(&self) -> &[Vec3] {
        self.wanted.as_ref().map_or(&[], |(_, points)| points)
    }

    /// Uploads the wanted effect's textures when they are not up yet (once per effect); with no art the flares stay off.
    pub fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database) {
        let Some(effect) = self.wanted.as_ref().map(|(effect, _)| effect.clone()) else { return };
        if self.loaded.as_ref().is_some_and(|loaded| loaded.effect == effect) {
            return;
        }
        self.release(renderer);
        let emitters = trigger_effect_emitters(db, &effect);
        let set = match emitters.is_empty() {
            true => {
                log::info!("no finish line effect {effect} in the install");
                None
            }
            false => build_set(renderer, dir, emitters),
        };
        log::info!(
            "finish line effect {effect}: {} emitter(s) with art, {} flare(s)",
            set.as_ref().map_or(0, |set| set.templates.len()),
            self.wanted.as_ref().map_or(0, |(_, points)| points.len())
        );
        self.loaded = Some(Loaded { effect, set });
        self.flares.clear();
    }

    /// Gives back the textures. The effect is loaded again the next time a race wants it.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.flares.clear();
        if let Some(Loaded { set: Some(set), .. }) = self.loaded.take() {
            set.destroy(renderer);
        }
    }

    /// One fixed physics tick.
    pub fn step(&mut self, dt: f32) {
        let (Some((effect, points)), Some(loaded)) = (&self.wanted, &self.loaded) else { return };
        let Some(set) = loaded.set.as_ref().filter(|_| &loaded.effect == effect) else { return };
        if self.flares.is_empty() {
            self.flares = points
                .iter()
                .enumerate()
                .map(|(flare, _)| {
                    set.templates
                        .iter()
                        .enumerate()
                        .map(|(i, (_, spec))| {
                            let seed = self.started * 1024 + flare as u64 * 16 + i as u64 + 1;
                            let mut emitter = Emitter::new(spec.clone(), seed);
                            emitter.set_limit(LIMIT);
                            emitter
                        })
                        .collect()
                })
                .collect();
        }
        for (emitters, &at) in self.flares.iter_mut().zip(points) {
            let frame = Frame { to_world: Mat4::from_translation(at), inherit_velocity: Vec3::ZERO, intensity: 1.0 };
            for emitter in emitters {
                emitter.step(dt, &frame);
            }
        }
    }

    /// Appends one textured batch per texture with live particles, the sprites turned to face the camera.
    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let Some(set) = self.loaded.as_ref().and_then(|loaded| loaded.set.as_ref()) else { return };
        if self.wanted.is_none() {
            return;
        }
        draw(set, self.flares.iter().map(Vec::as_slice), camera, forward, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showing_flares_replaces_the_points_and_clearing_removes_them() {
        let mut flares = FinishFlares::default();
        assert!(!flares.is_shown());
        flares.show("fxgame_flare_red", vec![Vec3::X, Vec3::Y]);
        assert!(flares.is_shown());
        assert_eq!(flares.points(), [Vec3::X, Vec3::Y]);
        flares.show("fxgame_flare_red", vec![Vec3::Z]);
        assert_eq!(flares.points(), [Vec3::Z]);
        flares.clear();
        assert!(!flares.is_shown());
        assert!(flares.points().is_empty());
    }

    #[test]
    fn a_step_with_no_art_does_nothing() {
        let mut flares = FinishFlares::default();
        flares.show("fxgame_flare_red", vec![Vec3::X]);
        flares.step(1.0 / 120.0);
        let mut out = Vec::new();
        flares.textured(Vec3::ZERO, Vec3::X, &mut out);
        assert!(out.is_empty());
    }
}
