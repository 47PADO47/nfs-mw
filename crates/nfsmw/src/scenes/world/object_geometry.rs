//! Collision shapes in the current and previous fixed-tick body frames.

use blackbox_collision::{BodyGeometry, BodyPrimitive, Shape, bounds_flags};
use blackbox_vehicle::rigid_body::{
    Obb, ObbContact, RigidBody, Sphere, box_contact, sphere_box_contact, sphere_contact,
};
use glam::{Mat3, Quat, Vec3};

pub type BodyPose = (Vec3, Quat);

#[derive(Clone, Copy)]
pub struct BodyGeometryFrame<'a> {
    pub geometry: &'a BodyGeometry,
    pub previous: BodyPose,
}

pub struct ObjectGeometry {
    pub bounds: Obb,
    pub shape: Shape,
    pub travel: Vec3,
    pub surface: u32,
}

pub fn primitive_at(p: &BodyPrimitive, pose: BodyPose) -> Obb {
    let rotation = Mat3::from_quat(pose.1);
    Obb { centre: pose.0 + rotation * p.centre, axes: rotation * Mat3::from_quat(p.rotation), half: p.half }
}

pub fn object_shapes(body: &RigidBody, frame: Option<BodyGeometryFrame<'_>>) -> Vec<ObjectGeometry> {
    let Some(frame) = frame else {
        return vec![ObjectGeometry {
            bounds: Obb { centre: body.position, axes: body.rotation(), half: body.dimension() },
            shape: Shape::Box,
            travel: Vec3::ZERO,
            surface: 0,
        }];
    };
    frame
        .geometry
        .primitives
        .iter()
        .filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS))
        .map(|p| {
            let bounds = primitive_at(p, (body.position, body.orientation));
            let previous = frame.previous.0 + Mat3::from_quat(frame.previous.1) * p.centre;
            ObjectGeometry { travel: bounds.centre - previous, bounds, shape: p.shape, surface: p.surface }
        })
        .collect()
}

#[cfg(test)]
mod tests;

impl ObjectGeometry {
    fn sphere(&self) -> Sphere {
        Sphere { centre: self.bounds.centre, radius: self.bounds.half.x }
    }

    pub fn contact(&self, other: &Self) -> Option<ObbContact> {
        match (self.shape, other.shape) {
            (Shape::Box, Shape::Box) => box_contact(&self.bounds, &other.bounds, self.travel, other.travel),
            (Shape::Sphere, Shape::Sphere) => sphere_contact(self.sphere(), other.sphere()),
            (Shape::Sphere, Shape::Box) => sphere_box_contact(self.sphere(), &other.bounds, self.travel - other.travel),
            (Shape::Box, Shape::Sphere) => {
                let hit = sphere_box_contact(other.sphere(), &self.bounds, other.travel - self.travel)?;
                Some(ObbContact { normal: -hit.normal, point_on_first: !hit.point_on_first, ..hit })
            }
            _ => None,
        }
    }
}
