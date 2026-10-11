//! Select a body's shapes from one bounds node, without flattening its fragments.
//! Measured construction rules: `docs/specs/world-sequencers.md`.

use glam::{Mat4, Quat, Vec3};

use crate::{Bounds, BoundsSet, Shape, bounds_flags as flags};

const PRIMITIVE: u16 = flags::PRIM_VS_WORLD | flags::PRIM_VS_OBJECTS | flags::PRIM_VS_GROUND;

/// Body attributes supplied by the host, independently of the serialized bounds.
#[derive(Debug, Clone, Copy)]
pub struct GeometryOptions {
    pub fallback_half: Vec3,
    pub fallback_shape: Shape,
    pub box_padding: Vec3,
    pub surface: u32,
}

#[derive(Debug, Clone)]
pub struct BodyPrimitive {
    pub name: u32,
    pub centre: Vec3,
    pub rotation: Quat,
    pub half: Vec3,
    pub shape: Shape,
    /// Original bounds flags, including disabled and the three collision channels.
    pub flags: u16,
    pub surface: u32,
}

impl BodyPrimitive {
    pub fn collides(&self, channel: u16) -> bool {
        self.flags & flags::DISABLED == 0 && self.flags & channel != 0
    }
}

#[derive(Debug, Clone)]
pub struct BodyMesh {
    pub name: u32,
    pub points: Vec<Vec3>,
    pub surface: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, Default)]
pub struct BodyGeometry {
    pub primitives: Vec<BodyPrimitive>,
    pub ground_meshes: Vec<BodyMesh>,
    pub used_fallback: bool,
}

impl BodyGeometry {
    /// The root is the body frame. Internal children and all deeper descendants
    /// belong to separate bodies; disabled flags remain available to the host.
    pub fn from_bounds(set: &BoundsSet, root: usize, options: GeometryOptions) -> Option<Self> {
        let root = set.nodes.get(root)?;
        let mut geometry = Self::default();
        for child in set.children(root).iter().filter(|n| n.flags & flags::INTERNAL == 0) {
            geometry.add(set, child, false, options);
        }
        if geometry.is_empty() {
            geometry.add(set, root, true, options);
        }
        if !geometry.is_empty() {
            return Some(geometry);
        }
        let (half, channels) = match options.fallback_shape {
            Shape::Sphere => (Vec3::splat(options.fallback_half.max_element()), PRIMITIVE & !flags::PRIM_VS_GROUND),
            Shape::Box => (options.fallback_half + options.box_padding, PRIMITIVE),
            Shape::None => return None,
        };
        geometry.primitives.push(BodyPrimitive {
            name: 0,
            centre: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            half,
            shape: options.fallback_shape,
            flags: channels,
            surface: options.surface,
        });
        geometry.used_fallback = true;
        Some(geometry)
    }

    fn is_empty(&self) -> bool {
        self.primitives.is_empty() && self.ground_meshes.is_empty()
    }

    fn add(&mut self, set: &BoundsSet, node: &Bounds, root: bool, options: GeometryOptions) {
        let (centre, rotation) = match root {
            true => (Vec3::ZERO, Quat::IDENTITY),
            false => (Vec3::from_array(node.position), quaternion(node.orientation)),
        };
        let surface = match node.surface {
            0 => options.surface,
            other => other,
        };
        if node.flags & PRIMITIVE != 0 && node.shape() != Shape::None {
            let half = match node.shape() {
                Shape::Sphere => Vec3::splat(node.radius),
                _ => Vec3::from_array(node.half_dimensions) + options.box_padding,
            };
            let rotation = match node.shape() {
                Shape::Sphere => Quat::IDENTITY,
                _ => rotation,
            };
            let primitive = BodyPrimitive {
                name: node.name_hash,
                centre,
                rotation,
                half,
                shape: node.shape(),
                flags: node.flags,
                surface,
            };
            match node.shape() {
                Shape::Sphere => self.primitives.insert(0, primitive),
                _ => self.primitives.push(primitive),
            }
        }
        if node.flags & flags::MESH_VS_GROUND == 0 {
            return;
        }
        let Some(cloud) = node.point_cloud.and_then(|i| set.point_clouds.get(usize::from(i))) else { return };
        if cloud.is_empty() {
            return;
        }
        let inverse = match root {
            true => {
                Mat4::from_rotation_translation(quaternion(node.orientation), Vec3::from_array(node.position)).inverse()
            }
            false => Mat4::IDENTITY,
        };
        self.ground_meshes.push(BodyMesh {
            name: node.name_hash,
            points: cloud.iter().map(|p| inverse.transform_point3(Vec3::from_array(*p))).collect(),
            surface,
            flags: node.flags,
        });
    }

    /// The original includes disabled object primitives when distributing mass.
    pub fn centre_of_gravity(&self) -> Vec3 {
        let (mut sum, mut volume) = (Vec3::ZERO, 0.0);
        for p in self.primitives.iter().filter(|p| p.flags & flags::PRIM_VS_OBJECTS != 0) {
            let weight = p.half.x * p.half.y * p.half.z;
            sum += p.centre * weight;
            volume += weight;
        }
        if volume > 0.0 {
            return sum / volume;
        }
        Vec3::ZERO
    }
}

fn quaternion(value: [f32; 4]) -> Quat {
    let q = Quat::from_array(value);
    if q.is_finite() && q.length_squared() > 1e-6 {
        return q.normalize();
    }
    Quat::IDENTITY
}

#[cfg(test)]
mod tests;
