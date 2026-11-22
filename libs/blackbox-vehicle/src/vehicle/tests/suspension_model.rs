//! The external suspension model seam: dispatch with a stub model (declined, not finite, a spring of its own, airborne), the camber
//! and the tyre load reaching the tyre model of the same wheel, and the proof that a vehicle without a model, or whose model
//! declines, is bit-identical.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::tyre_model::{drive, fingerprint, parked_in, throttle};
use super::*;
use crate::ground::NoGround;
use crate::suspension::{SuspensionModel, SuspensionQuery, SuspensionReject, SuspensionResult, SuspensionWheelResult};
use crate::tires::{TireIntegration, TyreModel, TyreQuery, TyreResult};

/// What a stub does with each axle.
#[derive(Clone, Copy)]
enum Answer {
    Decline,
    /// Return this result for every axle.
    Fixed(SuspensionResult),
    /// Carry the static weight on the spring (what a spring that sits at its rest compression does).
    Static,
}

#[derive(Default)]
struct Log {
    queries: RefCell<Vec<SuspensionQuery>>,
    rejected: Cell<u32>,
}

struct Stub {
    answer: Answer,
    log: Rc<Log>,
}

impl Stub {
    fn boxed(answer: Answer) -> (Box<dyn SuspensionModel>, Rc<Log>) {
        let log = Rc::new(Log::default());
        (Box::new(Stub { answer, log: log.clone() }), log)
    }
}

impl SuspensionModel for Stub {
    fn axle(&mut self, query: &SuspensionQuery) -> Option<SuspensionResult> {
        self.log.queries.borrow_mut().push(*query);
        match self.answer {
            Answer::Decline => None,
            Answer::Fixed(result) => Some(result),
            Answer::Static => {
                let wheel = |side: usize| SuspensionWheelResult {
                    body_force_n: query.wheels[side].static_load_n,
                    tyre_load_n: query.wheels[side].static_load_n,
                    camber_rad: -0.03,
                };
                Some(SuspensionResult { wheels: [wheel(0), wheel(1)] })
            }
        }
    }

    fn rejected(&mut self, _why: SuspensionReject) {
        self.log.rejected.set(self.log.rejected.get() + 1);
    }
}

fn wheel(body: f32, load: f32, camber: f32) -> SuspensionWheelResult {
    SuspensionWheelResult { body_force_n: body, tyre_load_n: load, camber_rad: camber }
}

fn fixed(body: f32, load: f32, camber: f32) -> SuspensionResult {
    SuspensionResult { wheels: [wheel(body, load, camber); 2] }
}

fn hooked(integration: TireIntegration, answer: Answer) -> (Vehicle, Rc<Log>) {
    let mut car = parked_in(integration);
    let (model, log) = Stub::boxed(answer);
    car.set_suspension_model(model, 9);
    (car, log)
}

/// A tyre stub that declines and remembers every question.
struct Spy(Rc<RefCell<Vec<TyreQuery>>>);

impl TyreModel for Spy {
    fn wheel(&mut self, query: &TyreQuery) -> Option<TyreResult> {
        self.0.borrow_mut().push(*query);
        None
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
        assert!(log.queries.borrow().len() >= 2 * 240, "the model was asked for both axles of every step");
        assert_eq!(declined.suspension_model_faults(), 0, "declining is not a fault");
    }
}

#[test]
fn a_removed_model_leaves_no_trace() {
    let mut plain = parked_in(TireIntegration::Regularized);
    let (mut removed, log) = hooked(TireIntegration::Regularized, Answer::Static);
    removed.clear_suspension_model();
    assert!(!removed.has_suspension_model());
    drive(&mut plain, &throttle(), 120);
    drive(&mut removed, &throttle(), 120);
    assert_eq!(fingerprint(&plain), fingerprint(&removed));
    assert!(log.queries.borrow().is_empty());
    assert_eq!(removed.wheel_camber(0), 0.0);
}

#[test]
fn a_not_finite_or_out_of_range_answer_is_refused_counted_and_the_built_in_spring_runs() {
    for bad in [
        fixed(f32::NAN, 1000.0, 0.0),
        fixed(1000.0, f32::INFINITY, 0.0),
        fixed(1000.0, -5.0, 0.0),
        fixed(1000.0, 1000.0, 2.0),
    ] {
        let mut plain = parked_in(TireIntegration::Regularized);
        let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Fixed(bad));
        drive(&mut plain, &throttle(), 60);
        drive(&mut car, &throttle(), 60);
        assert_eq!(fingerprint(&plain), fingerprint(&car), "a refused answer leaves the built-in result");
        assert!(car.suspension_model_faults() >= 2 * 60, "faults {}", car.suspension_model_faults());
        assert_eq!(log.rejected.get(), car.suspension_model_faults(), "the model hears about each refusal");
    }
}

#[test]
fn the_axles_are_asked_front_then_rear_with_what_the_vehicle_measured() {
    let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Decline);
    drive(&mut car, &throttle(), 2);
    let queries = log.queries.borrow();
    let axles: Vec<u32> = queries.iter().take(4).map(|q| q.axle).collect();
    assert_eq!(axles, [0, 1, 0, 1], "front before rear, every tick");
    let weight: f32 = queries.iter().take(2).flat_map(|q| q.wheels.iter()).map(|w| w.static_load_n).sum();
    assert!((weight - car.spec().mass * 9.81).abs() < 1.0, "the four static loads add up to the weight: {weight}");
    for q in queries.iter() {
        assert_eq!((q.role, q.dt), (9, FIXED_STEP));
        assert!(q.rest_travel_m > 0.01 && q.max_travel_m > q.rest_travel_m, "{q:?}");
        for w in &q.wheels {
            let sag = (w.compression_m - q.rest_travel_m).abs();
            assert!(
                sag < 0.1 * q.rest_travel_m,
                "a settled car reads its rest compression: {} vs {}",
                w.compression_m,
                q.rest_travel_m
            );
        }
        assert!((q.corner_mass_kg - 0.25 * car.spec().mass).abs() < 1e-3);
        for w in &q.wheels {
            assert!(w.loaded && w.compression_m > 0.0 && w.compression_m <= q.max_travel_m, "{w:?}");
        }
    }
}

#[test]
fn the_acceleration_of_the_body_is_minus_g_in_free_fall_and_zero_on_the_first_tick() {
    let mut car = parked();
    let (model, log) = Stub::boxed(Answer::Decline);
    car.set_suspension_model(model, 0);
    for _ in 0..12 {
        car.step(FIXED_STEP, &throttle(), &NoGround);
    }
    let queries = log.queries.borrow();
    assert!(queries[0].wheels.iter().all(|w| w.body_accel_up_mps2 == 0.0), "there is no previous tick yet");
    let last = queries.last().expect("asked");
    for w in &last.wheels {
        assert!(!w.loaded && w.compression_m == 0.0, "an airborne wheel is asked too");
        assert!((w.body_accel_up_mps2 + 9.81).abs() < 1.0, "free fall: {}", w.body_accel_up_mps2);
    }
}

#[test]
fn the_camber_and_the_tyre_load_of_a_handled_wheel_reach_its_tyre_model() {
    let mut car = parked();
    let (model, _log) = Stub::boxed(Answer::Fixed(fixed(3500.0, 4321.0, -0.05)));
    car.set_suspension_model(model, 0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    car.set_tyre_model(Box::new(Spy(seen.clone())), 0);
    drive(&mut car, &throttle(), 3);
    let questions = seen.borrow();
    assert!(!questions.is_empty());
    for q in questions.iter() {
        assert_eq!(q.camber_rad, -0.05, "wheel {}", q.wheel_index);
        assert_eq!(q.vertical_load_n, 4321.0, "the tyre load replaces the load the host derives from its own spring");
    }
    assert_eq!(car.wheel_camber(2), -0.05);
}

#[test]
fn a_declined_axle_gives_its_tyres_no_camber() {
    let mut car = parked();
    let (model, _log) = Stub::boxed(Answer::Decline);
    car.set_suspension_model(model, 0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    car.set_tyre_model(Box::new(Spy(seen.clone())), 0);
    drive(&mut car, &throttle(), 3);
    assert!(!seen.borrow().is_empty());
    assert!(seen.borrow().iter().all(|q| q.camber_rad == 0.0));
}

#[test]
fn the_force_of_a_handled_wheel_reaches_the_body() {
    let mut plain = parked();
    let (mut lifted, _log) = hooked(TireIntegration::Regularized, Answer::Static);
    let (mut shoved, _log) = hooked(TireIntegration::Regularized, Answer::Fixed(fixed(9000.0, 3500.0, 0.0)));
    drive(&mut plain, &throttle(), 20);
    drive(&mut lifted, &throttle(), 20);
    drive(&mut shoved, &throttle(), 20);
    assert!(
        shoved.position().y > plain.position().y + 0.01,
        "4 x 9000 N lift the body: {} vs {}",
        shoved.position().y,
        plain.position().y
    );
    assert!(
        lifted.position().y.is_finite() && up(&lifted).y > 0.99,
        "a model that carries the weight keeps the car upright"
    );
    assert_eq!(lifted.suspension_model_faults(), 0);
}

#[test]
fn the_answer_is_recorded_for_both_integration_modes() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let (car, _log) = hooked(integration, Answer::Static);
        assert!(car.suspension_model_applies(), "{integration:?}");
    }
}

#[test]
fn a_model_without_a_copy_is_not_cloned_and_a_retired_one_is_dropped() {
    let (car, _log) = hooked(TireIntegration::Regularized, Answer::Static);
    assert!(car.has_suspension_model());
    assert!(!car.clone().has_suspension_model(), "a clone runs the built-in springs");

    struct Retired;
    impl SuspensionModel for Retired {
        fn axle(&mut self, _query: &SuspensionQuery) -> Option<SuspensionResult> {
            Some(fixed(5000.0, 5000.0, 0.0))
        }
        fn retired(&self) -> bool {
            true
        }
    }
    let mut plain = parked();
    let mut dropped = parked();
    dropped.set_suspension_model(Box::new(Retired), 0);
    drive(&mut plain, &throttle(), 30);
    drive(&mut dropped, &throttle(), 30);
    assert!(!dropped.has_suspension_model());
    assert_eq!(fingerprint(&plain), fingerprint(&dropped));
}
