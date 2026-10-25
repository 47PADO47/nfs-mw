//! The traffic step leaves the same cars whether it integrates them on one thread or on several (`ai/parallel.rs`):
//! the same seed and inputs, stepped in the plain serial order and on four threads, give bit-identical cars and random
//! streams after every tick (`docs/physics-timing.md`; the netcode relies on it).

use super::free_roam::renderer;
use super::*;
use crate::scenes::world::ai::parallel::Parallel;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::props::PropWorld;

/// Ticks each run steps: long enough for every car to think several times and the cops to close in.
const TICKS: u32 = 150;

/// The player in physics space at tick `t`: driving up the road's lane at 12 m/s from z = -40 m.
fn player_at(t: u32) -> Vec3 {
    Vec3::new(2.0, 0.0, -40.0 + 12.0 * t as f32 * STEP)
}

fn player_focus(at: Vec3) -> Focus {
    Focus {
        position: [0.0, 0.0],
        heading: 0.0,
        speed: 12.0,
        body: Some(Body {
            position: at,
            velocity: Vec3::Z * 12.0,
            forward: Vec2::Y,
            half_width: 1.0,
            half_length: 2.0,
        }),
    }
}

/// A traffic car on the lane at `at` (physics space) heading along `forward`, rolling at 10 m/s.
fn traffic(world: &mut TrafficWorld, model: &TrafficModel, at: Vec3, forward: Vec3, ground: &WorldGround<'_>) {
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, at, forward, false));
    let position = space::to_render(at.to_array());
    let heading = space::to_render(forward.to_array());
    let stagger = world.cars.len() as u32;
    let car = AiCar::place(
        model,
        Role::Traffic,
        nav,
        Spawn { position, heading: heading.y.atan2(heading.x) },
        Start { speed: 10.0, stagger, cruise: Cruise::default(), patrol: false },
        ground,
    )
    .unwrap();
    world.cars.push(car);
}

/// Two cops chasing the player and seven traffic cars both ways along the cop road, stepped with `parallel`. The cops
/// come first: a chase spawn keeps its distance from every car already there.
fn world(parallel: Parallel, collision: &CollisionWorld, surfaces: &SurfaceTable) -> TrafficWorld {
    let mut renderer = renderer();
    let mut world = TrafficWorld::new(cop_road());
    world.parallel = parallel;
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.player_position = Some(player_at(0));
    world.start_pursuit(1, vec![wave()], player());
    for at in [Vec3::new(2.0, 0.0, -65.0), Vec3::new(-2.0, 0.0, 60.0)] {
        let spot = CopSpot { at, target: player_at(0), speed: 12.0, player: player(), band: None };
        assert!(world.spawn_cop_at("copx", spot, collision, surfaces), "cop at {at}");
    }
    let ground = WorldGround { collision, surfaces };
    let model = cop_model(&mut renderer, "carx", player()).model;
    for z in [-30.0, -15.0, 0.0, 15.0, 30.0] {
        traffic(&mut world, &model, Vec3::new(2.0, 0.0, z), Vec3::Z, &ground);
    }
    for z in [-20.0, 10.0] {
        traffic(&mut world, &model, Vec3::new(-2.0, 0.0, z), -Vec3::Z, &ground);
    }
    world
}

/// Every car's body as bits, then the next draw of the navigation stream, after each tick.
fn run(parallel: Parallel) -> Vec<(Vec<[u32; 8]>, u32)> {
    run_with_deltas(parallel, |_| STEP)
}

fn run_with_deltas(parallel: Parallel, delta: impl Fn(u32) -> f32) -> Vec<(Vec<[u32; 8]>, u32)> {
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let mut world = world(parallel, &collision, &surfaces);
    (0..TICKS)
        .map(|t| {
            let at = player_at(t);
            world.player_position = Some(at);
            let dt = delta(t);
            world.step_with_delta(dt, player_focus(at), &collision, &mut props, &surfaces);
            let cars = world
                .cars
                .iter()
                .map(|car| {
                    let body = car.body();
                    let [px, py, pz] = body.position.to_array().map(f32::to_bits);
                    let [vx, vy, vz] = body.velocity.to_array().map(f32::to_bits);
                    let [fx, fz] = body.forward.to_array().map(f32::to_bits);
                    [px, py, pz, vx, vy, vz, fx, fz]
                })
                .collect();
            (cars, world.rng.nav.clone().next_f32().to_bits())
        })
        .collect()
}

#[test]
fn the_traffic_step_is_the_same_on_one_thread_and_on_four() {
    crate::scenes::world::ai::parallel::tests::init_pool();
    let threaded = Parallel { min_cars: 1, per_thread: 1, max_threads: 4 };
    assert_eq!(threaded.threads(9), 4, "the parallel run must really share the cars out");
    let serial = run(Parallel::SERIAL);
    assert!(serial.iter().all(|(cars, _)| cars.len() == 9), "every car stays in the step");
    assert_ne!(serial.first().map(|t| &t.0), serial.last().map(|t| &t.0), "the cars must move");
    let parallel = run(threaded);
    for (tick, (serial, parallel)) in serial.iter().zip(&parallel).enumerate() {
        assert_eq!(serial, parallel, "tick {tick}");
    }
    assert_eq!(serial.len(), parallel.len());
}

#[test]
fn changing_scaled_tick_deltas_match_in_serial_and_parallel() {
    crate::scenes::world::ai::parallel::tests::init_pool();
    let delta = |t: u32| match t % 3 {
        0 => STEP,
        1 => STEP * 0.25,
        _ => 1.0 / 144.0,
    };
    let serial = run_with_deltas(Parallel::SERIAL, delta);
    let parallel = run_with_deltas(Parallel { min_cars: 1, per_thread: 1, max_threads: 4 }, delta);
    assert_eq!(serial, parallel);
    assert_ne!(serial, run(Parallel::SERIAL), "scaled time must change actual car integration");
}
