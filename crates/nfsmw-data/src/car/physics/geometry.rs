//! Vehicle contact shapes and named joint anchors from the original bounds hierarchy.

use std::sync::Arc;

use blackbox_collision::{BodyGeometry, BoundsSet, GeometryOptions, Shape, bounds_flags};
use glam::Vec3;

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
        let flag = match male {
            true => bounds_flags::JOINT_MALE,
            false => bounds_flags::JOINT_FEMALE,
        };
        let node =
            self.bounds.children(self.bounds.root()).iter().find(|n| n.name_hash == name && n.flags & flag != 0)?;
        Some(Vec3::from_array(node.position))
    }
}
