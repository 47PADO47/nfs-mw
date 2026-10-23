//! The sun-shadow casters that only change when the resident scenery does.
use blackbox_gfx::{Instance, InstanceKey, MeshHandle};
use glam::Mat4;

use super::{admits_scenery, stable_mesh};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::resident::Placed;
use crate::scenes::world::visibility::{self, NOT_A_PROP};

/// A placed prop: where it stands is the prop world's to say each frame (it can be knocked over, hidden or moving).
pub(super) struct PropCaster {
    id: u32,
    /// Where it was placed, for a prop the prop world has no pose for.
    transform: Mat4,
    mesh: Option<MeshHandle>,
    key: InstanceKey,
}

/// What the resident scenery contributes to the shadow casters.
#[derive(Default)]
pub(super) struct Statics {
    /// The leading instances of the caster list that never move, sorted by mesh.
    pub fixed_len: usize,
    /// Placed props, sorted by mesh so the instances they push come out nearly sorted.
    props: Vec<PropCaster>,
}

impl Statics {
    /// Rebuilds from `placed`: the fixed casters replace `out`, sorted by mesh, and the props are listed for
    /// [`push_props`](Self::push_props).
    pub fn build<'a>(&mut self, placed: impl Iterator<Item = &'a Placed>, min_radius: f32, out: &mut Vec<Instance>) {
        out.clear();
        self.props.clear();
        for p in placed {
            if !admits_scenery(p.radius, p.prop_id, min_radius) {
                continue;
            }
            // Authored shadow overlays are receivers/art, not physical occluders.
            if !p.casts_sun_shadow || p.original_shadow {
                continue;
            }
            // Keep one available silhouette for the resident object's lifetime. Colour-view LOD
            // thresholds must not remove off-camera casters or substitute a foliage billboard.
            let mesh = stable_mesh(&p.lods);
            if p.prop_id != NOT_A_PROP {
                self.props.push(PropCaster { id: p.prop_id, transform: p.transform, mesh, key: p.key });
                continue;
            }
            let Some(mesh) = mesh else { continue };
            out.push(Instance::keyed(mesh, p.transform, p.key).with_flipped_winding(p.flip_winding()));
        }
        visibility::sort_by_mesh(out);
        self.fixed_len = out.len();
        self.props.sort_unstable_by_key(|c| c.mesh);
    }

    /// Appends the props that are standing now, where they are drawn this frame.
    pub fn push_props(&self, props: &PropWorld, out: &mut Vec<Instance>) {
        for c in &self.props {
            // A knocked-over prop is not drawn, so it casts nothing.
            if props.hidden(c.id) {
                continue;
            }
            let transform = props.pose(c.id).map_or(c.transform, |(pose, _)| pose);
            if props.model_instances(c.id, transform, out) {
                continue;
            }
            let Some(mesh) = c.mesh else { continue };
            out.push(Instance::keyed(mesh, transform, c.key));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_scene::Aabb;

    fn placed(mesh: Option<usize>, prop_id: u32, index: u32) -> Placed {
        Placed {
            lods: [mesh.map(MeshHandle::from_raw), None, None, None],
            detailed: None,
            transform: Mat4::from_translation(glam::Vec3::splat(index as f32)),
            bounds: Aabb::EMPTY,
            radius: 5.0,
            flags: 0,
            group: 0,
            prop: None,
            prop_id,
            key: InstanceKey::new(1, index),
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            signal: None,
        }
    }

    #[test]
    fn scenery_is_sorted_and_props_are_listed_apart() {
        let mut overlay = placed(Some(9), 0, 3);
        overlay.original_shadow = true;
        let mut no_shadow = placed(Some(9), 0, 4);
        no_shadow.casts_sun_shadow = false;
        let list = [
            placed(Some(7), 0, 0),
            placed(Some(2), 0, 1),
            placed(Some(5), 12, 2),
            overlay,
            no_shadow,
            placed(None, 0, 5),
            placed(None, 13, 6),
        ];
        let (mut statics, mut out) = (Statics::default(), Vec::new());
        statics.build(list.iter(), 0.0, &mut out);
        let meshes: Vec<_> = out.iter().map(|i| i.mesh).collect();
        assert_eq!(meshes, [MeshHandle::from_raw(2), MeshHandle::from_raw(7)]);
        assert_eq!(statics.fixed_len, 2);
        assert_eq!(out[0].key, InstanceKey::new(1, 1));
        let ids: Vec<_> = statics.props.iter().map(|c| c.id).collect();
        assert_eq!(ids, [13, 12], "a prop without a mesh is still listed: its model may supply the meshes");
    }

    #[test]
    fn small_scenery_is_dropped_but_small_props_stay() {
        let mut small = placed(Some(1), 0, 0);
        small.radius = 0.1;
        let mut small_prop = placed(Some(1), 8, 1);
        small_prop.radius = 0.1;
        let list = [small, small_prop];
        let (mut statics, mut out) = (Statics::default(), Vec::new());
        statics.build(list.iter(), 1.0, &mut out);
        assert!(out.is_empty());
        assert_eq!(statics.props.len(), 1);
    }

    #[test]
    fn a_rebuild_replaces_the_previous_list() {
        let list = [placed(Some(1), 0, 0)];
        let (mut statics, mut out) = (Statics::default(), Vec::new());
        statics.build(list.iter(), 0.0, &mut out);
        statics.build(list.iter(), 0.0, &mut out);
        assert_eq!(out.len(), 1);
    }
}
