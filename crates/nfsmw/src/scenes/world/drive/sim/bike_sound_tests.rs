//! What a bike tells the car sound while it rides fast, and after it has lain down: no crash, no scrape, no landing, unless something is
//! hit (`docs/specs/ride6-feel-round12.md`, section 1). The report was a crash of the bike on the ground heard all the time when going
//! fast. Two causes were found and are pinned here: the body double's last contact of a fall was read again as a new impact in every
//! later step, and the car sound's "tipped" landing latched for good on a vehicle with two wheels.
//!
//! These tests were written without running them.

use blackbox_vehicle::bike::{BikeSpec, FallKind};
use blackbox_vehicle::ground::{Ground, GroundHit, SurfaceGrip};
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use nfsmw_data::car::physics::{CarBounds, CarPhysics, PerformancePackage, SurfaceTable};

use super::*;
use crate::scenes::world::drive::sound::SoundFeed;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };
/// The height of the road's mean surface (m), as `spawn` has it.
const BASE: f32 = 5.0;

fn bike_sim() -> CarSim {
    CarSim::new(CarPhysics::from_bike(Arc::new(BikeSpec::example()), 0.0, BOUNDS), [0.32; 4])
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, BASE), heading: 0.0 }
}

/// A road with bumps a few millimetres high, in both directions (physics space, y up): the surface of a worn road a bike crosses at
/// speed. Its normal is that of the surface, its rays are those the wheels cast downwards.
struct RoughRoad {
    amplitude: f32,
    wavelength: f32,
}

impl RoughRoad {
    fn height(&self, x: f32, z: f32) -> f32 {
        let turn = std::f32::consts::TAU;
        BASE + self.amplitude * 0.5 * ((turn * z / self.wavelength).sin() + (turn * x / 1.7).sin())
    }

    fn normal(&self, x: f32, z: f32) -> Vec3 {
        let step = 0.05;
        let dx = (self.height(x + step, z) - self.height(x - step, z)) / (2.0 * step);
        let dz = (self.height(x, z + step) - self.height(x, z - step)) / (2.0 * step);
        Vec3::new(-dx, 1.0, -dz).normalize()
    }
}

impl Ground for RoughRoad {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if dir.y > -0.2 {
            return None;
        }
        let mut distance = (origin.y - self.height(origin.x, origin.z)) / -dir.y;
        for _ in 0..4 {
            let at = origin + dir * distance;
            distance += (at.y - self.height(at.x, at.z)) / -dir.y;
        }
        if !(0.0..=max_distance).contains(&distance) {
            return None;
        }
        let at = origin + dir * distance;
        Some(GroundHit { distance, normal: self.normal(at.x, at.z), surface: SurfaceGrip::DEFAULT })
    }
}

/// One step through the sound feed, as the game does it. Whether it was quiet: no contact of the body or of the world in the step, and
/// none in what the feed hands to the sound.
fn quiet_step(sim: &mut CarSim, feed: &mut SoundFeed, ground: &dyn Ground, input: &DriveInput) -> bool {
    let impact = sim.step(input, ground, None);
    let quiet = impact.sounds.is_empty() && sim.vehicle.body_contacts().is_empty();
    feed.after_step(sim, &SurfaceTable::default(), &impact, input, FIXED_STEP, false);
    quiet
}

#[test]
fn a_bike_at_60_to_90_ms_over_a_rough_road_makes_no_crash_scrape_or_landing_sound() {
    let road = RoughRoad { amplitude: 0.003, wavelength: 6.0 };
    for speed in [60.0_f32, 75.0, 90.0] {
        let mut sim = bike_sim();
        let mut feed = SoundFeed::default();
        assert!(sim.place_moving(&road, spawn(), speed));
        for tick in 0..(5.0 / FIXED_STEP) as usize {
            let throttle = ((speed - sim.telemetry().speed_mps) * 0.2 + 0.1).clamp(0.0, 1.0);
            let input = DriveInput { throttle, ..DriveInput::default() };
            assert!(quiet_step(&mut sim, &mut feed, &road, &input), "{speed} m/s, step {tick}: a contact");
            assert!(!sim.rider_off(), "{speed} m/s, step {tick}: the rider fell");
        }
        assert!(sim.telemetry().speed_mps > 25.0, "{speed} m/s: the run was not fast: {}", sim.telemetry().speed_mps);
        let state = feed.state("KTM450", &Telemetry::default(), PerformancePackage::default());
        // Every tick of the run, drained once: the feed holds only the last ones, but none may carry a contact.
        assert!(state.ticks.iter().all(|tick| tick.contacts.is_empty()), "{speed} m/s");
        assert!(state.input.two_wheeler, "the sound is told it is a bike");
        assert!(state.input.up_dot > 0.99, "a bike under its rider is never tipped for the sound");
    }
}

#[test]
fn a_bike_leaned_hard_at_speed_is_not_tipped_for_the_sound_and_for_the_police() {
    let ground = FlatGround::new(BASE);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 30.0));
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        let input = DriveInput { throttle: 0.3, steer: 1.0, ..DriveInput::default() };
        let _ = sim.step(&input, &ground, None);
    }
    let lean = sim.ride.as_ref().map(|ride| ride.bike.lean().abs()).unwrap_or(0.0);
    assert!(lean > 0.4, "the bike leans hard in the corner: {lean}");
    assert_eq!(sim.up_dot(), 1.0);
}

#[test]
fn a_bike_that_fell_and_stood_up_again_does_not_crash_on_the_ground_in_every_later_step() {
    let ground = FlatGround::new(BASE);
    let mut sim = bike_sim();
    let mut feed = SoundFeed::default();
    assert!(sim.place_moving(&ground, spawn(), 25.0));
    let idle = DriveInput::default();
    for _ in 0..60 {
        let _ = quiet_step(&mut sim, &mut feed, &ground, &idle);
    }
    sim.throw_rider(FallKind::Generic);
    // It slides, lies and is stood up again (the rider is ready by default).
    let mut lay_down = false;
    let mut steps = 0;
    while steps < 20 * 60 {
        let _ = quiet_step(&mut sim, &mut feed, &ground, &idle);
        steps += 1;
        lay_down |= sim.rider_off();
        if lay_down && !sim.rider_off() {
            break;
        }
    }
    assert!(lay_down && !sim.rider_off(), "the bike went down and was stood up again");
    for step in 0..120 {
        assert!(
            quiet_step(&mut sim, &mut feed, &ground, &idle),
            "step {step} after the get-up still has the fall's contact"
        );
    }
}
