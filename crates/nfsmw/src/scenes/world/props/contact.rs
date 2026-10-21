//! Original contact materials, attachment and no-car-effect overrides.

use blackbox_collision::{Shape, bounds_flags};
use blackbox_vehicle::rigid_body::{
    BodyContact, BodyHit, BodyReaction, ObbContact, RigidBody, Separation, resolve_bodies,
};
use glam::Vec3;
use nfsmw_data::world::PropDynamics;

use crate::scenes::world::object_geometry::{BodyGeometryFrame, ObjectGeometry, primitive_at};

pub(super) fn shape_count(frame: Option<BodyGeometryFrame<'_>>) -> usize {
    frame.map_or(1, |f| f.geometry.primitives.iter().filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS)).count())
}

/// Prepare only the primitive being visited, retaining its fixed-tick previous centre.
pub(super) fn shape(body: &RigidBody, frame: Option<BodyGeometryFrame<'_>>, index: usize) -> Option<ObjectGeometry> {
    let Some(frame) = frame else {
        return (index == 0).then(|| ObjectGeometry {
            bounds: blackbox_vehicle::rigid_body::Obb {
                centre: body.position,
                axes: body.rotation(),
                half: body.dimension(),
            },
            shape: Shape::Box,
            travel: Vec3::ZERO,
            surface: 0,
        });
    };
    let primitive =
        frame.geometry.primitives.iter().filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS)).nth(index)?;
    let bounds = primitive_at(primitive, (body.position, body.orientation));
    let previous = primitive_at(primitive, frame.previous).centre;
    Some(ObjectGeometry {
        travel: bounds.centre - previous,
        bounds,
        shape: primitive.shape,
        surface: primitive.surface,
    })
}

pub(super) fn parameters(
    body: &RigidBody,
    normal: Vec3,
    spec: &PropDynamics,
    attached: bool,
    partner_immobile: bool,
) -> BodyHit {
    let mut hit = material(body, normal);
    hit.immobile = spec.immobile || attached;
    if !partner_immobile && !spec.immobile && attached {
        hit.breaking_impulse = spec.detach_impulse.max(0.0);
    }
    hit
}

pub(super) fn material(body: &RigidBody, normal: Vec3) -> BodyHit {
    let m = body.spec().objects;
    BodyHit {
        restitution: m.restitution(body.rotation().transpose() * normal),
        inertia_scale: m.inertia_scale,
        mass_scale: m.mass_scale,
        ..BodyHit::default()
    }
}

/// Returns impulse and each body's post-contact immobility. Contact flags do not freeze a moving car.
pub(super) fn resolve(
    a: &mut RigidBody,
    b: &mut RigidBody,
    c: ObbContact,
    ha: BodyHit,
    hb: BodyHit,
    separation: Separation,
) -> Option<BodyReaction> {
    let contact = BodyContact::from_geometry(c, a.spec().objects.friction_with(&b.spec().objects));
    resolve_bodies(a, b, &contact, &ha, &hb, separation)
}
