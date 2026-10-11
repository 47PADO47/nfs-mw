//! Runtime rigid body for an original scenery placement.

use blackbox_collision::{BoundsSet, Shape, bounds_flags};
use blackbox_scenery::ObjectSpawner;
use blackbox_vehicle::rigid_body::{BodyState, Obb, RigidBody};
use glam::{Mat3, Mat4, Quat, Vec3};
use nfsmw_data::world::PropDynamics;

use crate::scenes::world::space;

/// A primitive in physics model coordinates. Sphere geometry is retained for later narrow-phase work.
#[derive(Clone)]
pub(super) struct Primitive {
    pub centre: Vec3,
    pub rotation: Quat,
    pub half: Vec3,
    pub flags: u16,
    pub surface: u32,
}

pub(super) struct PropBody {
    pub rigid: RigidBody,
    pub primitives: Vec<Primitive>,
    pub attached: bool,
    pub moved: bool,
    pub previous: (Vec3, Quat),
    /// Original visual matrix, including any model scale/quantization.
    visual: Mat4,
    inverse_initial: Mat4,
}

impl PropBody {
    pub fn new(spawn: &ObjectSpawner, bounds: &BoundsSet, spec: &PropDynamics, visual: Mat4) -> Option<Self> {
        let root = bounds.nodes.first()?;
        let primitives: Vec<_> = bounds
            .nodes
            .iter()
            .filter_map(|n| {
                if n.flags & bounds_flags::DISABLED != 0 || n.shape() == Shape::None {
                    return None;
                }
                let q = Quat::from_array(n.orientation);
                let rotation = match q.is_finite() && q.length_squared() > 1e-6 {
                    true => q.normalize(),
                    false => Quat::IDENTITY,
                };
                Some(Primitive {
                    centre: Vec3::from_array(n.pivot),
                    rotation,
                    half: Vec3::from_array(n.half_dimensions),
                    flags: n.flags,
                    surface: n.surface,
                })
            })
            .collect();
        let mut rigid = RigidBody::new(spec.mass, Vec3::from_array(root.half_dimensions), spec.tensor_scale, spec.body);
        let orientation = Quat::from_array(spawn.orientation);
        let position = Vec3::from_array(spawn.position);
        if !position.is_finite() || !orientation.is_finite() || orientation.length_squared() < 1e-6 {
            return None;
        }
        rigid.place(position, orientation);
        // The original uses volume-weighted object primitives unless a centre marker is present.
        let (mut centre, mut volume) = (Vec3::ZERO, 0.0);
        for p in primitives.iter().filter(|p| p.flags & bounds_flags::PRIM_VS_OBJECTS != 0) {
            let weight = p.half.x * p.half.y * p.half.z;
            centre += p.centre * weight;
            volume += weight;
        }
        if volume > 0.0 {
            rigid.set_cog(centre / volume);
        }
        rigid.state = BodyState::Asleep;
        let previous = (rigid.position, rigid.orientation);
        let inverse_initial = pose_matrix(previous).inverse();
        Some(Self {
            rigid,
            primitives,
            attached: spec.detach_impulse != 0.0,
            moved: false,
            previous,
            visual,
            inverse_initial,
        })
    }

    pub fn obb(&self, primitive: &Primitive) -> Obb {
        Obb {
            centre: self.rigid.position + self.rigid.rotation() * primitive.centre,
            axes: self.rigid.rotation() * Mat3::from_quat(primitive.rotation),
            half: primitive.half,
        }
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
