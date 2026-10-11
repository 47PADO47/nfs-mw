//! Existing plane response applied to loose prop primitives; point-cloud parity is pending.

use blackbox_collision::bounds_flags;
use blackbox_vehicle::{
    Ground,
    rigid_body::{BodyState, ContactParams, PlaneContact},
};
use glam::Vec3;

use super::body::PropBody;

pub(super) fn step(prop: &mut PropBody, ground: &dyn Ground, dt: f32) {
    if prop.attached || prop.rigid.state != BodyState::Awake {
        return;
    }
    let previous = prop.rigid.position;
    prop.rigid.begin_frame(dt);
    prop.rigid.apply_drag();
    let travel = (prop.rigid.position - previous).length();
    let lift = 0.3 + travel;
    let mut contacts = Vec::new();
    for p in prop.primitives.iter().filter(|p| p.flags & bounds_flags::PRIM_VS_GROUND != 0) {
        let b = prop.obb(p);
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let corner = b.centre + b.axes * (b.half * Vec3::new(x, y, z));
                    let Some(hit) = ground.hit(corner + Vec3::Y * lift, Vec3::NEG_Y, lift + 0.02) else { continue };
                    let normal = hit.normal * hit.normal.y.signum();
                    if normal.y < 0.05 {
                        continue;
                    }
                    let depth = (lift - hit.distance) * normal.y;
                    if depth < -0.02 || !depth.is_finite() {
                        continue;
                    }
                    contacts.push((corner, normal, depth));
                }
            }
        }
    }
    contacts.sort_by(|a, b| b.2.total_cmp(&a.2));
    let spec = *prop.rigid.spec();
    for &(point, normal, _) in &contacts {
        let plane = PlaneContact {
            point,
            normal,
            friction_static: spec.ground_friction[0],
            friction_kinetic: spec.ground_friction[1],
        };
        let local = prop.rigid.rotation().transpose() * normal;
        let params = ContactParams {
            restitution: (local * spec.ground_elasticity).length(),
            inertia_scale: spec.ground_moment_scale,
        };
        if prop.rigid.react_plane(&plane, &params).is_some() {
            break;
        }
    }
    if let Some(&(_, normal, depth)) = contacts.first() {
        prop.rigid.position += normal * depth.max(0.0);
    }
    prop.rigid.update_sleep(contacts.len());
}
