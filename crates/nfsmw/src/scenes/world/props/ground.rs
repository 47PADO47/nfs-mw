//! Ground support from enabled box corners and installed point clouds.
//! The existing height-query tolerances and plane response still need original comparison.

use blackbox_collision::{Shape, bounds_flags};
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
    let mut contacts: Vec<_> = points(prop)
        .filter_map(|point| {
            let hit = ground.hit(point + Vec3::Y * lift, Vec3::NEG_Y, lift + 0.02)?;
            let normal = hit.normal * hit.normal.y.signum();
            if normal.y < 0.05 {
                return None;
            }
            let depth = (lift - hit.distance) * normal.y;
            if depth < -0.02 || !depth.is_finite() {
                return None;
            }
            Some((point, normal, depth))
        })
        .take(16)
        .collect();
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

fn points(prop: &PropBody) -> impl Iterator<Item = Vec3> + '_ {
    let boxes = prop
        .geometry
        .primitives
        .iter()
        .filter(|p| p.shape == Shape::Box && p.collides(bounds_flags::PRIM_VS_GROUND))
        .flat_map(|p| {
            let b = prop.obb(p);
            std::array::from_fn::<_, 8, _>(|i| {
                let sign = |mask| match i & mask == 0 {
                    true => -1.0,
                    false => 1.0,
                };
                b.centre + b.axes * (b.half * Vec3::new(sign(4), sign(2), sign(1)))
            })
        });
    let clouds = prop
        .geometry
        .ground_meshes
        .iter()
        .filter(|m| m.flags & bounds_flags::DISABLED == 0)
        .flat_map(|m| &m.points)
        .map(|p| prop.rigid.position + prop.rigid.rotation() * p);
    boxes.chain(clouds)
}
