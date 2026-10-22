//! The props in motion as a server replicates them (`docs/specs/multiplayer/world-host.md`): every body a car or a blast
//! set moving that is still awake. A prop at rest, asleep where it fell, or hidden while knocked over is not listed; the
//! clients' own copies of the tile hold the resting ones.

use blackbox_vehicle::rigid_body::BodyState;
use glam::{Quat, Vec3};

use super::PropWorld;
use super::body::pose_matrix;
use crate::scenes::world::space;

/// One moving prop after the last tick, render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovingProp {
    /// The prop's id in its world (`PropWorld::add_tile`).
    pub id: u32,
    /// The hash of the scenery name (`blackbox_hash::bstring_hash`), so a client can tell the model.
    pub name_hash: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

impl PropWorld {
    /// Every prop body in motion, in id order, into `out` (cleared first).
    pub fn moving_props(&self, out: &mut Vec<MovingProp>) {
        out.clear();
        for (&id, collider) in &self.colliders {
            let Some(body) = collider.body.as_ref() else { continue };
            if !body.moved || body.rigid.state != BodyState::Awake || self.hidden(id) {
                continue;
            }
            let (_, rotation, position) =
                pose_matrix((body.rigid.position, body.rigid.orientation)).to_scale_rotation_translation();
            out.push(MovingProp {
                id,
                name_hash: blackbox_hash::bstring_hash(&collider.shape.name),
                position,
                rotation,
                linear_velocity: space::to_render(body.rigid.linear_velocity.to_array()),
                angular_velocity: space::to_render(body.rigid.angular_velocity.to_array()),
            });
        }
        out.sort_unstable_by_key(|prop| prop.id);
    }
}
