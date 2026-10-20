//! Authored barrier volumes against loose props; `docs/specs/prop-world-barriers.md`.

use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, RayOptions, Shape, WorldBarrier, bounds_flags};
use blackbox_vehicle::rigid_body::{ContactParams, Obb, PlaneContact};
use glam::{Mat3, Vec3};
use nfsmw_data::{car::physics::SurfaceTable, world::PropDynamics};

use super::body::PropBody;
use crate::audio::{CarContact, ContactSource};
use crate::scenes::world::object_geometry::{ObjectGeometry, primitive_at};

pub(super) fn step(
    prop: &mut PropBody,
    spec: &PropDynamics,
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
    scratch: &mut Vec<WorldBarrier>,
    sounds: &mut Vec<CarContact>,
    visuals: &mut Vec<super::super::impact_effects::RawContact>,
) {
    if !prop.moved || spec.no_world_collisions {
        return;
    }
    let radius = prop.geometry.primitives.iter().fold(3.0_f32, |r, p| r.max(p.half.length() + p.centre.length()));
    // Original smackables use mask zero; scenery membership is an independent query gate.
    let options = RayOptions { exclude: 0, skip_groups: true, ..RayOptions::default() };
    collision.barriers_near_into(prop.rigid.position.to_array(), radius, &options, scratch);
    for primitive in &prop.geometry.primitives {
        if !primitive.collides(bounds_flags::PRIM_VS_WORLD) {
            continue;
        }
        let bounds = primitive_at(primitive, (prop.rigid.position, prop.rigid.orientation));
        let previous = primitive_at(primitive, prop.previous);
        let mut shape = ObjectGeometry {
            travel: bounds.centre - previous.centre,
            bounds,
            shape: primitive.shape,
            surface: primitive.surface,
        };
        for barrier in scratch.iter() {
            let forward = Vec3::from_array(barrier.normal);
            let Some(hit) = shape.contact(&barrier_shape(barrier)) else { continue };
            // AddCollisionSphere forces runtime ONESIDED; serialized bounds flags use a different layout.
            if (primitive.shape == Shape::Sphere || barrier.barrier.flags & BARRIER_TWO_SIDED == 0)
                && forward.dot(hit.normal) <= 0.0
            {
                continue;
            }
            let displacement = hit.normal * hit.overlap;
            let point = hit.point + if hit.point_on_first { displacement } else { Vec3::ZERO };
            prop.rigid.position += displacement;
            shape.bounds.centre += displacement;
            let walls = &spec.walls;
            let friction = surfaces.world_friction(barrier.surface_hash);
            let plane = PlaneContact {
                point,
                normal: hit.normal,
                friction_static: walls.friction[0] * friction,
                friction_kinetic: walls.friction[1].min(walls.friction[0]) * friction,
            };
            let elasticity = (prop.rigid.rotation().transpose() * hit.normal) * walls.elasticity;
            let velocity = prop.rigid.point_velocity(point);
            let reaction = prop.rigid.react_plane(
                &plane,
                &ContactParams { restitution: elasticity.length(), inertia_scale: walls.moment_scale },
            );
            if let Some(reaction) = reaction {
                visuals.push(super::super::impact_effects::RawContact::new(
                    super::super::impact_effects::Kind::World,
                    barrier.surface_hash,
                    point,
                    hit.normal,
                    velocity,
                    reaction.impulse / prop.rigid.mass(),
                    prop.rigid.linear_velocity,
                ));
                if spec.sound_profile == 0 {
                    continue;
                }
                sounds.push(CarContact {
                    sound_profile: spec.sound_profile,
                    audio_zone: Some(nfsmw_data::sound::CollisionZone::from_local_normal(
                        prop.rigid.rotation().transpose() * hit.normal,
                    )),
                    detached: false,
                    source: ContactSource::World,
                    identity: 0,
                    surface: barrier.surface_hash,
                    point,
                    normal: hit.normal,
                    velocity,
                    delta_v: reaction.impulse / prop.rigid.mass(),
                    front: false,
                    bottom: false,
                });
            }
        }
    }
}

fn barrier_shape(world: &WorldBarrier) -> ObjectGeometry {
    let barrier = &world.barrier;
    let p0 = Vec3::from_array(barrier.p0);
    let p1 = Vec3::from_array(barrier.p1);
    let forward = Vec3::from_array(world.normal);
    let width = Vec3::new(p1.x - p0.x, 0.0, p1.z - p0.z).length();
    ObjectGeometry {
        bounds: Obb {
            centre: (p0 + p1) * 0.5,
            axes: Mat3::from_cols(Vec3::Y.cross(forward), Vec3::Y, forward),
            half: Vec3::new(width * 0.5, (p1.y - p0.y).abs() * 0.5, 0.0),
        },
        shape: Shape::Box,
        travel: Vec3::ZERO,
        surface: world.surface_hash,
    }
}

#[cfg(test)]
mod installed_tests;
#[cfg(test)]
mod tests;
