//! Detached vehicle meshes share the resident prop solver without belonging to a streamed tile.

use std::{rc::Weak, sync::Arc};

use blackbox_render::Instance;
use blackbox_scene::Aabb;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{PropDynamics, PropKind, PropShape};

use super::{Collider, PropWorld, body::PropBody};

pub(super) struct FragmentVisual {
    instances: Vec<Instance>,
    owner: Weak<()>,
    scenery: bool,
}

impl PropWorld {
    #[cfg(test)]
    pub fn fragment_count(&self) -> usize {
        self.fragments.values().filter(|f| f.owner.strong_count() > 0).count()
    }

    pub(super) fn insert_fragment(
        &mut self,
        body: PropBody,
        spec: Arc<PropDynamics>,
        instances: Vec<Instance>,
        owner: Weak<()>,
    ) -> Option<u32> {
        self.insert_fragment_with(body, spec, instances, owner, false)
    }

    pub(super) fn insert_fragment_with(
        &mut self,
        mut body: PropBody,
        spec: Arc<PropDynamics>,
        instances: Vec<Instance>,
        owner: Weak<()>,
        scenery: bool,
    ) -> Option<u32> {
        if !scenery && self.fragments.values().filter(|f| !f.scenery && f.owner.strong_count() > 0).count() >= 121 {
            return None;
        }
        body.lifetime.activate(&spec);
        self.next_id += 1;
        let id = self.next_id | 0xC000_0000;
        let transform = body.transform(1.0);
        let shape = Arc::new(PropShape {
            name: format!("vehicle fragment {id}"),
            boxes: Vec::new(),
            kind: PropKind::Movable { mass: spec.mass },
            bounds: None,
            dynamics: Some(spec),
        });
        let mut collider = Collider {
            collidable: true,
            shape,
            transform,
            inverse_transform: transform.inverse(),
            original_bounds: Aabb::EMPTY,
            body: Some(body),
            cells: Vec::new(),
        };
        collider.original_bounds = collider.physics_bounds();
        self.colliders.insert(id, collider);
        super::lifetime::track_lifetime(&mut self.live_lifetimes, id);
        self.fragments.insert(id, FragmentVisual { instances, owner, scenery });
        self.regrid(id);
        Some(id)
    }

    pub fn fragment_instances(&self, out: &mut Vec<Instance>) {
        for (&id, fragment) in &self.fragments {
            if fragment.owner.strong_count() == 0 {
                continue;
            }
            let Some((world, _)) = self.pose(id) else { continue };
            if self.model_instances(id, world, out) {
                continue;
            }
            out.extend(fragment.instances.iter().map(|i| Instance::new(i.mesh, world * i.transform)));
        }
    }

    /// Bounding spheres (render space: centre and radius, metres) of the detached parts that `owner` released and
    /// that still exist. A part lives while its owner does (`owner` is the weak handle the car holds).
    pub fn fragment_spheres(&self, owner: &Weak<()>) -> Vec<(Vec3, f32)> {
        self.fragments
            .iter()
            .filter(|(_, fragment)| fragment.owner.strong_count() > 0 && fragment.owner.ptr_eq(owner))
            .filter_map(|(&id, _)| self.pose(id))
            .filter(|(_, bounds)| !bounds.is_empty())
            .map(|(_, bounds)| (bounds.center(), (bounds.max - bounds.min).length() * 0.5))
            .collect()
    }

    pub(super) fn remove_orphaned_fragments(&mut self) {
        let dead: Vec<_> =
            self.fragments.iter().filter(|(_, f)| f.owner.strong_count() == 0).map(|(&id, _)| id).collect();
        for id in dead {
            self.remove_fragment(id);
        }
    }

    pub(super) fn remove_fragment(&mut self, id: u32) {
        self.scenery.models.remove(&id);
        self.fragments.remove(&id);
        if let Some(collider) = self.colliders.remove(&id) {
            self.remove_cells(id, &collider.cells);
        }
    }
}

/// Everything needed at the successful spawn boundary; the caller then hides the parent part.
pub struct FragmentSpawn {
    pub bounds: Arc<blackbox_collision::BoundsSet>,
    pub root: usize,
    pub spec: Arc<PropDynamics>,
    pub pose: (glam::Vec3, glam::Quat),
    pub model: Mat4,
    pub velocity: (glam::Vec3, glam::Vec3),
    pub instances: Vec<Instance>,
    pub owner: Weak<()>,
}

impl PropWorld {
    #[cfg(test)]
    pub fn fragment_bodies(&self) -> Vec<(u32, &blackbox_vehicle::rigid_body::RigidBody, Mat4)> {
        self.fragments
            .keys()
            .filter_map(|&id| {
                let body = self.colliders.get(&id)?.body.as_ref()?;
                Some((id, &body.rigid, body.transform(1.0)))
            })
            .collect()
    }

    pub fn spawn_fragment(&mut self, spawn: FragmentSpawn) -> Option<u32> {
        let mut body = PropBody::fragment(&spawn.bounds, spawn.root, &spawn.spec, spawn.pose, spawn.model)?;
        body.rigid.linear_velocity = spawn.velocity.0;
        body.rigid.angular_velocity = spawn.velocity.1;
        self.insert_fragment(body, spawn.spec, spawn.instances, spawn.owner)
    }
}
