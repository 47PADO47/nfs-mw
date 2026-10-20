//! Test fixtures for installed-world checks outside the traffic module: a patrol car on the nearest road.

use blackbox_roads::{RoadNav, SegmentFilter, closest_segment};
use glam::Vec3;

use super::car::{AiCar, Role, Start};
use super::traffic::Cruise;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

/// A parked patrol car of the loaded `copmidsize` model, put on the road nearest to `near` (physics space) facing along
/// it. `None` when the model is not loaded or no road is near.
pub(in crate::scenes::world) fn patrol_at(scene: &WorldScene, near: Vec3) -> Option<AiCar> {
    let world = scene.traffic.as_ref()?;
    let model = &world.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case("copmidsize"))?.model;
    let hit = closest_segment(&world.network, &world.index, near, Vec3::ZERO, 0.0, SegmentFilter::default())?;
    let mut nav = RoadNav::traffic();
    if !nav.init_at_point(&world.network, &world.index, hit.point, Vec3::Z, false) {
        return None;
    }
    let spawn = Spawn { position: space::to_render(hit.point.to_array()), heading: 0.0 };
    let ground = WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
    AiCar::place(
        model,
        Role::Traffic,
        nav,
        spawn,
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &ground,
    )
}
