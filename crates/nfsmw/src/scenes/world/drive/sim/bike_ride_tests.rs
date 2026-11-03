//! The bike's ride: the two bodies, the levers and the fall.

use std::sync::Arc;

use blackbox_vehicle::bike::BikeSpec;
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::Quat;
use nfsmw_data::car::physics::{BikePhysics, CarBounds};

use super::lie::GET_UP_AFTER;
use super::*;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.05) };

fn physics() -> CarPhysics {
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    physics.bike = Some(BikePhysics { spec: Arc::new(BikeSpec::example()), origin_x: -0.02 });
    physics
}

fn ride_and_double() -> (BikeRide, Vehicle) {
    let mut physics = physics();
    let ride = BikeRide::new(&mut physics).expect("a bike");
    let mut vehicle = Vehicle::new(physics.spec);
    ride.attach(&mut vehicle);
    (ride, vehicle)
}

#[test]
fn a_car_has_no_ride() {
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    assert!(BikeRide::new(&mut physics).is_none());
}

#[test]
fn the_double_has_the_mass_and_the_inertia_of_the_bike() {
    let (ride, vehicle) = ride_and_double();
    let (bike, double) = (ride.bike.body(), vehicle.body());
    assert!((bike.mass() - double.mass()).abs() < 1e-3);
    let (a, b) = (bike.inertia(), double.inertia());
    assert!(((a - b) / a).abs().max_element() < 1e-4, "{a} {b}");
}

#[test]
fn the_two_bodies_share_their_centre_of_gravity_and_their_velocities() {
    let (mut ride, mut vehicle) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (3.0, -2.0, 2.0), 0.7, 12.0));
    ride.push(&mut vehicle);
    let (bike, double) = (ride.bike.body(), vehicle.body());
    assert!((bike.world_cog() - double.world_cog()).length() < 1e-4);
    assert!((bike.linear_velocity - double.linear_velocity).length() < 1e-6);
    assert_eq!(bike.orientation, double.orientation);
    // The double's origin is the bike's plus the offset between the model's origin and the box centre.
    let expected = bike.position + bike.rotation() * (BOUNDS.pivot - Vec3::new(0.0, 0.0, -0.02));
    assert!((double.position - expected).length() < 1e-4);
}

#[test]
fn what_a_contact_does_to_the_double_the_bike_takes() {
    let (mut ride, mut vehicle) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, 10.0));
    ride.push(&mut vehicle);
    // A wall stops the double and turns it.
    vehicle.body_mut().linear_velocity = Vec3::new(0.0, 0.0, 2.0);
    vehicle.body_mut().angular_velocity = Vec3::new(0.0, 1.5, 0.0);
    vehicle.body_mut().position += Vec3::new(0.1, 0.0, 0.0);
    let cog = vehicle.body().world_cog();
    ride.pull(&vehicle);
    let bike = ride.bike.body();
    assert_eq!(bike.linear_velocity, Vec3::new(0.0, 0.0, 2.0));
    assert_eq!(bike.angular_velocity, Vec3::new(0.0, 1.5, 0.0));
    assert!((bike.world_cog() - cog).length() < 1e-4, "the bike moved with the double");
}

#[test]
fn the_pedals_and_the_stick_become_the_riders_levers() {
    let input = InputState { throttle: 0.5, brake: 0.3, steer: -2.0, ..Default::default() };
    let rider = command(&input, 10.0);
    assert_eq!(rider, BikeInput { throttle: 0.5, front_brake: 0.3, rear_brake: 0.3, steer: -1.0 });
    // The full stick gives the whole of the lean limit at any speed (`choices::STICK_SHARE_AT_TOP`).
    assert!((command(&input, 40.0).steer + choices::stick_share(40.0)).abs() < 1e-6);
    assert!((command(&input, 55.0).steer + 1.0).abs() < 1e-6);
    // The handbrake is the rear brake alone.
    let handbrake = command(&InputState { handbrake: 1.0, ..Default::default() }, 0.0);
    assert_eq!((handbrake.front_brake, handbrake.rear_brake), (0.0, 1.0));
    // A harder brake pedal than the handbrake wins at the rear.
    let both = command(&InputState { brake: 0.8, handbrake: 0.5, ..Default::default() }, 0.0);
    assert_eq!((both.front_brake, both.rear_brake), (0.8, 0.8));
    // Shift requests and the clutch are not read, and a number that is not a number is no input.
    let odd = command(&InputState { throttle: f32::NAN, shift_up: true, clutch: 1.0, ..Default::default() }, 0.0);
    assert_eq!(odd, BikeInput::default());
}

#[test]
fn a_rider_who_lay_long_enough_stands_the_bike_up_where_it_is() {
    let (mut ride, _) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (4.0, 6.0, 2.0), 0.3, 0.0));
    // Tip it over: the next response lets go of the bike.
    ride.bike.body_mut().set_orientation(Quat::from_rotation_z(-1.5) * Quat::from_rotation_y(0.3));
    ride.bike.respond(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(ride.bike.fallen());
    ride.lie(GET_UP_AFTER, &ground);
    assert!(!ride.bike.fallen(), "the rider got up");
    assert!(ride.bike.lean().abs() < 0.05);
    let at = ride.bike.position();
    assert!((at.x - 4.0).abs() < 0.5 && (at.z - 6.0).abs() < 0.5, "{at}");
    assert_eq!(ride.down, 0.0);
}

#[test]
fn a_bike_that_has_lain_still_waits_for_its_rider_and_stays_down_until_he_is_ready() {
    let (mut ride, _) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (4.0, 6.0, 2.0), 0.3, 0.0));
    ride.bike.body_mut().set_orientation(Quat::from_rotation_z(-1.5) * Quat::from_rotation_y(0.3));
    ride.bike.respond(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(ride.bike.fallen());
    // The rider is still getting up: the bike lies as long as it takes, far past the time it lies for.
    ride.set_rider_ready(false);
    ride.lie(GET_UP_AFTER - 0.5, &ground);
    assert!(!ride.waiting_for_rider(), "it has not lain still for long enough yet");
    for _ in 0..20 {
        ride.lie(GET_UP_AFTER, &ground);
        assert!(ride.bike.fallen() && ride.bike.rider_off(), "the bike stays down while the rider gets up");
        assert!(ride.waiting_for_rider());
    }
    // He is up: the bike is stood up at the next step, with its rider.
    ride.set_rider_ready(true);
    ride.lie(FIXED_STEP, &ground);
    assert!(!ride.bike.fallen() && !ride.bike.rider_off());
    assert!(!ride.waiting_for_rider());
}

#[test]
fn a_rider_who_has_just_fallen_stays_down() {
    let (mut ride, _) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, 0.0));
    ride.bike.body_mut().set_orientation(Quat::from_rotation_z(-1.5));
    ride.bike.respond(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(ride.bike.fallen());
    ride.lie(0.5, &ground);
    assert!(ride.bike.fallen());
    assert!(ride.down > 0.4);
}

#[test]
fn a_bike_in_the_air_reports_the_wheels_it_has_off_the_ground() {
    let (ride, _) = ride_and_double();
    assert_eq!(ride.wheels_down(), 0);
    assert_eq!(ride.wheel_surface(0), None);
    assert_eq!(ride.wheel_hit(2), None);
    let front = ride.wheel_state(1);
    assert!(!front.on_ground && front.radius == BikeSpec::example().front.radius);
}

/// The keys held at time `t` of a run that opens the throttle and flicks the steering from side to side every `half` seconds.
fn flick_keys(t: f32, half: f32) -> Vec<bevy_input::keyboard::KeyCode> {
    use bevy_input::keyboard::KeyCode;
    let side = if ((t / half) as usize).is_multiple_of(2) { KeyCode::KeyD } else { KeyCode::KeyA };
    vec![KeyCode::KeyW, side]
}

#[test]
fn a_throttle_held_open_with_the_steering_flicked_from_side_to_side_does_not_throw_the_rider() {
    use crate::input::{ActionState, Bindings};
    use crate::scenes::world::drive::input::DriveInput;
    use crate::settings::WheelOptions;
    // A bike with the power of a big sports bike, whose front wheel comes up in the first gears.
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    physics.bike = Some(BikePhysics { spec: Arc::new(spec), origin_x: -0.02 });
    let ground = FlatGround::new(0.0);
    for (start, half) in [(0.0_f32, 0.25_f32), (0.0, 0.5), (6.0, 0.25), (12.0, 0.4), (20.0, 0.25)] {
        let mut fitted = physics.clone();
        let mut ride = BikeRide::new(&mut fitted).expect("a bike");
        let mut vehicle = Vehicle::new(fitted.spec);
        ride.attach(&mut vehicle);
        assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, start));
        ride.push(&mut vehicle);
        let mut actions = ActionState::default();
        let mut front_up = 0.0_f32;
        for step in 0..(6.0 / FIXED_STEP) as usize {
            // The keys, through the game's input layer, to the input the car takes.
            actions.update_for_test(&Bindings::default(), &flick_keys(step as f32 * FIXED_STEP, half), &[], false);
            let input = DriveInput::from_actions(&actions, WheelOptions::default());
            let step_start =
                StepStart { input: input.to_vehicle(), weight: input.weight, delta: FIXED_STEP, terrain: None };
            ride.begin_body(&mut vehicle, FIXED_STEP);
            ride.finish_body(&mut vehicle, FIXED_STEP, &ground);
            ride.respond(&mut vehicle, &step_start, &ground);
            ride.advance(FIXED_STEP);
            front_up = front_up.max(if ride.bike.wheel(0).on_ground { 0.0 } else { FIXED_STEP });
            assert!(!ride.bike.fallen(), "from {start} m/s, flicking every {half} s: fell at step {step}");
        }
        assert!(front_up > 0.0 || start > 0.0, "the front wheel never left the road from a standstill");
    }
}

/// A bike with the power of a big sports bike, ridden from 12 m/s with the first `settle` seconds on `before` and then 3 s on `during`
/// (both the inputs of a physics step, from the game's input layer); the heading it turned through in those 3 s (rad, positive to the
/// right), the share of the time its front wheel was up, and whether the rider was thrown.
fn wheelie_then(
    before: impl Fn(usize) -> crate::scenes::world::drive::input::DriveInput,
    during: impl Fn(usize) -> crate::scenes::world::drive::input::DriveInput,
) -> (f32, f32, bool) {
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    physics.bike = Some(BikePhysics { spec: Arc::new(spec), origin_x: -0.02 });
    let ground = FlatGround::new(0.0);
    let mut ride = BikeRide::new(&mut physics).expect("a bike");
    let mut vehicle = Vehicle::new(physics.spec);
    ride.attach(&mut vehicle);
    assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, 12.0));
    ride.push(&mut vehicle);
    let yaw = |ride: &BikeRide| {
        let forward = ride.bike.orientation() * Vec3::Z;
        forward.x.atan2(forward.z)
    };
    let (settle, held) = ((3.0 / FIXED_STEP) as usize, (3.0 / FIXED_STEP) as usize);
    let (mut from, mut up) = (0.0, 0.0_f32);
    for step in 0..settle + held {
        let input = if step < settle { before(step) } else { during(step - settle) };
        let step_start =
            StepStart { input: input.to_vehicle(), weight: input.weight, delta: FIXED_STEP, terrain: None };
        ride.begin_body(&mut vehicle, FIXED_STEP);
        ride.finish_body(&mut vehicle, FIXED_STEP, &ground);
        ride.respond(&mut vehicle, &step_start, &ground);
        ride.advance(FIXED_STEP);
        if ride.bike.fallen() {
            return (0.0, 0.0, true);
        }
        if step + 1 == settle {
            from = yaw(&ride);
        }
        if step >= settle {
            up += f32::from(u8::from(!ride.bike.wheel(0).on_ground)) * FIXED_STEP;
        }
    }
    (yaw(&ride) - from, up / 3.0, false)
}

#[test]
fn a_wheelie_held_with_v_and_w_turns_with_a_and_d_through_the_input_layer() {
    use bevy_input::keyboard::KeyCode;

    use crate::input::{ActionState, Bindings};
    use crate::scenes::world::drive::input::DriveInput;
    use crate::settings::WheelOptions;
    let from_keys = |keys: &[KeyCode]| {
        let mut actions = ActionState::default();
        actions.update_for_test(&Bindings::default(), keys, &[], false);
        DriveInput::from_actions(&actions, WheelOptions::default())
    };
    // W and V: the throttle and the weight lever back, the wheelie. A and D on top: the steering.
    let wheelie = from_keys(&[KeyCode::KeyW, KeyCode::KeyV]);
    assert!(wheelie.throttle == 1.0 && wheelie.weight == -1.0 && wheelie.steer == 0.0);
    let (straight, up, fell) = wheelie_then(|_| wheelie, |_| wheelie);
    assert!(!fell && up > 0.9, "the wheelie is held: up {up}");
    assert!(straight.abs() < 0.05, "and goes straight: {straight}");
    for (key, side) in [(KeyCode::KeyA, -1.0_f32), (KeyCode::KeyD, 1.0)] {
        let steered = from_keys(&[KeyCode::KeyW, KeyCode::KeyV, key]);
        assert_eq!(steered.steer, side);
        assert!(
            steered.weight == -1.0 && steered.throttle == 1.0,
            "the steering does not take the weight or the throttle"
        );
        let (heading, up, fell) = wheelie_then(|_| wheelie, |_| steered);
        assert!(!fell, "{key:?}");
        assert!(up > 0.9, "the front wheel stays up with {key:?}: {up}");
        assert!(heading * side > 0.4, "{key:?} turned the wheelie only {heading} rad in 3 s");
    }
}

#[test]
fn a_wheelie_held_with_the_stick_down_and_to_the_left_turns_left() {
    use crate::scenes::world::drive::input::DriveInput;
    // The pad's left stick pulled back and to the left on its rim: the lever at its end and the steering about 0.7 (`input/rider_weight_tests.rs`).
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    let pad = DriveInput { throttle: 1.0, steer: -diagonal, weight: -1.0, ..DriveInput::default() };
    let wheelie = DriveInput { throttle: 1.0, weight: -1.0, ..DriveInput::default() };
    let (heading, up, fell) = wheelie_then(|_| wheelie, |_| pad);
    assert!(!fell && up > 0.9, "up {up}");
    assert!(heading < -0.3, "stick down and left turned the wheelie {heading} rad in 3 s");
}
