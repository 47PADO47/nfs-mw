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
    pub(super) fn update_shadow_casters(&mut self, renderer: &mut dyn RenderBackend) {
        let _span = bevy_log::info_span!("world::shadow_feed").entered();
        let settings = renderer.render_experiments();
        if settings.sun_shadows || settings.contact_shadows || settings.car_skylight {
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
            if p.original_shadow {
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
}
