//! Runtime rigid body for an original scenery placement.

use blackbox_collision::{BodyGeometry, BodyPrimitive, BoundsSet, GeometryOptions};
use blackbox_scenery::ObjectSpawner;
use blackbox_vehicle::rigid_body::{BodyState, Obb, RigidBody};
use glam::{Mat3, Mat4, Quat, Vec3};
use nfsmw_data::world::PropDynamics;

use crate::scenes::world::{
    object_geometry::{BodyGeometryFrame, primitive_at},
    space,
};

pub(super) struct PropBody {
    pub rigid: RigidBody,
    pub geometry: BodyGeometry,
    pub attached: bool,
    pub moved: bool,
    pub previous: (Vec3, Quat),
    /// Original visual matrix, including any model scale/quantization.
    visual: Mat4,
    inverse_initial: Mat4,
}

impl PropBody {
    /// A vehicle fragment starts in the named child frame, with its parent mesh pose preserved.
    pub fn fragment(
        bounds: &BoundsSet,
        root: usize,
        spec: &PropDynamics,
        pose: (Vec3, Quat),
        visual: Mat4,
    ) -> Option<Self> {
        let half = Vec3::from_array(bounds.nodes.get(root)?.half_dimensions).max(Vec3::splat(0.025));
        let geometry = BodyGeometry::from_bounds(
            bounds,
            root,
            GeometryOptions {
                fallback_half: half,
                fallback_shape: spec.fallback_shape,
                box_padding: spec.body.collision_box_pad,
                surface: spec.base_surface,
            },
        )?;
        let mut rigid = RigidBody::new(spec.mass, half, spec.tensor_scale, spec.body);
        rigid.place(pose.0, pose.1);
        rigid.set_cog(geometry.centre_of_gravity());
        let pose = (rigid.position, rigid.orientation);
        Some(Self {
            rigid,
            geometry,
            attached: spec.detach_impulse != 0.0,
            moved: true,
            previous: pose,
            visual,
            inverse_initial: pose_matrix(pose).inverse(),
        })
    }

    pub fn new(spawn: &ObjectSpawner, bounds: &BoundsSet, spec: &PropDynamics, visual: Mat4) -> Option<Self> {
        let root = bounds.nodes.first()?;
        let half = Vec3::from_array(root.half_dimensions).max(Vec3::splat(0.025));
        let options = GeometryOptions {
            fallback_half: half,
            fallback_shape: spec.fallback_shape,
            box_padding: spec.body.collision_box_pad,
            surface: spec.base_surface,
        };
        let geometry = BodyGeometry::from_bounds(bounds, 0, options)?;
        let mut rigid = RigidBody::new(spec.mass, half, spec.tensor_scale, spec.body);
        let orientation = Quat::from_array(spawn.orientation);
        let position = Vec3::from_array(spawn.position);
        if !position.is_finite() || !orientation.is_finite() || orientation.length_squared() < 1e-6 {
            return None;
        }
        let orientation = orientation.normalize();
        rigid.place(position + orientation * Vec3::from_array(root.pivot), orientation);
        // The original uses volume-weighted object primitives unless a centre marker is present.
        rigid.set_cog(geometry.centre_of_gravity());
        rigid.state = BodyState::Asleep;
        let previous = (rigid.position, rigid.orientation);
        let inverse_initial = pose_matrix(previous).inverse();
        Some(Self {
            rigid,
            geometry,
            attached: spec.detach_impulse != 0.0,
            moved: false,
            previous,
            visual,
            inverse_initial,
        })
    }

    pub fn obb(&self, primitive: &BodyPrimitive) -> Obb {
        primitive_at(primitive, (self.rigid.position, self.rigid.orientation))
    }

    pub fn geometry_frame(&self) -> BodyGeometryFrame<'_> {
        BodyGeometryFrame { geometry: &self.geometry, previous: self.previous }
    }

    pub fn transform(&self, alpha: f32) -> Mat4 {
        let pose =
            (self.previous.0.lerp(self.rigid.position, alpha), self.previous.1.slerp(self.rigid.orientation, alpha));
        pose_matrix(pose) * self.inverse_initial * self.visual
    }
}

fn pose_matrix(pose: (Vec3, Quat)) -> Mat4 {
    let r = Mat3::from_quat(pose.1);
    let convert = |v: Vec3| space::to_render(v.to_array());
    let rotation = Mat3::from_cols(convert(r.z_axis), convert(-r.x_axis), convert(r.y_axis));
    Mat4::from_rotation_translation(Quat::from_mat3(&rotation), convert(pose.0))
}
