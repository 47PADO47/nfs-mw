//! A chase must not depend on how the physics ticks group into rendered frames: the traffic step, the sight test and
//! the police run once per tick (`simulation.rs`), so 30, 60 and 144 FPS and uneven frames with a stall give the same
//! cops, sightings, busted meter and police random stream (`docs/specs/multiplayer/determinism.md`, D2 and D4).

use super::*;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::drive::sight::sight_cast;
use crate::scenes::world::drive::{FixedClock, STEP};
use crate::scenes::world::props::PropWorld;

/// What one tick leaves behind: the chase flags, the meter, the sighting, the cars and the next police draw.
#[derive(Debug, PartialEq)]
struct TickState {
    pursuing: bool,
    busted: bool,
    meter: u32,
    seen: bool,
    cars: Vec<[u32; 4]>,
    police_draw: u32,
}

/// The player in physics space at tick `t`: driving up the cop road's lane at 12 m/s from z = -40 m.
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

/// Runs a chase for the given frame times, ticking the traffic, the sight test and the police once per physics tick
/// as the world scene does, and records the state after every tick.
fn run(frame_times: impl IntoIterator<Item = f32>) -> Vec<TickState> {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.player_position = Some(player_at(0));
    world.start_pursuit(1, vec![wave()], player());
    let spot =
        CopSpot { at: Vec3::new(2.0, 0.0, -55.0), target: player_at(0), speed: 12.0, player: player(), band: None };
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    let cast = sight_cast(&collision);
    let mut clock = FixedClock::default();
    let mut trace = Vec::new();
    for dt in frame_times {
        for _ in 0..clock.advance(dt) {
            let t = trace.len() as u32;
            let at = player_at(t);
            world.player_position = Some(at);
            world.step(player_focus(at), &collision, &mut props, &surfaces);
            world.update_sight(STEP, &cast);
            let view = PlayerView {
                nos_input: false,
                nitrous_boost: 0.0,
                velocity: (space::to_render(Vec3::Z.to_array())) * (12.0),
                position: space::to_render(at.to_array()),
                forward: space::to_render(Vec3::Z.to_array()),
                speed: 12.0,
                nitrous: 0.0,
                throttle: 1.0,
            };
            world.update_police(STEP, &view, &|_, _| true);
            trace.push(TickState {
                pursuing: world.is_pursuing(),
                busted: world.is_busted(),
                meter: world.busted_meter().to_bits(),
                seen: world.seen_by_cops,
                cars: world
                    .cars
                    .iter()
                    .map(|car| {
                        let p = car.physics_position();
                        [p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), car.speed().to_bits()]
                    })
                    .collect(),
                police_draw: world.rng.police.clone().next_f32().to_bits(),
            });
        }
    }
    trace
}

#[test]
fn a_chase_is_the_same_at_30_60_and_144_fps_and_after_uneven_frames() {
    let seconds = 5;
    let reference = run(std::iter::repeat_n(1.0 / 60.0, 60 * seconds));
    assert_eq!(reference.len(), 300);
    assert!(reference.iter().any(|tick| tick.seen), "the scenario must exercise the sight test");
    assert!(reference.iter().all(|tick| !tick.cars.is_empty()), "the cop stays in the chase");
    for fps in [30, 144] {
        let trace = run(std::iter::repeat_n(1.0 / fps as f32, fps * seconds));
        assert_eq!(trace.len(), reference.len(), "tick count at {fps} FPS");
        assert_eq!(trace, reference, "chase state at {fps} FPS");
    }
    // Uneven frames with a 300 ms stall: every owed tick runs the police once.
    let cycle: [f32; 5] = [1.0 / 144.0, 1.0 / 30.0, 0.3, 1.0 / 240.0, 1.0 / 65.0];
    let mut remaining = seconds as f64;
    let mut frames = Vec::new();
    for dt in cycle.into_iter().cycle() {
        if remaining <= 1e-10 {
            break;
        }
        let elapsed = (dt as f64).min(remaining) as f32;
        frames.push(elapsed);
        remaining -= f64::from(elapsed);
    }
    let uneven = run(frames);
    assert_eq!(uneven, reference, "chase state after uneven frames and a stall");
}
