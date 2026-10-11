//! Ground support from enabled box corners and installed point clouds.
//! Spec: `docs/specs/ground-contact-response.md`.

use blackbox_collision::{Shape, bounds_flags};
use blackbox_vehicle::{
    Ground,
    rigid_body::{BOX_CORNERS, BodyState},
};
use glam::Vec3;

use super::body::PropBody;

pub(super) fn step(prop: &mut PropBody, ground: &dyn Ground, dt: f32) {
    if prop.attached || prop.rigid.state != BodyState::Awake {
        return;
    }
    prop.rigid.begin_frame(dt);
    prop.rigid.apply_drag();
    let mut contacts: Vec<_> =
        points(prop).filter_map(|point| prop.rigid.ground_contact(point, ground, dt)).take(16).collect();
    prop.rigid.resolve_ground(&mut contacts);
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
            BOX_CORNERS.map(|corner| b.centre + b.axes * (b.half * corner))
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
