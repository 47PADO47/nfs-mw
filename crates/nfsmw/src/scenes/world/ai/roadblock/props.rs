//! The props of a roadblock: the sawhorse barriers and the spike strips (`XO_Sawhorse_1b_00`, `XO_SpikeBelt_1b_DW_00`).
//! Both models are in the track's shared scenery; the collision boxes and the sawhorse's dynamics are the same tables a
//! map placement joins. A sawhorse is put in the prop world as a body that sleeps until something reaches it; a spike
//! strip is tested by the tire box (`blackbox_pursuit::spikes`) and drawn from its model. Spec:
//! `docs/specs/ai-pursuit-roadblocks.md` sections 2.5 and 6.1.

use std::collections::BTreeSet;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use blackbox_collision::BoundsSet;
use blackbox_render::Instance;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{PropCatalog, PropDynamics};

use crate::scenes::world::drive::{CarPose, CarRig};
use crate::scenes::world::props::scenery::ModelParts;
use crate::scenes::world::props::{PlacedSpawn, PropWorld};
use crate::scenes::world::space;

/// The model of the sawhorse barrier (`kBarrier`), and the `smackable` collection that gives it its dynamics.
pub const SAWHORSE_MODEL: &str = "XO_Sawhorse_1b_00";
pub const SAWHORSE_SMACKABLE: &str = "sawhorse";
/// The model of the spike strip (`kSpikeStrip`).
pub const SPIKE_MODEL: &str = "XO_SpikeBelt_1b_DW_00";

/// The uploaded models.
pub(in crate::scenes::world::ai) struct Kit {
    spike: Rc<CarRig>,
    /// The sawhorse's meshes in its own frame, as the prop world draws a body.
    sawhorse_local: Vec<Instance>,
    /// The sawhorse's fragments, for its event sequence (the hierarchy of `SAWHORSE_MODEL`).
    sawhorse_parts: Option<Arc<ModelParts>>,
}

impl Kit {
    pub fn new(spike: Rc<CarRig>, sawhorse: Rc<CarRig>) -> Self {
        let mut sawhorse_local = Vec::new();
        sawhorse.instances(&pose(Mat4::IDENTITY), None, &BTreeSet::new(), &mut sawhorse_local);
        Self { spike, sawhorse_local, sawhorse_parts: None }
    }

    /// The parts the sawhorse's sequence releases when it breaks.
    pub fn set_sawhorse_parts(&mut self, parts: Option<Arc<ModelParts>>) {
        self.sawhorse_parts = parts;
    }

    /// The spike strip drawn at `frame` (render space).
    pub fn draw_strip(&self, frame: Mat4, out: &mut Vec<Instance>) {
        self.spike.instances(&pose(frame), None, &BTreeSet::new(), out);
    }
}

/// What the install says about the props' bodies.
pub(in crate::scenes::world::ai) struct Physics {
    pub sawhorse: Option<(Arc<BoundsSet>, Arc<PropDynamics>)>,
    /// Half extents of the spike strip's collision box in model axes: along its facing, across it, up.
    pub strip_half: Option<[f32; 3]>,
}

impl Physics {
    pub fn from_catalog(catalog: &PropCatalog) -> Self {
        let strip_half = catalog
            .shape(SPIKE_MODEL)
            .and_then(|shape| shape.boxes.first().map(|b| b.half))
            .map(|half| [half.x, half.y, half.z]);
        Self { sawhorse: catalog.placeable(SAWHORSE_MODEL, SAWHORSE_SMACKABLE), strip_half }
    }
}

/// A rig pose from a render-space frame.
fn pose(frame: Mat4) -> CarPose {
    let (_, rotation, position) = frame.to_scale_rotation_translation();
    CarPose { position, rotation, wheels: Default::default() }
}

/// The frame of a prop in render space: its origin at `position` (physics space), its +x along `heading` (radians in
/// the physics x-z plane, as the layout gives it) tilted onto the ground and its up along the ground `normal`.
pub fn frame(position: Vec3, heading: f32, normal: Vec3) -> Mat4 {
    let up = normal.try_normalize().unwrap_or(Vec3::Y);
    let level = Vec3::new(heading.cos(), 0.0, heading.sin());
    let forward = (level - up * level.dot(up)).try_normalize().unwrap_or(level);
    let render = |v: Vec3| space::to_render(v.to_array());
    let (forward, up) = (render(forward), render(up));
    let left = up.cross(forward);
    Mat4::from_cols(forward.extend(0.0), left.extend(0.0), up.extend(0.0), render(position).extend(1.0))
}

impl Kit {
    /// Puts a sawhorse at `frame` in the prop world. Returns its id, `None` without the install's bounds or with a
    /// body the solver cannot use.
    pub fn place_sawhorse(
        &self,
        physics: &Physics,
        props: &mut PropWorld,
        frame: Mat4,
        owner: Weak<()>,
    ) -> Option<u32> {
        let (bounds, spec) = physics.sawhorse.clone()?;
        props.spawn_placed(PlacedSpawn {
            bounds,
            spec,
            model: frame,
            instances: self.sawhorse_local.clone(),
            owner,
            parts: self.sawhorse_parts.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prop_faces_its_heading_and_stands_on_the_ground_normal() {
        // Heading 0 is physics +x; on level ground the frame's forward is that, in render axes.
        let f = frame(Vec3::new(1.0, 2.0, 3.0), 0.0, Vec3::Y);
        let forward = f.transform_vector3(Vec3::X);
        let up = f.transform_vector3(Vec3::Z);
        let expect_forward = space::to_render([1.0, 0.0, 0.0]);
        let expect_up = space::to_render([0.0, 1.0, 0.0]);
        assert!((forward - expect_forward).length() < 1e-5);
        assert!((up - expect_up).length() < 1e-5);
        assert!((f.w_axis.truncate() - space::to_render([1.0, 2.0, 3.0])).length() < 1e-5);
        assert!(f.to_scale_rotation_translation().1.is_normalized());
    }

    #[test]
    fn the_frame_is_right_handed_and_follows_a_slope() {
        let normal = Vec3::new(0.0, 1.0, 0.2).normalize();
        let f = frame(Vec3::ZERO, 1.0, normal);
        let (x, y, z) = (f.x_axis.truncate(), f.y_axis.truncate(), f.z_axis.truncate());
        assert!((x.cross(y) - z).length() < 1e-5, "forward x left = up");
        assert!(x.dot(z).abs() < 1e-5, "forward lies in the slope");
        let want = space::to_render(normal.to_array());
        assert!((z - want).length() < 1e-5);
    }
}
