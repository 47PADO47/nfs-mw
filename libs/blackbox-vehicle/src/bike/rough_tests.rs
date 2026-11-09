//! The arcade help on loose and rough ground (`docs/specs/ride6-bike-rough-ground.md`): bikes of the three classes in a corner on dirt and over a
//! washboard.

use crate::FIXED_STEP;
use crate::bike::{ArcadeSteering, Bike, BikeInput, BikeSpec, TerrainClass};
use crate::ground::{FlatGround, Ground, ProfileGround, SurfaceGrip};

/// Dirt as the host's surface table gives it: well under the road's grip, a little slower to roll on.
const DIRT: SurfaceGrip = SurfaceGrip { lateral: 0.65, drive: 0.7, rolling: 1.5, tag: 3 };

fn spec_of(class: TerrainClass, arcade: ArcadeSteering) -> BikeSpec {
    let mut spec = BikeSpec::example();
    let (travel, radius) = match class {
        TerrainClass::Road => (0.12, 0.30),
        TerrainClass::Motard => (0.25, 0.29),
        TerrainClass::Offroad => (0.27, 0.349),
    };
    spec.front_suspension.max_travel = travel;
    spec.front.radius = radius;
    spec.steering.arcade = arcade;
    spec
}

fn dirt() -> FlatGround {
    FlatGround { height: 0.0, surface: DIRT }
}

/// What a ride came to.
struct Ride {
    fell: bool,
    /// The mean lateral acceleration over the last two seconds, in g.
    lateral_g: f32,
    /// The speed at the end (m/s).
    speed: f32,
}

/// Ride `seconds` at `speed` m/s on `ground` with the stick at `stick` (the throttle holds the speed).
fn ride(spec: BikeSpec, ground: &dyn Ground, speed: f32, stick: f32, seconds: f32) -> Ride {
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(ground, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = (seconds / FIXED_STEP) as usize;
    let (mut sum, mut count) = (0.0_f32, 0.0_f32);
    for step in 0..steps {
        let throttle = (0.25 + 0.2 * (speed - bike.velocity().length())).clamp(0.0, 1.0);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, ground);
        if step + (2.0 / FIXED_STEP) as usize >= steps {
            sum += bike.velocity().length() * bike.body().angular_velocity.y.abs() / 9.81;
            count += 1.0;
        }
    }
    Ride { fell: bike.fallen(), lateral_g: sum / count.max(1.0), speed: bike.velocity().length() }
}

#[test]
fn no_class_of_bike_falls_in_a_dirt_corner_with_the_help() {
    for class in [TerrainClass::Road, TerrainClass::Motard, TerrainClass::Offroad] {
        for speed in [11.0, 22.0] {
            let ride = ride(spec_of(class, ArcadeSteering::default()), &dirt(), speed, 0.8, 5.0);
            assert!(!ride.fell, "{class:?} fell in a dirt corner at {speed} m/s");
        }
    }
}

#[test]
fn an_offroad_bike_corners_harder_on_dirt_than_a_road_bike() {
    let g = |class| ride(spec_of(class, ArcadeSteering::default()), &dirt(), 11.0, 0.8, 6.0).lateral_g;
    let (road, motard, offroad) = (g(TerrainClass::Road), g(TerrainClass::Motard), g(TerrainClass::Offroad));
    assert!(motard >= road * 0.99, "motard {motard} against road {road}");
    assert!(offroad > road * 1.05, "offroad {offroad} against road {road}");
}

#[test]
fn the_bike_follows_the_surface_and_turns_wider_on_dirt_than_on_tarmac() {
    let spec = || spec_of(TerrainClass::Road, ArcadeSteering::default());
    let tarmac = ride(spec(), &FlatGround::new(0.0), 11.0, 0.8, 6.0);
    let loose = ride(spec(), &dirt(), 11.0, 0.8, 6.0);
    assert!(loose.lateral_g < tarmac.lateral_g, "{} on dirt against {} on tarmac", loose.lateral_g, tarmac.lateral_g);
}

#[test]
fn a_washboard_does_not_throw_any_class_and_the_offroad_bike_holds_its_speed() {
    // Four bumps of 5 cm, half a metre long each: a strip of 2 m.
    for length in [0.5, 3.0] {
        let strip = ProfileGround::washboard(10.0, length, 4, 0.05, DIRT);
        for class in [TerrainClass::Road, TerrainClass::Motard, TerrainClass::Offroad] {
            for speed in [11.0, 22.0] {
                let ride = ride(spec_of(class, ArcadeSteering::default()), &strip, speed, 0.0, 4.0);
                assert!(!ride.fell, "{class:?} at {speed} m/s over a strip of {} m", 4.0 * length);
                if class == TerrainClass::Offroad {
                    assert!(ride.speed > 0.7 * speed, "the offroad bike held {} of {speed} m/s", ride.speed);
                }
            }
        }
    }
}

#[test]
fn the_suspension_damps_more_on_loose_ground_for_the_offroad_bike_and_not_on_tarmac() {
    let damping = |class: TerrainClass, ground: &dyn Ground| {
        let mut bike = Bike::new(spec_of(class, ArcadeSteering::default()));
        assert!(bike.place_moving(ground, 0.0, 0.0, 2.0, 0.0, 10.0));
        for _ in 0..90 {
            bike.step(FIXED_STEP, &BikeInput { throttle: 0.3, ..Default::default() }, ground);
        }
        bike.damping_factor()
    };
    assert_eq!(damping(TerrainClass::Offroad, &FlatGround::new(0.0)), 1.0);
    let offroad = damping(TerrainClass::Offroad, &dirt());
    assert!(offroad > 1.3 && offroad <= 1.4 + 1e-4, "{offroad}");
    assert!(damping(TerrainClass::Road, &dirt()) <= 1.0 + 1e-6, "a road bike's damper is the data's");
}

#[test]
fn the_reference_steering_rides_dirt_with_the_tyres_alone() {
    let mut bike = Bike::new(spec_of(TerrainClass::Offroad, ArcadeSteering::REFERENCE));
    let ground = dirt();
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 10.0));
    for _ in 0..60 {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.3, ..Default::default() }, &ground);
    }
    assert_eq!(bike.damping_factor(), 1.0);
    let wheel = bike.wheel(0);
    assert!(wheel.on_ground && wheel.surface_tag == 3);
}
