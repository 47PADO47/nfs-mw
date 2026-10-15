//! Existing physical props that opt into original-camera avoidance.

use glam::{Mat4, Vec3};

use super::PropWorld;
use crate::scenes::world::space;
use crate::viewer::camera::original::Avoidable;

impl PropWorld {
    pub fn camera_bodies(&self, out: &mut Vec<Avoidable>) {
        let mut ids: Vec<_> = self.colliders.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let collider = &self.colliders[&id];
            if !collider.collidable || self.hidden(id) {
                continue;
            }
            let Some(spec) = &collider.shape.dynamics else { continue };
            if !spec.camera_avoidable {
                continue;
            }
            let Some(body) = &collider.body else { continue };
            if body.lifetime.sinking {
                continue;
            }
            let rigid = &body.rigid;
            let rotation = rigid.rotation();
            let convert = |v: Vec3| space::to_render(v.to_array());
            let half = rigid.dimension();
            out.push(Avoidable {
                transform: Mat4::from_cols(
                    convert(rotation.z_axis).extend(0.0),
                    convert(rotation.x_axis).extend(0.0),
                    convert(rotation.y_axis).extend(0.0),
                    convert(rigid.position).extend(1.0),
                ),
                half_extents: Vec3::new(half.z, half.x, half.y),
                velocity: convert(rigid.linear_velocity),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn opt_in_hidden_and_sinking_props_use_the_same_physical_registration() {
        let mut placed = crate::scenes::world::props::tests::placement(1, Vec3::ZERO, 100.0, false, 0.0);
        let shape = Arc::make_mut(placed.prop.as_mut().unwrap());
        Arc::make_mut(shape.dynamics.as_mut().unwrap()).camera_avoidable = true;
        let mut world = PropWorld::default();
        world.add_tile(0, std::slice::from_mut(&mut placed));
        let id = placed.prop_id;
        let mut bodies = Vec::new();
        world.camera_bodies(&mut bodies);
        assert_eq!(bodies.len(), 1);
        let expected = world.colliders[&id].body.as_ref().unwrap().rigid.position;
        assert_eq!(bodies[0].transform.w_axis.truncate(), space::to_render(expected.to_array()));
        world.knock(id);
        bodies.clear();
        world.camera_bodies(&mut bodies);
        assert!(bodies.is_empty());
        world.knocked.clear();
        world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap().lifetime.sinking = true;
        world.camera_bodies(&mut bodies);
        assert!(bodies.is_empty());
    }
}
