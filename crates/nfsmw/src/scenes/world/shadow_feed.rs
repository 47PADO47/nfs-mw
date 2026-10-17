//! Shadow candidates from resident geometry, including tiles outside the camera's drawn set.
use super::WorldScene;
use blackbox_gfx::{Instance, RenderBackend};

impl WorldScene {
    pub(super) fn update_shadow_casters(&mut self, renderer: &mut dyn RenderBackend) {
        let settings = renderer.render_experiments();
        let _span = bevy_log::info_span!("world::shadow_feed").entered();
        if settings.sun_shadows || settings.contact_shadows || settings.car_skylight {
            renderer.set_lighting_rig(&crate::scenes::car::lighting::rig(self.sun.to_sun()));
        }
        self.shadow_casters.clear();
        if !settings.sun_shadows {
            renderer.set_shadow_casters(None);
            return;
        }
        let props = self.residency.props();
        for p in self.residency.shadow_placed() {
            // Authored shadow overlays are receivers/art, not physical occluders.
            if p.original_shadow || props.hidden(p.prop_id) {
                continue;
            }
            let transform = props.pose(p.prop_id).map_or(p.transform, |pose| pose.0);
            if props.model_instances(p.prop_id, transform, &mut self.shadow_casters) {
                continue;
            }
            // Keep one available silhouette for the resident object's lifetime. Colour-view LOD
            // thresholds must not remove off-camera casters or substitute a foliage billboard.
            let Some(mesh) = stable_mesh(&p.lods) else { continue };
            self.shadow_casters.push(Instance::keyed(mesh, transform, p.key));
        }
        props.fragment_instances(&mut self.shadow_casters);
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut self.shadow_casters);
        }
        if let Some(drive) = &self.drive {
            drive.instances(&mut self.shadow_casters);
        }
        self.shadow_casters.sort_unstable_by_key(|i| i.mesh);
        renderer.set_shadow_casters(Some(&self.shadow_casters));
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
