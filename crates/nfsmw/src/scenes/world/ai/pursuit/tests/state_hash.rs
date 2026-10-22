//! The traffic world's part of the state hash (`ai/state_hash.rs`): a chase run twice from the same seed hashes the same
//! every second, and another seed does not. The installed test of the whole world is `determinism_tests.rs`.

use super::*;
use crate::scenes::world::StateHasher;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::drive::sight::sight_cast;
use crate::scenes::world::props::PropWorld;

/// The player in physics space at tick `t`: up the cop road's lane at 12 m/s from z = -40 m.
fn player_at(t: u32) -> Vec3 {
    Vec3::new(2.0, 0.0, -40.0 + 12.0 * t as f32 * STEP)
}

/// Five seconds of a chase from `seed` (the default seed with `None`); the hash after every second.
fn chase(seed: Option<u64>) -> Vec<u64> {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let mut world = TrafficWorld::new(cop_road());
    if let Some(seed) = seed {
        world.reseed(seed);
    }
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.player_position = Some(player_at(0));
    world.start_pursuit(1, vec![wave()], player());
    let spot =
        CopSpot { at: Vec3::new(2.0, 0.0, -55.0), target: player_at(0), speed: 12.0, player: player(), band: None };
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    let cast = sight_cast(&collision);
    let mut hashes = Vec::new();
    for t in 0..300 {
        let at = player_at(t);
        world.player_position = Some(at);
        let body = Body { position: at, velocity: Vec3::Z * 12.0, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 };
        let focus = Focus { position: [0.0, 0.0], heading: 0.0, speed: 12.0, body: Some(body) };
        world.step(focus, &collision, &mut props, &surfaces);
        world.update_sight(STEP, &cast);
        let view = PlayerView {
            position: space::to_render(at.to_array()),
            forward: space::to_render(Vec3::Z.to_array()),
            speed: 12.0,
            nitrous: 0.0,
            throttle: 1.0,
        };
        world.update_police(STEP, &view, &|_, _| true);
        if t % 60 == 59 {
            let mut hash = StateHasher::default();
            world.hash_into(&mut hash);
            hashes.push(hash.finish());
        }
    }
    hashes
}

#[test]
fn the_same_seed_gives_the_same_hash_every_second() {
    let first = chase(Some(7));
    assert_eq!(first.len(), 5);
    assert_eq!(first, chase(Some(7)));
    assert_eq!(chase(None), chase(None));
}

#[test]
fn another_seed_gives_another_hash() {
    assert_ne!(chase(Some(7)), chase(Some(8)), "the random streams are part of the state");
}
