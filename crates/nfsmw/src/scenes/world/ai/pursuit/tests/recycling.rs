//! A cop that fell out of play is taken off the road by the cop manager's rule, not by the traffic's unseen time
//! (`docs/specs/ai-pursuit-cops.md` sections 2 and 3.3).

use super::free_roam::{patrol, renderer};
use super::*;
use crate::scenes::world::props::PropWorld;
use crate::viewer::camera::Lens;

/// A camera at `z` metres along the road (physics space), looking along the road when `along` is true and across it
/// otherwise.
fn camera_at(z: f32, along: bool) -> Lens {
    let eye = space::to_render([0.0, 0.0, z]);
    let ahead = space::to_render(match along {
        true => [0.0, 0.0, z + 10.0],
        false => [10.0, 0.0, z],
    });
    Lens::look_at(eye, ahead, 70.0, 0.1, 1.0)
}

fn player_at(z: f32, speed: f32) -> Focus {
    let body = Body {
        position: Vec3::new(0.0, 0.0, z),
        velocity: Vec3::new(0.0, 0.0, speed),
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    Focus { position: [0.0, z], heading: 0.0, speed, body: Some(body) }
}

/// Runs the world and the recycling for `seconds`, the camera and the player standing still.
fn run(world: &mut TrafficWorld, focus: Focus, camera: &Lens, seconds: f32) {
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let mut props = PropWorld::default();
    for _ in 0..(seconds * 60.0) as usize {
        world.step(focus, &collision, &mut props, &surfaces);
        world.recycle(1.0 / 60.0, focus, camera, camera, &props);
    }
}

/// A world with one police car at `z` along the road. It stands still (a roadblock car holds its place), so its distance
/// from the camera stays what the test says.
fn world_with_a_stationary_cop_at(z: f32) -> TrafficWorld {
    let mut world = world_with_a_driving_patrol_at(z);
    world.cars[0].set_roadblock();
    world
}

fn world_with_a_driving_patrol_at(z: f32) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, z));
    world
}

#[test]
fn a_patrol_beyond_the_middle_of_the_window_goes_after_ten_seconds() {
    let mut world = world_with_a_stationary_cop_at(150.0);
    let (focus, camera) = (player_at(-150.0, 0.0), camera_at(-150.0, true));
    run(&mut world, focus, &camera, 9.0);
    assert_eq!(world.cars.len(), 1, "still there at 9 s");
    run(&mut world, focus, &camera, 2.0);
    assert!(world.cars.is_empty(), "300 m from the camera is available once it is 10 s old");
}

#[test]
fn a_patrol_inside_the_window_and_ahead_of_the_camera_stays() {
    let mut world = world_with_a_stationary_cop_at(60.0);
    let (focus, camera) = (player_at(-150.0, 0.0), camera_at(-150.0, true));
    run(&mut world, focus, &camera, 30.0);
    assert_eq!(world.cars.len(), 1, "210 m straight ahead is in play");
}

#[test]
fn a_patrol_beside_the_camera_between_the_minimum_and_the_middle_goes() {
    let mut world = world_with_a_stationary_cop_at(100.0);
    let (focus, camera) = (player_at(-100.0, 0.0), camera_at(-100.0, false));
    run(&mut world, focus, &camera, 11.0);
    assert!(world.cars.is_empty(), "200 m off to the side is outside the forward cone");
}

#[test]
fn a_cop_in_a_chase_stays_while_it_is_about_to_meet_the_target() {
    let mut world = world_with_a_driving_patrol_at(100.0);
    // The player stands 150 m from the patrol, which sees it and joins, while the camera is 200 m off to the side.
    world.player_position = Some(Vec3::new(0.0, 0.0, -50.0));
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars[0].is_cop());
    let (focus, camera) = (player_at(-50.0, 0.0), camera_at(-100.0, false));
    run(&mut world, focus, &camera, 30.0);
    assert_eq!(world.cars.len(), 1, "the cop is inside the 375 m of the point ahead of the player");
}

#[test]
fn a_wreck_in_play_stays_in_view_and_goes_once_it_may_respawn_out_of_play() {
    use crate::scenes::world::ai::collide::DriverClass;
    let mut world = world_with_a_stationary_cop_at(60.0);
    let hit = crate::scenes::world::events::DamageHit {
        arm: glam::Vec3::ZERO,
        normal: glam::Vec3::X,
        half: glam::Vec3::ONE,
        closing_speed: 50.0,
        force: 1_000_000.0,
        shock_force: 500_000.0,
        before_velocity: glam::Vec3::ZERO,
        other: crate::scenes::world::events::OtherBody { vehicle: true, player: true, ..Default::default() },
    };
    world.cars[0].on_hit(1_000_000.0, true, DriverClass::Human, 50.0, hit, 0.0);
    assert!(world.cars[0].is_wrecked());
    let (focus, camera) = (player_at(-150.0, 0.0), camera_at(-150.0, true));
    // The wreck coasts to a stop (the native retirement): a few seconds, while it is still inside the window.
    run(&mut world, focus, &camera, 3.0);
    assert_eq!(world.cars.len(), 1, "a wreck 210 m ahead of the camera is in play");
    let (focus, camera) = (player_at(-150.0, 0.0), camera_at(-150.0, false));
    run(&mut world, focus, &camera, 12.0);
    assert!(world.cars.is_empty(), "turn away and it is out of the cone");
}
