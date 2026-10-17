//! Shadow candidates from resident geometry, including tiles outside the camera's drawn set.
use super::WorldScene;
use super::visibility::{self, NOT_A_PROP};
use blackbox_gfx::{Instance, RenderBackend};

/// The sun-shadow casters for the current frame, and whether the renderer draws shadows from them.
#[derive(Default)]
pub(super) struct ShadowFeed {
    pub casters: Vec<Instance>,
    /// True once the renderer reads `casters` for its sun shadows, so the frame lists need not carry
    /// the off-camera casters.
    pub fed: bool,
}

impl WorldScene {
    /// Apply a changed mode to existing actors, including detached cargo snapshots.
    pub(super) fn update_car_shading(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(drive) = &mut self.drive {
            let replacements = drive.set_car_shading(renderer, self.car_shading, self.sun.to_sun());
            self.residency.world_parts().1.remap_fragment_meshes(&replacements);
        }
        if let Some(traffic) = &mut self.traffic {
            traffic.reshade(renderer, self.car_shading, self.sun.to_sun(), self.residency.world_parts().1);
        }
    }

    pub(super) fn update_shadow_casters(&mut self, renderer: &mut dyn RenderBackend) {
        let _span = bevy_log::info_span!("world::shadow_feed").entered();
        let settings = renderer.render_experiments();
        if self.car_shading.is_original() {
            renderer.set_lighting_rig(&crate::scenes::car::original_lighting::gameplay(
                self.sun.to_sun(),
                self.car_specular_scale(),
            ));
        }
        if !self.car_shading.is_original()
            && (settings.sun_shadows || settings.contact_shadows || settings.car_skylight)
        {
            renderer.set_lighting_rig(&crate::scenes::car::lighting::rig(self.sun.to_sun()));
        }
        self.shadow_feed.casters.clear();
        self.shadow_feed.fed = false;
        if !settings.sun_shadows {
            renderer.set_shadow_casters(None);
            return;
        }
        let props = self.residency.props();
        for p in self.residency.shadow_placed() {
            // Authored shadow overlays are receivers/art, not physical occluders.
            if !p.casts_sun_shadow || p.original_shadow || props.hidden(p.prop_id) {
                continue;
            }
            // A knocked-over prop is not drawn, so it casts nothing. A non-prop has no entry in `props`.
            let Some((transform, _)) = visibility::current_pose(p, props) else {
                continue;
            };
            if p.prop_id != NOT_A_PROP && props.model_instances(p.prop_id, transform, &mut self.shadow_feed.casters) {
                continue;
            }
            // Keep one available silhouette for the resident object's lifetime. Colour-view LOD
            // thresholds must not remove off-camera casters or substitute a foliage billboard.
            let Some(mesh) = stable_mesh(&p.lods) else { continue };
            self.shadow_feed.casters.push(Instance::keyed(mesh, transform, p.key));
        }
        props.fragment_instances(&mut self.shadow_feed.casters);
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut self.shadow_feed.casters);
        }
        if let Some(drive) = &self.drive {
            drive.instances(&mut self.shadow_feed.casters);
        }
        visibility::sort_by_mesh(&mut self.shadow_feed.casters);
        self.shadow_feed.fed = renderer.capabilities().separate_shadow_casters;
        renderer.set_shadow_casters(Some(&self.shadow_feed.casters));
    }
}

fn stable_mesh<T: Copy>(lods: &[Option<T>; 4]) -> Option<T> {
    lods.iter().flatten().next().copied()
}

#[cfg(test)]
mod tests {
    use super::stable_mesh;

    #[test]
    fn a_missing_coarse_lod_never_removes_a_detailed_caster() {
        assert_eq!(stable_mesh(&[Some(7), None, None, None]), Some(7));
        assert_eq!(stable_mesh(&[None, None, Some(9), None]), Some(9));
        assert_eq!(stable_mesh(&[Some(7), Some(8), Some(9), None]), Some(7));
        assert_eq!(stable_mesh::<u8>(&[None; 4]), None);
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
