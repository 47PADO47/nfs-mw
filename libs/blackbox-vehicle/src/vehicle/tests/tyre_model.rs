//! The external tyre model seam: dispatch with a stub model (handled, declined, not finite, airborne), the answer per
//! integration mode, and the proof that a vehicle without a model, or whose model declines, is bit-identical.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::*;
use crate::ground::NoGround;
use crate::tires::{TireIntegration, TyreModel, TyreQuery, TyreReject, TyreResult};

/// What a stub does with each question.
#[derive(Clone, Copy)]
enum Answer {
    /// Return `None`.
    Decline,
    /// Return this result for every wheel.
    Fixed(TyreResult),
    /// A linear tyre: force proportional to the slip speed up to the friction limit, wheel spin integrated implicitly.
    Linear,
}

/// Everything the stub saw, shared with the test.
#[derive(Default)]
struct Log {
    queries: RefCell<Vec<TyreQuery>>,
    rejected: Cell<u32>,
}

struct Stub {
    answer: Answer,
    log: Rc<Log>,
}

impl Stub {
    fn boxed(answer: Answer) -> (Box<dyn TyreModel>, Rc<Log>) {
        let log = Rc::new(Log::default());
        (Box::new(Stub { answer, log: log.clone() }), log)
    }
}

/// The linear tyre of the tests. Friction coefficient 1, slip stiffness 8000 N per m/s, sideways stiffness 3000.
fn linear(q: &TyreQuery) -> TyreResult {
    const MU: f32 = 1.0;
    const K_LONG: f32 = 8000.0;
    const K_LAT: f32 = 3000.0;
    let (r, inertia, dt) = (q.wheel_radius_m, q.wheel_inertia_kgm2, q.dt);
    let brake = q.brake_torque_nm * q.wheel_spin.signum();
    let torque = q.drive_torque_nm - brake;
    if !q.loaded {
        let spin = q.wheel_spin + torque / inertia * dt;
        let spin = if spin * q.wheel_spin < 0.0 { 0.0 } else { spin };
        return TyreResult { wheel_spin: spin, traction: 0.0, ..TyreResult::default() };
    }
    let limit = MU * q.vertical_load_n;
    let implicit = (inertia * q.wheel_spin / dt + torque + r * K_LONG * q.patch_forward_velocity)
        / (inertia / dt + r * r * K_LONG);
    let mut fx = K_LONG * (implicit * r - q.patch_forward_velocity);
    let mut spin = implicit;
    let mut traction = 1.0;
    if fx.abs() > limit {
        fx = fx.clamp(-limit, limit);
        spin = q.wheel_spin + (torque - fx * r) / inertia * dt;
        traction = 0.5;
    }
    if spin * q.wheel_spin < 0.0 && q.brake_torque_nm > 0.0 {
        spin = 0.0;
    }
    let fy = (-K_LAT * q.patch_lateral_velocity).clamp(-limit, limit);
    TyreResult {
        longitudinal_force_n: fx,
        lateral_force_n: fy,
        aligning_torque_nm: 0.0,
        wheel_spin: spin,
        traction,
        slip: spin * r - q.patch_forward_velocity,
        slip_angle: q.patch_lateral_velocity.atan2(q.patch_forward_velocity.abs()) / std::f32::consts::TAU,
    }
}

impl TyreModel for Stub {
    fn wheel(&mut self, query: &TyreQuery) -> Option<TyreResult> {
        self.log.queries.borrow_mut().push(*query);
        match self.answer {
            Answer::Decline => None,
            Answer::Fixed(result) => Some(result),
            Answer::Linear => Some(linear(query)),
        }
    }

    fn rejected(&mut self, _why: TyreReject) {
        self.log.rejected.set(self.log.rejected.get() + 1);
    }
}

fn fixed(longitudinal: f32, spin: f32) -> TyreResult {
    TyreResult {
        longitudinal_force_n: longitudinal,
        lateral_force_n: 0.0,
        aligning_torque_nm: 12.5,
        wheel_spin: spin,
        traction: 0.75,
        slip: 0.4,
        slip_angle: 0.01,
    }
}

fn throttle() -> InputState {
    InputState { throttle: 1.0, ..Default::default() }
}

/// An example car in `integration` mode, settled on flat ground (the mode is set before it settles, as the game does).
fn parked_in(integration: TireIntegration) -> Vehicle {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_tire_integration(integration);
    assert!(car.place_on_ground(&flat(), 0.0, 0.0, 5.0, 0.0));
    run(&mut car, &InputState::default(), 3.0);
    car
}

/// A parked car in `integration`, with the stub installed.
fn hooked(integration: TireIntegration, answer: Answer) -> (Vehicle, Rc<Log>) {
    let mut car = parked_in(integration);
    let (model, log) = Stub::boxed(answer);
    car.set_tyre_model(model, 7);
    (car, log)
}

/// The position and the wheel spins of a car, as bits, for a comparison that must be exact.
fn fingerprint(car: &Vehicle) -> Vec<u32> {
    let p = car.position();
    let mut bits =
        vec![p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), car.forward_speed().to_bits(), car.rpm().to_bits()];
    bits.extend((0..4).map(|i| car.wheel(i).simulated_angular_velocity.to_bits()));
    bits
}

fn drive(car: &mut Vehicle, input: &InputState, steps: usize) {
    let ground = flat();
    for _ in 0..steps {
        car.step(FIXED_STEP, input, &ground);
    }
}

#[test]
fn a_declining_model_changes_nothing() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let mut plain = parked_in(integration);
        let (mut declined, log) = hooked(integration, Answer::Decline);
        drive(&mut plain, &throttle(), 240);
        drive(&mut declined, &throttle(), 240);
        assert_eq!(fingerprint(&plain), fingerprint(&declined), "{integration:?}");
        assert!(log.queries.borrow().len() >= 4 * 240, "the model was asked for every wheel of every step");
        assert_eq!(declined.tyre_model_faults(), 0, "declining is not a fault");
    }
}

#[test]
fn a_removed_model_leaves_no_trace() {
    let mut plain = parked_in(TireIntegration::Regularized);
    let (mut removed, log) = hooked(TireIntegration::Regularized, Answer::Linear);
    removed.clear_tyre_model();
    assert!(!removed.has_tyre_model());
    drive(&mut plain, &throttle(), 120);
    drive(&mut removed, &throttle(), 120);
    assert_eq!(fingerprint(&plain), fingerprint(&removed));
    assert!(log.queries.borrow().is_empty());
}

#[test]
fn a_not_finite_answer_is_refused_counted_and_the_built_in_tyres_run() {
    for bad in [f32::NAN, f32::INFINITY] {
        let mut plain = parked_in(TireIntegration::Regularized);
        let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Fixed(fixed(bad, 1.0)));
        drive(&mut plain, &throttle(), 60);
        drive(&mut car, &throttle(), 60);
        assert_eq!(fingerprint(&plain), fingerprint(&car), "a refused answer leaves the built-in result");
        assert!(car.tyre_model_faults() >= 4 * 60, "faults {}", car.tyre_model_faults());
        assert_eq!(log.rejected.get(), car.tyre_model_faults(), "the model hears about each refusal");
    }
}

#[test]
fn an_out_of_range_answer_is_refused_too() {
    let mut result = fixed(0.0, 1.0);
    result.traction = 3.0;
    let (mut car, _log) = hooked(TireIntegration::Regularized, Answer::Fixed(result));
    drive(&mut car, &throttle(), 10);
    assert!(car.tyre_model_faults() >= 40);
}

#[test]
fn a_handled_answer_fills_the_tire_fields_the_effects_read() {
    let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Fixed(fixed(500.0, 20.0)));
    drive(&mut car, &throttle(), 1);
    for i in 0..4 {
        let wheel = car.wheel(i);
        assert_eq!(wheel.longitudinal_force, 500.0, "wheel {i}");
        assert_eq!(wheel.simulated_angular_velocity, 20.0);
        assert_eq!(wheel.traction, 0.75);
        assert_eq!(wheel.slip, 0.4);
        assert!((wheel.slip_angle - 0.01 * std::f32::consts::TAU).abs() < 1e-6);
        assert!(wheel.load > 0.0, "the load is kept for the sound and the marks");
        assert_eq!(car.wheel_aligning_torque(i), 12.5);
    }
    assert_eq!(car.tyre_model_faults(), 0);
    let queries = log.queries.borrow();
    let first_tick: Vec<u32> = queries.iter().take(4).map(|q| q.wheel_index).collect();
    assert_eq!(first_tick, [0, 1, 2, 3], "wheels are asked in order");
    assert!(queries.iter().all(|q| q.role == 7 && q.loaded && q.dt == FIXED_STEP));
    assert!(queries.iter().all(|q| q.vertical_load_n > 0.0 && q.wheel_radius_m > 0.1 && q.wheel_inertia_kgm2 > 0.0));
    assert!(queries.iter().all(|q| q.surface_lateral == 1.0 && q.tyre_condition == 1.0));
    assert!(queries.iter().all(|q| q.ambient_temperature_c == crate::tires::AMBIENT_TEMPERATURE_C));
}

#[test]
fn the_forces_of_a_handled_wheel_reach_the_body() {
    let (mut car, _log) = hooked(TireIntegration::Regularized, Answer::Fixed(fixed(2000.0, 0.0)));
    drive(&mut car, &throttle(), 30);
    assert!(car.forward_speed() > 2.0, "4 x 2000 N push a parked car for half a second: {}", car.forward_speed());
}

#[test]
fn an_airborne_wheel_is_asked_with_loaded_false_and_only_its_spin_is_used() {
    let mut car = parked();
    let (model, log) = Stub::boxed(Answer::Fixed(fixed(9000.0, 5.0)));
    car.set_tyre_model(model, 0);
    car.step(FIXED_STEP, &InputState::default(), &NoGround);
    let queries = log.queries.borrow();
    assert_eq!(queries.len(), 4);
    assert!(queries.iter().all(|q| !q.loaded && q.vertical_load_n == 0.0 && q.patch_forward_velocity == 0.0));
    assert!(queries.iter().all(|q| q.surface_tag == 0 && q.surface_lateral == 1.0));
    for i in 0..4 {
        let wheel = car.wheel(i);
        assert_eq!(wheel.simulated_angular_velocity, 5.0, "the spin comes from the model");
        assert_eq!(wheel.longitudinal_force, 0.0, "the forces of an airborne wheel are ignored");
        assert_eq!(wheel.traction, 0.0);
        assert_eq!(car.wheel_aligning_torque(i), 0.0);
    }
}

#[test]
fn a_linear_model_drives_the_car_in_both_integration_modes() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let (mut car, log) = hooked(integration, Answer::Linear);
        drive(&mut car, &throttle(), 300);
        let speed = car.forward_speed();
        assert!(speed.is_finite() && speed > 6.0, "{integration:?}: speed after 5 s is {speed}");
        assert!(car.position().x.abs() < 3.0, "{integration:?}: drifted {}", car.position().x);
        assert!(up(&car).y > 0.95, "{integration:?}: the car stays upright");
        assert_eq!(car.tyre_model_faults(), 0, "{integration:?}");
        for i in 0..4 {
            let wheel = car.wheel(i);
            let spin_speed = wheel.simulated_angular_velocity * wheel.radius;
            assert!(
                spin_speed.is_finite() && (spin_speed - speed).abs() < 0.3 * speed.max(1.0) + 4.0,
                "{integration:?}: wheel {i}"
            );
        }
        assert!(
            log.queries.borrow().iter().any(|q| q.drive_torque_nm > 0.0),
            "{integration:?}: drive torque reaches the model"
        );
        drive(&mut car, &InputState { brake: 1.0, ..Default::default() }, 480);
        assert!(car.forward_speed() < 1.0, "{integration:?}: braking stops the car, speed {}", car.forward_speed());
        assert!(
            log.queries.borrow().iter().any(|q| q.brake_torque_nm > 0.0),
            "{integration:?}: brake torque reaches the model"
        );
    }
}

#[test]
fn the_answer_per_integration_mode_is_recorded() {
    // Both modes take a model. If a future change makes the reference mode refuse, this test says so first.
    assert!(crate::integration_takes_model(TireIntegration::Regularized));
    assert!(crate::integration_takes_model(TireIntegration::Reference));
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let (car, _log) = hooked(integration, Answer::Linear);
        assert!(car.tyre_model_applies(), "{integration:?}");
    }
}

#[test]
fn a_model_without_a_copy_is_not_cloned() {
    let (car, _log) = hooked(TireIntegration::Regularized, Answer::Linear);
    let copy = car.clone();
    assert!(car.has_tyre_model());
    assert!(!copy.has_tyre_model(), "a clone (a prediction, a replay) runs the built-in tyres");
}

#[test]
fn a_retired_model_is_dropped_before_the_next_step() {
    struct Retired;
    impl TyreModel for Retired {
        fn wheel(&mut self, _query: &TyreQuery) -> Option<TyreResult> {
            Some(fixed(5000.0, 0.0))
        }
        fn retired(&self) -> bool {
            true
        }
    }
    let mut plain = parked();
    let mut car = parked();
    car.set_tyre_model(Box::new(Retired), 0);
    drive(&mut plain, &throttle(), 30);
    drive(&mut car, &throttle(), 30);
    assert!(!car.has_tyre_model());
    assert_eq!(fingerprint(&plain), fingerprint(&car));
}
