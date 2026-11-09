//! The air control and the arcade help on jumps (`docs/specs/air-control.md`, `docs/specs/ride6-bike-jumps.md`): a bike on flat ground that finds
//! itself in the air (the ground is taken away, or lowered) and what it does there and on landing.

use glam::Vec3;

use crate::FIXED_STEP;
use crate::air_control::AirAxes;
use crate::bike::{ArcadeAir, ArcadeSteering, Bike, BikeInput, BikeSpec, TerrainClass};
use crate::ground::{FlatGround, NoGround, SlopedGround};

/// The example bike made a bike of `class` (its fork travel and its front wheel) with the given steering.
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

/// The reference game's bike, and this game's arcade bike without the help on jumps (what the game rode before it).
fn reference() -> ArcadeSteering {
    ArcadeSteering::REFERENCE
}

fn without_air_help() -> ArcadeSteering {
    ArcadeSteering { air: ArcadeAir::REFERENCE, ..ArcadeSteering::default() }
}

/// The throttle that holds `speed`.
fn hold(speed: f32, bike: &Bike) -> BikeInput {
    BikeInput { throttle: (0.25 + 0.2 * (speed - bike.velocity().length())).clamp(0.0, 1.0), ..Default::default() }
}

/// A bike riding at `speed` on flat ground for a third of a second.
fn riding(spec: BikeSpec, speed: f32) -> Bike {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    for _ in 0..20 {
        let input = hold(speed, &bike);
        bike.step(FIXED_STEP, &input, &ground);
    }
    bike
}

/// A bike that leaves the ground at `speed` with the air control `axes` and falls for `steps` steps with no ground below.
fn in_air(spec: BikeSpec, speed: f32, axes: Option<AirAxes>, steps: usize) -> Bike {
    let mut bike = riding(spec, speed);
    bike.set_air_control(axes);
    for _ in 0..steps {
        bike.step(FIXED_STEP, &BikeInput::default(), &NoGround);
    }
    bike
}

/// The angular velocity of the bike in its own axes (x right, y up, z forward).
fn local_spin(bike: &Bike) -> Vec3 {
    bike.body().rotation().transpose() * bike.body().angular_velocity
}

fn tilt(bike: &Bike) -> f32 {
    (bike.orientation() * Vec3::Y).y
}

#[test]
fn the_air_control_pitches_rolls_and_spins_a_bike_that_has_left_the_ground() {
    let spec = || spec_of(TerrainClass::Offroad, reference());
    let idle = local_spin(&in_air(spec(), 20.0, None, 30));
    let pitch = local_spin(&in_air(spec(), 20.0, Some(AirAxes { pitch: 1.0, ..AirAxes::ZERO }), 30));
    assert!(pitch.x > idle.x + 0.8, "the stick forward turns the nose down: {} against {}", pitch.x, idle.x);
    let back = local_spin(&in_air(spec(), 20.0, Some(AirAxes { pitch: -1.0, ..AirAxes::ZERO }), 30));
    assert!(back.x < idle.x - 0.8, "the stick back lifts it: {}", back.x);
    let spin = local_spin(&in_air(spec(), 20.0, Some(AirAxes { yaw: 1.0, ..AirAxes::ZERO }), 30));
    assert!(spin.y > idle.y + 0.8, "a spin to the right: {}", spin.y);
    let roll = in_air(spec(), 20.0, Some(AirAxes { roll: 1.0, ..AirAxes::ZERO }), 24);
    assert!(roll.lean() > 0.2, "the stick right rolls the right side down: {}", roll.lean());
    assert!(!roll.fallen());
}

#[test]
fn the_air_control_does_nothing_on_the_ground_or_when_it_is_off() {
    let ground = FlatGround::new(0.0);
    let ride = |axes: Option<AirAxes>| {
        let mut bike = riding(spec_of(TerrainClass::Offroad, reference()), 20.0);
        bike.set_air_control(axes);
        for _ in 0..90 {
            let input = hold(20.0, &bike);
            bike.step(FIXED_STEP, &input, &ground);
        }
        bike
    };
    let pushed = ride(Some(AirAxes { pitch: 1.0, roll: 1.0, yaw: 1.0 }));
    let plain = ride(None);
    assert_eq!(pushed.orientation(), plain.orientation(), "a wheel on the ground: the controls do nothing");
    assert_eq!(pushed.position(), plain.position());
    let off = in_air(spec_of(TerrainClass::Offroad, reference()), 20.0, None, 30);
    let still_off = in_air(spec_of(TerrainClass::Offroad, reference()), 20.0, Some(AirAxes::ZERO), 30);
    assert!(local_spin(&still_off).length() <= local_spin(&off).length() + 0.2, "no input only damps");
}

#[test]
fn a_held_stick_stops_short_of_the_fall_limits() {
    let bike =
        in_air(spec_of(TerrainClass::Offroad, reference()), 20.0, Some(AirAxes { pitch: 1.0, ..AirAxes::ZERO }), 180);
    assert!(!bike.fallen(), "three seconds of the stick forward in the air");
    let limit = bike.spec().fall.extra.airborne_pitch.max(bike.spec().fall.pitch);
    assert!(-bike.pitch() < limit, "{} of {}", bike.pitch(), limit);
    assert!(-bike.pitch() > 0.5 * limit, "it did pitch");
}

#[test]
fn a_bike_thrown_nose_up_off_a_ramp_levels_in_the_air_and_the_reference_flips() {
    let spun = |arcade: ArcadeSteering| {
        let mut bike = riding(spec_of(TerrainClass::Offroad, arcade), 20.0);
        for _ in 0..2 {
            bike.step(FIXED_STEP, &BikeInput::default(), &NoGround);
        }
        let right = bike.body().rotation() * Vec3::X;
        bike.body_mut().angular_velocity = right * -2.0;
        for _ in 0..90 {
            bike.step(FIXED_STEP, &BikeInput::default(), &NoGround);
        }
        bike
    };
    let helped = spun(ArcadeSteering::default());
    let plain = spun(reference());
    assert!(!helped.fallen());
    assert!(tilt(&helped) > 0.9, "levelled: {}", tilt(&helped));
    assert!(tilt(&plain) < 0.5, "the reference carries its spin over: {}", tilt(&plain));
}

#[test]
fn the_bike_pitches_to_the_slope_it_will_land_on() {
    let pitch_before_landing = |arcade: ArcadeSteering| {
        let flat = FlatGround::new(0.0);
        let mut bike = Bike::new(spec_of(TerrainClass::Offroad, arcade));
        assert!(bike.place_on_ground(&flat, 0.0, 0.0, 2.0, 0.0));
        let slope = SlopedGround { height: -4.0, slope: 0.36, surface: Default::default() };
        let (mut pitch, mut flew) = (0.0, false);
        for _ in 0..120 {
            bike.step(FIXED_STEP, &BikeInput::default(), &slope);
            if bike.wheels_on_ground() == 0 {
                pitch = bike.pitch();
                flew = true;
                continue;
            }
            if flew {
                break;
            }
        }
        pitch
    };
    let helped = pitch_before_landing(ArcadeSteering::default());
    let plain = pitch_before_landing(reference());
    assert!(helped > 0.1, "nose up to the uphill landing: {helped}");
    assert!(plain.abs() < 0.05, "the reference keeps its attitude: {plain}");
}

#[test]
fn the_bars_are_centred_in_the_air() {
    let bars = |arcade: ArcadeSteering| {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec_of(TerrainClass::Offroad, arcade));
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
        for _ in 0..20 {
            bike.step(FIXED_STEP, &BikeInput { steer: 1.0, ..hold(20.0, &bike) }, &ground);
        }
        for _ in 0..30 {
            bike.step(FIXED_STEP, &BikeInput { steer: 1.0, ..Default::default() }, &NoGround);
        }
        bike.steer_angle().abs()
    };
    let centred = bars(ArcadeSteering::default());
    assert!(centred < 0.1, "centred: {centred}");
    assert!(centred <= bars(reference()) + 1e-3, "never more turned than the reference's");
}

/// The bike at `speed` finds the ground `drop` m lower than it was: the longest wheel load, whether it fell, and whether a wheel touched.
fn drop_of(spec: BikeSpec, speed: f32, drop: f32) -> (f32, bool, bool) {
    let mut bike = riding(spec, speed);
    let lowered = FlatGround::new(-drop);
    let (mut peak, mut landed) = (0.0_f32, false);
    for _ in 0..240 {
        let input = hold(speed, &bike);
        bike.step(FIXED_STEP, &input, &lowered);
        peak = peak.max(bike.wheel(0).load).max(bike.wheel(1).load);
        landed |= bike.wheels_on_ground() > 0;
    }
    (peak, bike.fallen(), landed)
}

#[test]
fn a_bike_lands_a_drop_of_three_metres_at_speed_on_its_wheels() {
    for class in [TerrainClass::Road, TerrainClass::Motard, TerrainClass::Offroad] {
        for speed in [15.0, 28.0] {
            let (_, fell, landed) = drop_of(spec_of(class, ArcadeSteering::default()), speed, 3.0);
            assert!(landed, "{class:?} at {speed} m/s");
            assert!(!fell, "{class:?} at {speed} m/s fell after a drop of 3 m");
        }
    }
}

#[test]
fn the_landing_load_is_held_under_the_cap_and_never_above_what_it_was() {
    let weight = |spec: &BikeSpec| spec.total_mass() * 9.81;
    for class in [TerrainClass::Motard, TerrainClass::Offroad] {
        let helped_spec = spec_of(class, ArcadeSteering::default());
        let cap = helped_spec.steering.arcade.air.load_cap_g * weight(&helped_spec);
        let (helped, _, _) = drop_of(helped_spec, 20.0, 4.0);
        let (plain, _, _) = drop_of(spec_of(class, without_air_help()), 20.0, 4.0);
        assert!(helped <= cap * 1.02, "{class:?}: {helped} N over a cap of {cap} N");
        assert!(helped <= plain * 1.05 + 1.0, "{class:?}: {helped} N with the help, {plain} N without");
    }
}

#[test]
fn a_landing_is_forgiven_for_a_while_and_a_hop_is_not() {
    let landing = |arcade: ArcadeSteering, drop: f32| {
        let mut bike = riding(spec_of(TerrainClass::Offroad, arcade), 20.0);
        let lowered = FlatGround::new(-drop);
        let mut up = 0.0;
        for _ in 0..240 {
            let input = hold(20.0, &bike);
            bike.step(FIXED_STEP, &input, &lowered);
            up = bike.air_time().max(up);
            if up > 0.0 && bike.wheels_on_ground() > 0 {
                break;
            }
        }
        // The step after the touch-down is the one that notices it.
        bike.step(FIXED_STEP, &hold(20.0, &bike), &lowered);
        let forgiven = bike.landing_forgiven();
        for _ in 0..80 {
            let input = hold(20.0, &bike);
            bike.step(FIXED_STEP, &input, &lowered);
        }
        (forgiven, bike.landing_forgiven())
    };
    assert_eq!(landing(ArcadeSteering::default(), 2.0), (true, false), "forgiven after a jump, and for a time only");
    assert!(!landing(ArcadeSteering::default(), 0.3).0, "a hop is no jump");
    assert_eq!(landing(reference(), 2.0), (false, false), "the reference forgives nothing");
}

#[test]
fn a_spin_on_landing_that_throws_the_reference_bike_is_forgiven_by_the_help() {
    let fell = |arcade: ArcadeSteering, spin: f32| {
        let mut bike = riding(spec_of(TerrainClass::Offroad, arcade), 20.0);
        let lowered = FlatGround::new(-2.0);
        for _ in 0..240 {
            let input = hold(20.0, &bike);
            bike.step(FIXED_STEP, &input, &lowered);
            if bike.air_time() > 0.0 && bike.wheels_on_ground() > 0 {
                break;
            }
        }
        let forward = bike.body().rotation() * Vec3::X;
        bike.body_mut().angular_velocity = forward * spin;
        bike.step(FIXED_STEP, &hold(20.0, &bike), &lowered);
        bike.fallen()
    };
    let limit = BikeSpec::example().fall.angular_speed * ArcadeSteering::default().fall_angular_speed;
    let spin = 1.4 * limit;
    assert!(fell(without_air_help(), spin), "a spin of {spin} rad/s is over the limit of {limit}");
    assert!(!fell(ArcadeSteering::default(), spin), "the landing is forgiven");
    assert!(fell(ArcadeSteering::default(), 30.0), "a tumble is still a fall");
}

#[test]
fn the_reference_steering_has_no_help_in_the_air_and_no_cap() {
    assert_eq!(ArcadeSteering::REFERENCE.air, ArcadeAir::REFERENCE);
    let bike = Bike::new(spec_of(TerrainClass::Offroad, reference()));
    assert_eq!(bike.soft_load(123456.0), 123456.0);
    let game = Bike::new(spec_of(TerrainClass::Offroad, ArcadeSteering::default()));
    let cap = game.spec().steering.arcade.air.load_cap_g * game.body().mass() * 9.81;
    assert_eq!(game.soft_load(100.0), 100.0, "a light load is left alone");
    assert!(game.soft_load(10.0 * cap) <= cap && game.soft_load(10.0 * cap) > 0.99 * cap);
    assert!(
        game.soft_load(0.8 * cap) < 0.8 * cap && game.soft_load(0.8 * cap) > 0.6 * cap,
        "the knee bends it smoothly"
    );
}
