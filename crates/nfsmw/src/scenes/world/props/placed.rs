//! Props the game puts in the world after the map is read, as a placed prop behaves: asleep until something reaches it,
//! then a rigid body with the dynamics of its `smackable` collection. A roadblock's sawhorses are these. They share the
//! solver and the grid with map props and live as long as the handle that owns them.

use std::rc::Weak;
use std::sync::Arc;

use blackbox_render::Instance;
use blackbox_scenery::ObjectSpawner;
use glam::{Mat3, Mat4, Quat, Vec3};
use nfsmw_data::world::PropDynamics;

use super::{PropWorld, body::PropBody};
use crate::scenes::world::space;

/// One prop to put in the world.
pub struct PlacedSpawn {
    pub bounds: Arc<blackbox_collision::BoundsSet>,
    pub spec: Arc<PropDynamics>,
    /// The model's frame in render space (x forward, y left, z up), where its origin stands.
    pub model: Mat4,
    /// The model's meshes in its own frame (as a rig draws them at the origin).
    pub instances: Vec<Instance>,
    /// The prop lives while this handle's owner does.
    pub owner: Weak<()>,
}

/// The spawner record a map would hold for a model standing at `model` (render space): physics-space position and the
/// quaternion of the model's frame in the body axes (`pose_matrix` in `body.rs` is its inverse).
pub fn spawner_for(model: &Mat4) -> ObjectSpawner {
    let (_, rotation, translation) = model.to_scale_rotation_translation();
    let basis = Mat3::from_quat(rotation);
    let physics = |v: Vec3| Vec3::from_array(space::to_physics(v));
    // Render columns (forward, left, up) are the body's (z, -x, y) axes.
    let body = Mat3::from_cols(-physics(basis.y_axis), physics(basis.z_axis), physics(basis.x_axis));
    let q = Quat::from_mat3(&body).normalize();
    ObjectSpawner {
        orientation: q.to_array(),
        position: physics(translation).to_array(),
        model: 0,
        collision: 0,
        attributes: 0,
        override_index: 0,
        unique_id: 0,
        exclude_flags: 0,
    }
}

impl PropWorld {
    /// Puts a prop in the world. Returns its id, or `None` when its bounds or placement are unusable.
    pub fn spawn_placed(&mut self, spawn: PlacedSpawn) -> Option<u32> {
        let spawner = spawner_for(&spawn.model);
        let body = PropBody::new(&spawner, &spawn.bounds, &spawn.spec, spawn.model)?;
        self.insert_fragment_with(body, spawn.spec, spawn.instances, spawn.owner, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spawner_frame_round_trips_through_the_body_pose() {
        let model = Mat4::from_rotation_translation(Quat::from_rotation_z(0.7), Vec3::new(10.0, -4.0, 2.0));
        let spawner = spawner_for(&model);
        let pose = (Vec3::from_array(spawner.position), Quat::from_array(spawner.orientation));
        let back = super::super::body::pose_matrix(pose);
        for (a, b) in back.to_cols_array().into_iter().zip(model.to_cols_array()) {
            assert!((a - b).abs() < 1e-4, "{back:?} vs {model:?}");
        }
    }
}
