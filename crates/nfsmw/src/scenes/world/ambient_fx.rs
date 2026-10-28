//! Streamed world particle groups and authored proximity triggers. See `docs/specs/world-emitter-placements.md`.
use std::collections::{BTreeMap, BTreeSet};

use blackbox_attrib::{Database, vlt_hash};
use blackbox_gfx::RenderBackend;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::exhaust::group_emitters;
use nfsmw_data::world::emitter_placements::Placement;

mod admission;
mod budget;

use super::explosion_fx::{Set, build_set, draw};
use admission::{Crossing, visible};

const MAX_PLACEMENTS: usize = 64;
const MAX_EMITTERS: usize = 16;

struct Live {
    group: u32,
    matrix: Mat4,
    emitters: Vec<Emitter>,
    spawning: bool,
}

#[derive(Default)]
pub struct AmbientFx {
    sets: BTreeMap<u32, Option<Set>>,
    live: BTreeMap<(usize, usize), Live>,
    triggers: BTreeMap<(usize, usize), (Placement, Vec<Crossing>)>,
    time: f64,
    random: u64,
    last_service_time: Option<f64>,
    last_views: Vec<(Vec3, Vec3)>,
}

impl AmbientFx {
    pub fn maintain<'a>(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        db: &Database,
        placements: impl Iterator<Item = ((usize, usize), &'a Placement)>,
        views: &[(Vec3, Vec3)],
        actors: &[Vec3],
    ) {
        let simulation_changed = self.last_service_time != Some(self.time);
        let service = simulation_changed || self.last_views.as_slice() != views;
        let placements: Vec<_> = placements.collect();
        let resident: BTreeSet<_> = placements.iter().map(|(id, _)| *id).collect();
        self.triggers.retain(|id, _| resident.contains(id));
        let mut candidates = Vec::new();
        for (id, placement) in &placements {
            if placement.section == 0 {
                continue;
            }
            let Some(group) = db.collection_by_key(vlt_hash("emittergroup"), placement.group) else { continue };
            let Some(far) = group.get_f32("FarClip").filter(|f| f.is_finite() && *f >= 0.0) else { continue };
            let matrix = Mat4::from_cols_array(&placement.matrix);
            let position = matrix.w_axis.truncate();
            // Activity icons already have their own visibility policy and must not compete with scenery.
            let icon = id.0 == usize::MAX - 1;
            let permanent = placement.section == 0xa28 || icon;
            let admitted = permanent || visible(position, far + 20.0, 0.55, views);
            if !admitted && service {
                self.live.remove(id);
            }
            let trigger = self.triggers.entry(*id).or_insert_with(|| {
                ((*placement).clone(), placement.triggers.iter().map(|_| Crossing::default()).collect())
            });
            if trigger.0 != **placement {
                *trigger = ((*placement).clone(), placement.triggers.iter().map(|_| Crossing::default()).collect());
                self.live.remove(id);
            }
            let mut fired = false;
            for actor in actors.iter().filter(|_| simulation_changed) {
                for (spec, crossing) in placement.triggers.iter().zip(&mut trigger.1) {
                    fired |= crossing.enter(spec, *actor, || {
                        self.random = self.random.wrapping_mul(6364136223846793005).wrapping_add(1);
                        ((self.random >> 32) % 65535) as u16
                    });
                }
            }
            if admitted || fired || (!service && self.live.contains_key(id)) {
                let distance = views
                    .iter()
                    .map(|(camera, _)| position.distance_squared(*camera))
                    .min_by(f32::total_cmp)
                    .unwrap_or(f32::INFINITY);
                let spawning = permanent || visible(position, far, 0.7, views);
                candidates.push((!icon, distance, *id, *placement, group, spawning));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)).then_with(|| a.2.cmp(&b.2)));
        // Marker effects have independent lifetimes; the scenery budget cannot evict them.
        let mut admitted = 0;
        candidates.retain(|(scenery, ..)| {
            if !scenery {
                return true;
            }
            admitted += 1;
            admitted <= MAX_PLACEMENTS
        });
        let wanted: BTreeSet<_> = candidates.iter().map(|(_, _, id, ..)| *id).collect();
        self.live.retain(|id, _| wanted.contains(id));
        for (_, _, id, placement, group, spawning) in candidates {
            let matrix = Mat4::from_cols_array(&placement.matrix);
            if self.live.get(&id).is_some_and(|live| live.group == placement.group && live.matrix == matrix) {
                self.live.get_mut(&id).unwrap().spawning = spawning;
                continue;
            }
            self.live.remove(&id);
            let set = self.sets.entry(placement.group).or_insert_with(|| {
                let mut specs = group_emitters(db, group);
                specs.truncate(MAX_EMITTERS);
                build_set(renderer, dir, specs)
            });
            let Some(set) = set else { continue };
            let emitters = set
                .templates
                .iter()
                .enumerate()
                .map(|(i, (_, spec))| {
                    let mut emitter = Emitter::new(
                        spec.clone(),
                        (id.0 as u64).wrapping_mul(4099).wrapping_add(id.1 as u64 * 17 + i as u64 + 1),
                    );
                    emitter.set_limit(budget::capacity(spec));
                    emitter
                })
                .collect();
            self.live.insert(id, Live { group: placement.group, matrix, emitters, spawning });
        }
        for (id, (placement, states)) in self.triggers.iter_mut().filter(|_| simulation_changed) {
            for (spec, state) in placement.triggers.iter().zip(states) {
                state.update(spec, self.live.contains_key(id), self.time);
            }
        }
        self.last_service_time = Some(self.time);
        self.last_views.clear();
        self.last_views.extend_from_slice(views);
    }

    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.time += f64::from(dt);
        for live in self.live.values_mut() {
            let frame = Frame {
                to_world: live.matrix,
                inherit_velocity: Vec3::ZERO,
                intensity: f32::from(u8::from(live.spawning)),
            };
            for emitter in &mut live.emitters {
                emitter.step(dt, &frame);
            }
        }
        self.live
            .retain(|_, live| !live.emitters.iter().all(|emitter| emitter.finished_spawning() && emitter.is_empty()));
    }

    pub fn textured(&self, camera: Vec3, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        for (key, set) in &self.sets {
            let Some(set) = set else { continue };
            draw(
                set,
                self.live.values().filter(|live| live.group == *key).map(|live| live.emitters.as_slice()),
                camera,
                forward,
                out,
            );
        }
    }

    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.live.clear();
        self.triggers.clear();
        self.time = 0.0;
        self.last_service_time = None;
        self.last_views.clear();
        for (_, set) in std::mem::take(&mut self.sets) {
            if let Some(set) = set {
                set.destroy(renderer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_steamjet_keeps_authored_population_above_legacy_limit() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let placement = Placement {
            group: vlt_hash("fxnis_steamjet"),
            section: 1,
            matrix: Mat4::from_translation(Vec3::Y * 10.0).to_cols_array(),
            triggers: vec![],
        };
        let mut fx = AmbientFx::default();
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &[(Vec3::ZERO, Vec3::Y)], &[]);
        let source = &fx.live[&(0, 0)].emitters[0];
        assert_eq!((source.spec().rate, source.spec().life), (100.0, 2.0));
        let mut legacy = source.clone();
        legacy.set_limit(128);
        for _ in 0..360 {
            fx.step(1.0 / 120.0);
            legacy.step(1.0 / 120.0, &Frame::default());
        }
        let emitter = &fx.live[&(0, 0)].emitters[0];
        assert!(emitter.len() > 128 && emitter.len() <= emitter.limit());
        assert!(legacy.len() <= 128 && legacy.len() < emitter.len());
        fx.release(&mut renderer);
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_trigger_libraries_follow_service_before_dispatch() {
        use nfsmw_data::world::emitter_placements::Trigger;
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let mut placement = Placement {
            group: vlt_hash("fxenv_fog1"),
            section: 1,
            matrix: Mat4::from_translation(-Vec3::Y * 10.0).to_cols_array(),
            triggers: vec![Trigger { radius: 2.0, position: [0.0; 3], reset_time: 1.0, probability: u16::MAX }],
        };
        let mut fx = AmbientFx::default();
        let views = [(Vec3::ZERO, Vec3::Y)];
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[Vec3::ZERO]);
        assert_eq!(fx.live.len(), 1, "entry dispatch creates an off-camera group");
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[Vec3::ZERO]);
        assert_eq!(fx.live.len(), 1, "a repeated upload must not retire the group before a simulation tick");
        fx.step(0.1);
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[Vec3::ZERO]);
        assert!(fx.live.is_empty(), "next ordinary service retires the off-camera group before dispatch");
        fx.step(2.0);
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[Vec3::ZERO]);
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[Vec3::ZERO]);
        assert!(fx.live.is_empty(), "rearming does not invent another entry");
        placement.matrix = Mat4::from_translation(Vec3::Y * 10.0).to_cols_array();
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &[]);
        assert_eq!(fx.live.len(), 1, "ordinary service includes triggered libraries without an actor");
        fx.release(&mut renderer);
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_paused_uploads_do_not_reroll_triggers_but_refresh_views_and_retirement() {
        use nfsmw_data::world::emitter_placements::Trigger;
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let placement = Placement {
            group: vlt_hash("fxenv_fog1"),
            section: 1,
            matrix: Mat4::from_translation(-Vec3::Y * 10.0).to_cols_array(),
            triggers: vec![Trigger { radius: 2.0, position: [0.0; 3], reset_time: 1.0, probability: 65534 }],
        };
        let actors = [Vec3::ZERO, Vec3::X * 3.0];
        let mut fx = AmbientFx::default();
        let views = [(Vec3::ZERO, Vec3::Y)];
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &actors);
        let random = fx.random;
        for _ in 0..3 {
            fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &views, &actors);
        }
        assert_eq!(fx.random, random, "the shared player-entry latch must not reroll on paused redraws");
        assert!(fx.live.is_empty());
        let reversed = [(Vec3::ZERO, -Vec3::Y)];
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &reversed, &actors);
        assert_eq!(fx.live.len(), 1, "paused camera movement still admits newly visible scenery");
        assert_eq!(fx.random, random);
        fx.step(0.1);
        fx.maintain(&mut renderer, &dir, &db, std::iter::once(((0, 0), &placement)), &reversed, &actors);
        assert_ne!(fx.random, random, "another simulation tick evaluates players in original order");
        fx.maintain(&mut renderer, &dir, &db, std::iter::empty(), &reversed, &actors);
        assert!(fx.live.is_empty() && fx.triggers.is_empty(), "stream retirement is immediate without another tick");
        fx.release(&mut renderer);
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_marker_effects_do_not_compete_with_scenery_admission() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let scenery = Placement {
            group: vlt_hash("fxenv_fog1"),
            section: 1,
            matrix: Mat4::from_translation(Vec3::Y * 10.0).to_cols_array(),
            triggers: vec![],
        };
        let mut marker = scenery.clone();
        marker.matrix = Mat4::from_translation(-Vec3::Y * 10.0).to_cols_array();
        let mut fx = AmbientFx::default();
        fx.maintain(
            &mut renderer,
            &dir,
            &db,
            (0..MAX_PLACEMENTS + 1).map(|i| ((0, i), &scenery)).chain(std::iter::once(((usize::MAX - 1, 0), &marker))),
            &[(Vec3::ZERO, Vec3::Y)],
            &[],
        );
        assert_eq!(fx.live.len(), MAX_PLACEMENTS + 1);
        assert!(fx.live.contains_key(&(usize::MAX - 1, 0)));
        assert!(fx.live[&(usize::MAX - 1, 0)].spawning);
        fx.release(&mut renderer);
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_fog_and_smoke_groups_emit_and_retire() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let mut renderer =
            crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking();
        let placements: Vec<_> = ["fxenv_fog1", "fxenv_smokestack", "fxenv_small_steam1"]
            .into_iter()
            .map(|name| Placement {
                group: vlt_hash(name),
                section: 1,
                matrix: Mat4::from_translation(Vec3::Y * 10.0).to_cols_array(),
                triggers: Vec::new(),
            })
            .collect();
        let mut fx = AmbientFx::default();
        fx.maintain(
            &mut renderer,
            &dir,
            &db,
            placements.iter().enumerate().map(|(i, p)| ((0, i), p)),
            &[(Vec3::ZERO, Vec3::Y)],
            &[],
        );
        assert_eq!(fx.live.len(), placements.len());
        for _ in 0..120 {
            fx.step(1.0 / 60.0);
        }
        let mut batches = Vec::new();
        fx.textured(Vec3::ZERO, Vec3::Y, &mut batches);
        assert!(!batches.is_empty(), "authored world particles reach draw batches");
        for live in fx.live.values() {
            let set = fx.sets[&live.group].as_ref().unwrap();
            let mut group_batches = Vec::new();
            draw(set, std::iter::once(live.emitters.as_slice()), Vec3::ZERO, Vec3::Y, &mut group_batches);
            assert!(!group_batches.is_empty(), "group {:08x} did not produce particles", live.group);
        }
        fx.maintain(&mut renderer, &dir, &db, std::iter::empty(), &[(Vec3::ZERO, Vec3::Y)], &[]);
        assert!(fx.live.is_empty(), "unloaded placements retire");
        fx.release(&mut renderer);
        assert_eq!(renderer.resources.lock().unwrap().textures, 0);
    }
}
