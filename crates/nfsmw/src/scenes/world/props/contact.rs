//! Original contact materials, attachment and no-car-effect overrides.

use blackbox_vehicle::rigid_body::{
    BodyContact, BodyHit, BodyReaction, ObbContact, RigidBody, Separation, resolve_bodies,
};
use glam::Vec3;
use nfsmw_data::world::PropDynamics;

use super::body::PropBody;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};

pub(super) fn car_contact(
    car: &RigidBody,
    geometry: Option<BodyGeometryFrame<'_>>,
    prop: &PropBody,
) -> Option<(ObbContact, u32)> {
    let shapes = object_shapes(car, geometry);
    object_shapes(&prop.rigid, Some(prop.geometry_frame()))
        .iter()
        .flat_map(|p| shapes.iter().filter_map(move |a| Some((a.contact(p)?, p.surface))))
        .max_by(|a, b| a.0.overlap.total_cmp(&b.0.overlap))
}

pub(super) fn prop_contact(a: &PropBody, b: &PropBody) -> Option<ObbContact> {
    let aa = object_shapes(&a.rigid, Some(a.geometry_frame()));
    let bb = object_shapes(&b.rigid, Some(b.geometry_frame()));
    aa.iter().flat_map(|a| bb.iter().filter_map(move |b| a.contact(b))).max_by(|a, b| a.overlap.total_cmp(&b.overlap))
}

pub(super) fn parameters(body: &RigidBody, normal: Vec3, spec: &PropDynamics, attached: bool) -> BodyHit {
    let mut hit = material(body, normal);
    hit.immobile = spec.immobile || attached;
    if !spec.immobile && attached {
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
