//! Whether a bounds set can be a vehicle body: the checks a host makes before it builds a rigid body from one.

use crate::bytes::malformed;
use crate::math::Vec3;
use crate::{BoundsSet, Result, Shape, bounds_flags as flags};

/// The sizes a vehicle body may have, as half dimensions of the root box (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyLimits {
    pub min_half: Vec3,
    pub max_half: Vec3,
}

impl Default for BodyLimits {
    /// From a small kart to a long articulated lorry.
    fn default() -> Self {
        Self { min_half: [0.25, 0.15, 0.5], max_half: [3.0, 3.5, 12.0] }
    }
}

impl BoundsSet {
    /// Checks that this set is a vehicle body within `limits`:
    ///
    /// - the root is a box with finite half dimensions and pivot inside `limits`;
    /// - the nodes form a tree reachable from the root, each node once, with children inside the array;
    /// - every box and sphere has finite, positive dimensions no bigger than the root's extent plus its pivot offset;
    /// - every mesh-versus-ground node names a point cloud that exists and holds finite points, and no cloud is empty;
    /// - at least one node below the root takes part in collisions (a primitive, or a mesh for the ground).
    pub fn check_vehicle_body(&self, limits: &BodyLimits) -> Result<()> {
        let root = self.root();
        if root.shape() != Shape::Box {
            return Err(malformed("vehicle bounds", "the root bounds is not a box"));
        }
        for a in 0..3 {
            let half = root.half_dimensions[a];
            if !half.is_finite() || half < limits.min_half[a] || half > limits.max_half[a] {
                return Err(malformed(
                    "vehicle bounds",
                    format!(
                        "root half dimension {a} is {half} m, outside {}..{}",
                        limits.min_half[a], limits.max_half[a]
                    ),
                ));
            }
        }
        if root.pivot.iter().any(|v| !v.is_finite() || v.abs() > limits.max_half[2] * 2.0) {
            return Err(malformed("vehicle bounds", format!("root pivot {:?} is out of range", root.pivot)));
        }
        let count = self.nodes.len();
        for (i, node) in self.nodes.iter().enumerate() {
            let first = usize::try_from(node.first_child).unwrap_or(usize::MAX);
            if node.child_count > 0 && first.saturating_add(usize::from(node.child_count)) > count {
                return Err(malformed("vehicle bounds", format!("node {i}: children run past the nodes")));
            }
        }
        if !self.is_tree() {
            return Err(malformed("vehicle bounds", "the nodes do not form a tree"));
        }
        let reach = 2.0 * limits.max_half[2];
        for (i, node) in self.nodes.iter().enumerate() {
            let sized = node.half_dimensions.iter().all(|h| h.is_finite() && *h >= 0.0 && *h <= reach);
            if !sized || !node.pivot.iter().chain(&node.position).all(|v| v.is_finite()) {
                return Err(malformed("vehicle bounds", format!("node {i} has a dimension that is not usable")));
            }
            if node.shape() != Shape::None && node.half_dimensions.iter().any(|h| *h <= 0.0) {
                return Err(malformed("vehicle bounds", format!("node {i} is a shape with no volume")));
            }
            if node.flags & flags::MESH_VS_GROUND != 0 && !self.cloud_of(node).is_some_and(|c| !c.is_empty()) {
                return Err(malformed(
                    "vehicle bounds",
                    format!("node {i} is a mesh for the ground without a point cloud"),
                ));
            }
        }
        for (i, cloud) in self.point_clouds.iter().enumerate() {
            if cloud.is_empty() || cloud.iter().flatten().any(|v| !v.is_finite() || v.abs() > reach) {
                return Err(malformed("vehicle bounds", format!("point cloud {i} is empty or out of range")));
            }
        }
        let collides = self.children(root).iter().any(|n| {
            n.flags & flags::DISABLED == 0
                && n.flags
                    & (flags::PRIM_VS_WORLD | flags::PRIM_VS_OBJECTS | flags::PRIM_VS_GROUND | flags::MESH_VS_GROUND)
                    != 0
        });
        if !collides {
            return Err(malformed("vehicle bounds", "no node below the root takes part in collisions"));
        }
        Ok(())
    }

    fn cloud_of(&self, node: &crate::Bounds) -> Option<&Vec<[f32; 3]>> {
        self.point_clouds.get(usize::from(node.point_cloud?))
    }
}
