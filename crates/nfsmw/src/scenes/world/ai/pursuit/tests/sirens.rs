//! The world feeds the audio one siren source per police car, and the state follows the chase.
use blackbox_pursuit::siren::State;

use super::*;
use crate::scenes::world::ai::PlayerView;

fn patrol(world: &mut TrafficWorld, at: Vec3) {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let model = cop_model(&mut renderer, "copx", player());
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, at, Vec3::Z, false));
    let position = space::to_render(at.to_array());
    let heading = space::to_render(Vec3::Z.to_array());
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position, heading: heading.y.atan2(heading.x) },
        Start { speed: 10.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &WorldGround { collision: &floor(), surfaces: &SurfaceTable::default() },
    )
    .unwrap();
    world.cars.push(car);
    world.add_cop_model(model);
}

fn police_tick(world: &mut TrafficWorld) {
    world.seen_by_cops = true;
    world.update_police(
        0.25,
        &PlayerView {
            nos_input: false,
            nitrous_boost: 0.0,
            velocity: (Vec3::X) * (10.0),
            position: Vec3::ZERO,
            forward: Vec3::X,
            speed: 10.0,
            nitrous: 0.0,
            throttle: 0.0,
        },
        &|_, _| true,
    );
}

fn states(world: &mut TrafficWorld) -> Vec<State> {
    world.siren_frame(0.1).expect("a police car").sources.iter().map(|s| s.state).collect()
}

#[test]
fn a_patrol_car_is_a_silent_source_and_a_chase_gives_the_cop_its_siren() {
    let mut world = TrafficWorld::new(cop_road());
    assert!(world.siren_frame(0.1).is_none(), "no police, no sirens");
    patrol(&mut world, Vec3::new(2.0, 0.0, 20.0));
    // The patrol cruises with its light bar dark for a while: silent.
    for _ in 0..200 {
        assert_eq!(states(&mut world), [State::Off]);
    }
    let frame = world.siren_frame(0.1).unwrap();
    assert_eq!(frame.sources.len(), 1);
    assert_eq!(frame.sources[0].id, world.cars[0].audio_emitter().id, "the source is the car's own identity");

    // The chase begins and runs past its opening: the same car, its lights on, now sounds.
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars[0].emergency_lights());
    // The player is driving fast: no collapse, so the cops do not press.
    world.player_speed = 40.0;
    let mut seen = Vec::new();
    for _ in 0..300 {
        police_tick(&mut world);
        seen.extend(states(&mut world));
    }
    // Opening yelps, the starting yelp for six seconds, then (a hot chase yelps on until the cycle) the wail.
    assert!(seen.contains(&State::Yelp), "{seen:?}");
    assert!(seen.contains(&State::Wail), "{seen:?}");
    assert!(world.siren_status().contains("Wail"), "{}", world.siren_status());
}

#[test]
fn a_roadblock_car_wails_and_stops_when_its_lights_go() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, Vec3::new(2.0, 0.0, 20.0));
    world.cars[0].set_roadblock();
    assert_eq!(states(&mut world), [State::Wail]);
    world.cars[0].hold_for_arrest();
    assert_eq!(states(&mut world), [State::Off], "stopped for the arrest, light bar dark");
}

#[test]
fn a_car_that_is_gone_leaves_the_frame() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, Vec3::new(2.0, 0.0, 20.0));
    assert_eq!(states(&mut world).len(), 1);
    world.cars.clear();
    assert!(world.siren_frame(0.1).is_none());
    assert_eq!(world.siren_status(), "no police cars");
}
