//! Streamed, untriggered world particle groups. See `docs/specs/world-emitter-placements.md`.
use std::collections::{BTreeMap, BTreeSet};

use blackbox_attrib::{Database, vlt_hash};
use blackbox_gfx::RenderBackend;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::TexturedEffect;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::exhaust::group_emitters;
use nfsmw_data::world::emitter_placements::Placement;

use super::explosion_fx::{Set, build_set, draw};

const MAX_PLACEMENTS: usize = 64;
const MAX_EMITTERS: usize = 16;
const PARTICLES_PER_EMITTER: usize = 128;

struct Live {
    group: u32,
    matrix: Mat4,
    emitters: Vec<Emitter>,
}

#[derive(Default)]
pub struct AmbientFx {
    sets: BTreeMap<u32, Option<Set>>,
    live: BTreeMap<(usize, usize), Live>,
}

impl AmbientFx {
    pub fn maintain<'a>(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        db: &Database,
        placements: impl Iterator<Item = ((usize, usize), &'a Placement)>,
        camera: Vec3,
    ) {
        let mut candidates = Vec::new();
        for (id, placement) in placements {
            if placement.section == 0 || placement.section == 0xa28 || !placement.triggers.is_empty() {
                continue;
            }
            let Some(group) = db.collection_by_key(vlt_hash("emittergroup"), placement.group) else { continue };
            let Some(far) = group.get_f32("FarClip").filter(|f| f.is_finite() && *f >= 0.0) else { continue };
            let matrix = Mat4::from_cols_array(&placement.matrix);
            let distance = matrix.w_axis.truncate().distance_squared(camera);
            if distance <= (far + 20.0).powi(2) {
                candidates.push((distance, id, placement, group));
            }
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        candidates.truncate(MAX_PLACEMENTS);
        let wanted: BTreeSet<_> = candidates.iter().map(|(_, id, _, _)| *id).collect();
        self.live.retain(|id, _| wanted.contains(id));
        for (_, id, placement, group) in candidates {
            let matrix = Mat4::from_cols_array(&placement.matrix);
            if self.live.get(&id).is_some_and(|live| live.group == placement.group && live.matrix == matrix) {
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
                    emitter.set_limit(PARTICLES_PER_EMITTER);
                    emitter
                })
                .collect();
            self.live.insert(id, Live { group: placement.group, matrix, emitters });
        }
    }

    pub fn step(&mut self, dt: f32) {
        for live in self.live.values_mut() {
            let frame = Frame { to_world: live.matrix, inherit_velocity: Vec3::ZERO, intensity: 1.0 };
            for emitter in &mut live.emitters {
                emitter.step(dt, &frame);
            }
        }
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
        fx.maintain(&mut renderer, &dir, &db, placements.iter().enumerate().map(|(i, p)| ((0, i), p)), Vec3::ZERO);
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
        fx.maintain(&mut renderer, &dir, &db, std::iter::empty(), Vec3::ZERO);
        assert!(fx.live.is_empty(), "unloaded placements retire");
        fx.release(&mut renderer);
        assert_eq!(renderer.resources.lock().unwrap().textures, 0);
    }
}
