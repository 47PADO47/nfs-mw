//! Authored world mesh animation. See `docs/specs/world-mesh-animation.md`.
use std::collections::HashMap;

use blackbox_anim::{Clip, Pose, Skeleton};
use blackbox_gfx::{Instance, InstanceKey};
use blackbox_hash::{bstring_hash, bstring_hash_continue};
use game_install::GameDir;
use glam::Mat4;
use nfsmw_data::world::mesh_animation::{Entity, Instance as Placement, WorldAnimations};

use super::residency::Residency;

mod playback;
use playback::Playback;

struct AnimatedInstance {
    placement: Placement,
    entities: Vec<Entity>,
    controllers: Vec<Option<Playback>>,
}

#[derive(Default)]
pub(super) struct MeshAnimations {
    skeleton: Option<Skeleton>,
    clips: HashMap<u32, Clip>,
    instances: Vec<AnimatedInstance>,
}

impl MeshAnimations {
    pub fn load(dir: &GameDir, track: Option<&[u8]>) -> Self {
        match Self::try_load(dir, track) {
            Ok(result) => result,
            Err(error) => {
                log::warn!("world animation unavailable: {error:#}");
                Self::default()
            }
        }
    }

    fn try_load(dir: &GameDir, track: Option<&[u8]>) -> anyhow::Result<Self> {
        use anyhow::Context;
        let track = track.context("track metadata unavailable")?;
        let data = WorldAnimations::read(track)?;
        let objects = nfsmw_data::anim::AnimObjects::from_bytes(track)?;
        let global = nfsmw_data::anim::AnimObjects::load(dir, "GLOBAL/InGameB.bun")?;
        let skeleton = global
            .skeletons
            .into_iter()
            .find(|s| s.name == "ROOT" && s.bones.len() >= 2)
            .context("ROOT skeleton unavailable")?;
        let mut clips = HashMap::new();
        for (object, bank) in &objects.banks {
            for animation in bank.iter() {
                let clip = Clip::read(object, animation)?;
                if clip.skipped.is_empty() {
                    clips.insert(bstring_hash(&clip.name), clip);
                }
            }
        }
        let mut instances = Vec::new();
        for placement in data.instances {
            if placement.track != 0 || placement.flags & 0x600 != 0 {
                continue;
            }
            let Some(entities) = data.trees.get(&placement.tree) else { continue };
            let controllers = entities
                .iter()
                .map(|entity| {
                    let mut frames = None;
                    for (bit, suffix) in [(1, b"_t"), (2, b"_q"), (4, b"_s")] {
                        if entity.flags & bit != 0 {
                            frames = Some(clips.get(&bstring_hash_continue(entity.animation, suffix))?.frames);
                        }
                    }
                    Playback::new(&placement, f32::from(frames?))
                })
                .collect();
            instances.push(AnimatedInstance { placement, entities: entities.clone(), controllers });
        }
        log::info!("world animations: {} instances, {} channels", instances.len(), clips.len());
        Ok(Self { skeleton: Some(skeleton), clips, instances })
    }

    pub fn advance(&mut self, dt: f32, residency: &Residency) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        for item in &mut self.instances {
            if residency.animation_section_visible(item.placement.section) {
                for controller in item.controllers.iter_mut().flatten() {
                    controller.advance(dt);
                }
            }
        }
    }

    pub fn place(&self, residency: &Residency, out: &mut Vec<Instance>) {
        let Some(skeleton) = &self.skeleton else { return };
        for (instance_index, item) in self.instances.iter().enumerate() {
            if !residency.animation_section_visible(item.placement.section) {
                continue;
            }
            let mut parents: Vec<Option<Mat4>> = Vec::with_capacity(item.entities.len());
            for (entity_index, entity) in item.entities.iter().enumerate() {
                let local = if entity.name == item.placement.tree {
                    Some(Mat4::from_cols_array(&item.placement.matrix))
                } else if entity.animation != 0 {
                    item.controllers[entity_index]
                        .as_ref()
                        .filter(|c| c.visible())
                        .and_then(|c| self.sample(entity, skeleton, c.frame / 30.0))
                } else {
                    Some(Mat4::from_cols_array(&entity.matrix))
                };
                let world = local.and_then(|local| match entity.parent {
                    Some(parent) => parents.get(parent).copied().flatten().map(|matrix| matrix * local),
                    None => Some(local),
                });
                parents.push(world);
                if entity.name == item.placement.tree || entity.model == 0 {
                    continue;
                }
                let mesh = residency.animation_mesh(item.placement.section, entity.model).or_else(|| {
                    let channel = self.clips.get(&bstring_hash_continue(entity.animation, b"_q"))?;
                    let (_, part) = channel.name.split_once("_XO_")?;
                    let part = part.strip_suffix("_q")?;
                    let name = format!("XO_{part}").to_ascii_uppercase();
                    residency.animation_mesh(item.placement.section, bstring_hash(&name))
                });
                let Some((mesh, transform)) = world.zip(mesh).map(|(m, h)| (h, m)) else {
                    continue;
                };
                out.push(Instance::keyed(
                    mesh,
                    transform,
                    InstanceKey::new(0x7fff_fffd, (instance_index * 256 + entity_index) as u32),
                ));
            }
        }
    }

    fn sample(&self, entity: &Entity, skeleton: &Skeleton, seconds: f32) -> Option<Mat4> {
        let mut dofs = vec![0.0; skeleton.bones.len() * blackbox_anim::DOFS_PER_BONE];
        skeleton.bind_pose().write_dofs(&mut dofs);
        for (bit, suffix) in [(2, b"_q"), (4, b"_s"), (1, b"_t")] {
            if entity.flags & bit == 0 {
                continue;
            }
            let clip = self.clips.get(&bstring_hash_continue(entity.animation, suffix))?;
            let duration = clip.duration_secs();
            if !duration.is_finite() || duration <= 0.0 {
                return None;
            }
            clip.sample(seconds * 30.0 / clip.fps, &mut dofs);
        }
        let pose = Pose::from_dofs(&dofs);
        skeleton.world(&pose).get(1).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs the installed world animation assets"]
    fn installed_world_channels_produce_motion() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let track = nfsmw_data::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
        let animations = MeshAnimations::try_load(&dir, Some(&track)).unwrap();
        let skeleton = animations.skeleton.as_ref().unwrap();
        assert_eq!(animations.instances.len(), 11);
        let mut count = 0;
        for item in &animations.instances {
            for entity in item.entities.iter().filter(|entity| entity.animation != 0) {
                let clip = animations.clips.get(&bstring_hash_continue(entity.animation, b"_q")).unwrap();
                let first = animations.sample(entity, skeleton, 0.0).unwrap();
                let later = animations.sample(entity, skeleton, clip.duration_secs() * 0.5).unwrap();
                assert!(first.is_finite() && later.is_finite());
                assert!(!first.abs_diff_eq(later, 1e-5), "animated entity must move");
                count += 1;
            }
        }
        assert_eq!(count, 13);
    }

    #[test]
    #[ignore = "needs the installed world and streams its crane area"]
    fn installed_crane_streams_and_reaches_the_view() {
        use crate::input::ActionState;
        use crate::viewer::Scene;
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let mut scene = super::super::WorldScene::open(
            &dir,
            super::super::Options {
                at: Some([4006.8, 1135.8]),
                height: 50.0,
                heading: 0.0,
                pitch: 0.0,
                fog_distance: 1500.0,
                wait_for_load: true,
                drive: None,
            },
        )
        .unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        scene.init(&mut renderer).unwrap();
        for _ in 0..2000 {
            scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
            if scene.residency.complete() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(scene.residency.complete());
        let mut first = Vec::new();
        scene.mesh_animations.place(&scene.residency, &mut first);
        assert!(!first.is_empty(), "a visible installed world animation must reach the renderer");
        for suffix in ["10", "11", "12"] {
            let hash = bstring_hash(&format!("XO_AIRLINERA_1B_LB_{suffix}"));
            let mesh = scene.residency.animation_mesh(2600, hash).expect("airliner part must be resident");
            assert!(first.iter().any(|instance| instance.mesh == mesh), "airliner part {suffix} must be placed");
        }
        for item in &mut scene.mesh_animations.instances {
            for controller in item.controllers.iter_mut().flatten() {
                controller.advance(1.0);
            }
        }
        let mut second = Vec::new();
        scene.mesh_animations.place(&scene.residency, &mut second);
        assert!(
            first.iter().any(|a| second.iter().any(|b| a.key == b.key && !a.transform.abs_diff_eq(b.transform, 1e-5))),
            "a streamed animated mesh changes its transform"
        );
        scene.close(&mut renderer);
    }

    #[test]
    #[ignore = "needs the installed world and streams the donut shop"]
    fn installed_donut_sign_rotates_in_place() {
        use crate::input::ActionState;
        use crate::viewer::Scene;
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let mut scene = super::super::WorldScene::open(
            &dir,
            super::super::Options {
                at: Some([1906.3, 1859.1]),
                height: 170.0,
                heading: 0.0,
                pitch: 0.0,
                fog_distance: 1500.0,
                wait_for_load: true,
                drive: None,
            },
        )
        .unwrap();
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        scene.init(&mut renderer).unwrap();
        for _ in 0..2000 {
            scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
            if scene.residency.complete() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(scene.residency.complete());
        let sign = blackbox_hash::bstring_hash("XB_DONUTSIGNB_1B_BK_00");
        let placed = scene
            .residency
            .placed()
            .find(|p| p.lods.iter().flatten().any(|mesh| scene.residency.animation_mesh(821, sign) == Some(*mesh)))
            .expect("installed donut sign must be resident");
        let mesh = scene.residency.animation_mesh(821, sign).unwrap();
        let mut instances = vec![Instance::keyed(mesh, placed.transform, placed.key)];
        let before = instances[0].transform;
        scene.residency.animate_simple_models(&mut instances, 1.0);
        assert!(!before.abs_diff_eq(instances[0].transform, 1e-5));
        assert!(before.w_axis.abs_diff_eq(instances[0].transform.w_axis, 1e-5));
        scene.close(&mut renderer);
    }
}
