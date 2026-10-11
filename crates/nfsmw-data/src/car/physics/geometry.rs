//! Vehicle contact shapes and named joint anchors from the original bounds hierarchy.

use std::sync::Arc;

use blackbox_collision::{BodyGeometry, Bounds, BoundsSet, GeometryOptions, Shape, bounds_flags};
use blackbox_vehicle::rigid_body::{AngularLimit, LimitMode};
use glam::{Quat, Vec3};

use super::Fields;

#[derive(Debug, Clone)]
pub struct CarGeometry {
    /// The complete hierarchy, including cargo, joints and damage parts.
    pub bounds: Arc<BoundsSet>,
    /// Only the geometry belonging to the intact rigid body.
    pub body: BodyGeometry,
}

impl CarGeometry {
    pub fn read(bounds: Arc<BoundsSet>, record: Option<Fields<'_>>) -> Option<Self> {
        let options = GeometryOptions {
            fallback_half: Vec3::from_array(bounds.nodes.first()?.half_dimensions),
            fallback_shape: match record.and_then(|r| r.0.get_str("DEFAULT_COL_BOX")) {
                Some("SPHERE") => Shape::Sphere,
                _ => Shape::Box,
            },
            box_padding: record.map_or(Vec3::ZERO, |r| r.vec3("COLLISION_BOX_PAD")),
            surface: record.and_then(|r| r.0.get_string_key("BASE_MATERIAL")).map_or(0, |k| k.hash32),
        };
        Some(Self { body: BodyGeometry::from_bounds(&bounds, 0, options)?, bounds })
    }

    /// Stored body-relative position, not a difference between quantized absolute pivots.
    pub fn joint(&self, name: u32, male: bool) -> Option<Vec3> {
        Some(Vec3::from_array(self.joint_node(name, male)?.position))
    }

    fn joint_node(&self, name: u32, male: bool) -> Option<&Bounds> {
        let flag = match male {
            true => bounds_flags::JOINT_MALE,
            false => bounds_flags::JOINT_FEMALE,
        };
        self.bounds.children(self.bounds.root()).iter().find(|n| n.name_hash == name && n.flags & flag != 0)
    }

    /// Match the female constraint children to male posts by serialized names.
    pub fn joint_limits(&self, other: &Self, name: u32) -> Vec<AngularLimit> {
        let (Some(female), Some(male)) = (self.joint_node(name, false), other.joint_node(name, true)) else {
            return Vec::new();
        };
        let convert = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        self.bounds
            .children(female)
            .iter()
            .filter_map(|constraint| {
                let mode = match constraint.flags {
                    n if n & bounds_flags::CONSTRAINT_CONICAL != 0 => LimitMode::Conical,
                    n if n & bounds_flags::CONSTRAINT_PRISMATIC != 0 => LimitMode::Prismatic,
                    _ => return None,
                };
                let post = other
                    .bounds
                    .children(male)
                    .iter()
                    .find(|p| p.name_hash == constraint.name_hash && p.flags & bounds_flags::MALE_POST != 0)?;
                let frame_a = Quat::from_array(female.orientation) * Quat::from_array(constraint.orientation) * convert;
                let frame_b = Quat::from_array(male.orientation) * Quat::from_array(post.orientation) * convert;
                let positive = constraint.half_dimensions[0].atan2(2.0 * constraint.half_dimensions[1]);
                Some(AngularLimit {
                    frame_a: frame_a.normalize(),
                    vector_b: frame_b * Vec3::Z * (2.0 * post.half_dimensions[1]),
                    positive,
                    negative: positive,
                    mode,
                })
            })
            .collect()
    }
}
