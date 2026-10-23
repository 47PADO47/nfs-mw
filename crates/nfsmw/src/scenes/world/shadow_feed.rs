//! Shadow candidates from resident geometry, including tiles outside the camera's drawn set.
//!
//! Nearly all of the resident geometry is static, so its caster list is built once per scenery change
//! ([`statics`]); a frame only appends the casters that can move: props, fragments, traffic and the player's car.
use super::resident::Placed;
use super::scenery_groups::SceneryGroups;
use super::visibility::{self, NOT_A_PROP};
use super::{Residency, WorldScene};
use blackbox_gfx::{Instance, RenderBackend};
use glam::Vec3;

mod cache;
mod rig;
mod statics;

/// The sun-shadow casters for the current frame, and whether the renderer draws shadows from them.
#[derive(Default)]
pub(super) struct ShadowFeed {
    /// The static casters (`statics.value.fixed_len` of them, sorted by mesh), then this frame's moving ones.
    pub casters: Vec<Instance>,
    /// True once the renderer reads `casters` for its sun shadows, so the frame lists need not carry
    /// the off-camera casters.
    pub fed: bool,
    /// The static part of `casters`, and what it was built for.
    statics: cache::Cached<statics::Statics>,
    /// The car lighting rig the renderer has.
    rig: rig::RigMemory,
    /// The curve warning sign lamps of the drawn scenery.
    curve_lamps: cache::Cached<Vec<Vec3>>,
}

impl ShadowFeed {
    /// Rebuilds the static casters from `placed` unless the scenery is the one they were built for. Returns whether
    /// it did; `placed` is not read otherwise.
    fn refresh_statics<'a>(
        &mut self,
        revision: u64,
        groups: &SceneryGroups,
        min_radius: f32,
        placed: impl Iterator<Item = &'a Placed>,
    ) -> bool {
        let casters = &mut self.casters;
        self.statics
            .refresh(revision, groups, min_radius.to_bits(), |statics| statics.build(placed, min_radius, casters))
    }

    /// The lamps of the curve warning signs the residency draws, listed again only when the scenery changes.
    pub(super) fn curve_lamps(&mut self, residency: &Residency, groups: &SceneryGroups) -> &[Vec3] {
        self.curve_lamps.refresh(residency.visual_revision(), groups, 0, |lamps| {
            lamps.clear();
            lamps.extend(residency.placed().filter_map(|p| p.curve_lamp));
        });
        &self.curve_lamps.value
    }
}

impl WorldScene {
    /// Apply a changed mode to existing actors, including detached cargo snapshots.
    pub(super) fn update_car_shading(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(drive) = self.players.lead_mut() {
            let replacements = drive.set_car_shading(renderer, self.car_shading, self.sun.to_sun());
            self.residency.world_parts().1.remap_fragment_meshes(&replacements);
        }
        if let Some(traffic) = &mut self.traffic {
            traffic.reshade(renderer, self.car_shading, self.sun.to_sun(), self.residency.world_parts().1);
        }
        // Uploading the cars' materials set the renderer's rig.
        self.shadow_feed.rig.forget();
    }

    pub(super) fn update_shadow_casters(&mut self, renderer: &mut dyn RenderBackend) {
        let _trace = blackbox_trace::scope("World: prepare shadow casters");
        let _span = bevy_log::info_span!("world::shadow_feed").entered();
        let settings = renderer.render_experiments();
        if self.car_shading.is_original() {
            let rig = crate::scenes::car::original_lighting::gameplay(self.sun.to_sun(), self.car_specular_scale());
            self.shadow_feed.rig.send(renderer, &rig);
        }
        if !self.car_shading.is_original() {
            let rig = crate::scenes::car::lighting::world_rig(
                self.sun.to_sun(),
                settings.weather_car_lighting.then(|| self.surface_light()).flatten(),
            );
            self.shadow_feed.rig.send(renderer, &rig);
        }
        self.shadow_feed.fed = false;
        if !settings.sun_shadows {
            self.shadow_feed.casters.clear();
            self.shadow_feed.statics.invalidate();
            renderer.set_shadow_casters(None);
            return;
        }
        self.shadow_feed.refresh_statics(
            self.residency.visual_revision(),
            &self.scenery_groups,
            settings.shadow_caster_min_radius,
            self.residency.shadow_placed(),
        );
        let feed = &mut self.shadow_feed;
        let fixed = feed.statics.value.fixed_len;
        feed.casters.truncate(fixed);
        let props = self.residency.props();
        feed.statics.value.push_props(props, &mut feed.casters);
        props.fragment_instances(&mut feed.casters);
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut feed.casters);
        }
        if let Some(drive) = self.players.lead() {
            drive.instances(&mut feed.casters);
        }
        // The static casters are already sorted; only what this frame added needs it.
        visibility::sort_by_mesh(&mut feed.casters[fixed..]);
        feed.fed = renderer.capabilities().separate_shadow_casters;
        renderer.set_shadow_casters(Some(&feed.casters));
    }
}

fn stable_mesh<T: Copy>(lods: &[Option<T>; 4]) -> Option<T> {
    lods.iter().flatten().next().copied()
}

fn admits_scenery(radius: f32, prop_id: u32, minimum: f32) -> bool {
    // Keep dynamic props, unknown radii, and the full-detail default intact.
    prop_id != NOT_A_PROP || minimum <= 0.0 || radius <= 0.0 || !radius.is_finite() || radius >= minimum
}

#[cfg(test)]
mod tests {
    use super::{ShadowFeed, stable_mesh};
    use crate::scenes::world::resident::Placed;
    use crate::scenes::world::scenery_groups::SceneryGroups;
    use blackbox_gfx::{InstanceKey, MeshHandle};

    #[test]
    fn scenery_detail_preserves_dynamic_props_and_the_full_detail_default() {
        assert!(super::admits_scenery(0.1, 0, 0.0));
        assert!(!super::admits_scenery(0.1, 0, 1.0));
        assert!(super::admits_scenery(0.1, 12, 1.0));
        assert!(super::admits_scenery(1.0, 0, 1.0));
        assert!(super::admits_scenery(f32::NAN, 0, 1.0));
        assert!(super::admits_scenery(0.0, 0, 1.0));
    }

    #[test]
    fn a_missing_coarse_lod_never_removes_a_detailed_caster() {
        assert_eq!(stable_mesh(&[Some(7), None, None, None]), Some(7));
        assert_eq!(stable_mesh(&[None, None, Some(9), None]), Some(9));
        assert_eq!(stable_mesh(&[Some(7), Some(8), Some(9), None]), Some(7));
        assert_eq!(stable_mesh::<u8>(&[None; 4]), None);
    }

    fn scenery(mesh: usize, index: u32) -> Placed {
        Placed {
            lods: [Some(MeshHandle::from_raw(mesh)), None, None, None],
            detailed: None,
            transform: glam::Mat4::from_translation(glam::Vec3::splat(index as f32)),
            bounds: blackbox_scene::Aabb::EMPTY,
            radius: 5.0,
            flags: 0,
            group: 0,
            prop: None,
            prop_id: 0,
            key: InstanceKey::new(1, index),
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            curve_lamp: None,
            signal: None,
        }
    }

    #[test]
    fn the_static_casters_are_rebuilt_only_when_the_scenery_changes() {
        let mut feed = ShadowFeed::default();
        let groups = SceneryGroups::default();
        let list = [scenery(3, 0), scenery(1, 1)];
        let walked = std::cell::Cell::new(0);
        let refresh = |feed: &mut ShadowFeed, revision, list: &[Placed]| {
            feed.refresh_statics(revision, &groups, 0.0, list.iter().inspect(|_| walked.set(walked.get() + 1)))
        };
        assert!(refresh(&mut feed, 4, &list));
        assert_eq!(walked.get(), 2);
        assert_eq!(feed.casters.len(), 2);
        assert_eq!(feed.casters[0].mesh, MeshHandle::from_raw(1), "sorted by mesh");
        for _ in 0..3 {
            assert!(!refresh(&mut feed, 4, &list), "an unchanged revision does not rebuild");
        }
        assert_eq!(walked.get(), 2, "and does not walk the placed objects");
        assert!(refresh(&mut feed, 5, &list[..1]), "a changed revision rebuilds");
        assert_eq!(feed.casters.len(), 1);
    }

    #[test]
    fn the_curve_lamp_list_follows_the_same_stamp() {
        let mut feed = ShadowFeed::default();
        assert!(feed.curve_lamps.value.is_empty());
        let groups = SceneryGroups::default();
        assert!(feed.curve_lamps.refresh(1, &groups, 0, |lamps| lamps.push(glam::Vec3::X)));
        assert!(!feed.curve_lamps.refresh(1, &groups, 0, |lamps| lamps.push(glam::Vec3::Y)));
        assert_eq!(feed.curve_lamps.value, [glam::Vec3::X]);
    }

    #[test]
    #[ignore = "needs installed lighting presets selected by NFSMW_GAME_DIR"]
    fn installed_original_car_rig_updates_without_custom_lighting() {
        use super::super::{Options, WorldScene};
        use crate::app::test_backend::NullBackend;
        use crate::settings::CarShading;
        use blackbox_gfx::RenderBackend;
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let mut scene = WorldScene::open(
            &dir,
            Options {
                at: None,
                height: 40.0,
                heading: 0.0,
                pitch: -15.0,
                fog_distance: 1500.0,
                wait_for_load: false,
                drive: None,
            },
        )
        .unwrap();
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let experiments = renderer.render_experiments();
        assert!(!experiments.sun_shadows && !experiments.contact_shadows && !experiments.car_skylight);
        for mode in [CarShading::Original, CarShading::OriginalUnshadowed] {
            scene.car_shading = mode;
            scene.sun.set_phase(0.5);
            scene.update_shadow_casters(&mut renderer);
            let first = renderer.lighting_rig.unwrap();
            assert_eq!(first.material_specular_scale, scene.car_specular_scale());
            assert!(first.material_specular_scale.is_finite());
            scene.sun.set_phase(0.15);
            scene.update_shadow_casters(&mut renderer);
            let next = renderer.lighting_rig.unwrap();
            assert_ne!(first.lights, next.lights);
            assert_eq!(next.material_specular_scale, scene.car_specular_scale());
        }
        let authored = scene.time_of_day.take().expect("installed presets");
        scene.update_shadow_casters(&mut renderer);
        assert_eq!(
            renderer.lighting_rig.unwrap().material_specular_scale,
            1.0,
            "unavailable authored inputs use neutral fallback"
        );
        scene.time_of_day = Some(authored);
        eprintln!(
            "original car lighting: {} per-frame updates with every custom option off",
            renderer.lighting_updates
        );
    }
}
