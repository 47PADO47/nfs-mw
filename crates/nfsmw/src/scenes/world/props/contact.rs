//! Original contact materials, attachment and no-car-effect overrides.

use blackbox_vehicle::rigid_body::{BodyHit, ObbContact, PlaneContact, RigidBody, react_moments};
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
) -> Option<(f32, bool, bool)> {
    if c.overlap <= 0.0 || (ha.immobile && hb.immobile) {
        return None;
    }
    let share = match (ha.immobile, hb.immobile) {
        (true, false) => 0.0,
        (false, true) => 1.0,
        _ => b.mass() / (a.mass() + b.mass()),
    };
    a.position += c.normal * c.overlap * share;
    b.position -= c.normal * c.overlap * (1.0 - share);
    let [friction_static, friction_kinetic] = a.spec().objects.friction_with(&b.spec().objects);
    let mut ma = a.contact_moment(&ha);
    let mut mb = b.contact_moment(&hb);
    let plane = PlaneContact { point: c.point, normal: c.normal, friction_static, friction_kinetic };
    let result = react_moments(&mut ma, &mut mb, &plane, 32)?;
    a.apply_contact_moment(&ma);
    b.apply_contact_moment(&mb);
    Some((result.impulse, ma.immobile, mb.immobile))
}
