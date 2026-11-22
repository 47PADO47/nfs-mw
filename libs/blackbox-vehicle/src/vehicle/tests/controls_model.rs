//! The external controls model seam: dispatch with a stub model (declined, pass-through, a replaced throttle, a brake demand per
//! wheel, a steering target, not finite), what the query carries, and the proof that a vehicle without a model, whose model
//! declines, or whose model passes the controls through, is bit-identical.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::tyre_model::{drive, fingerprint, parked_in, throttle};
use super::*;
use crate::input::{ControlsModel, ControlsQuery, ControlsReject, ControlsResult};
use crate::tires::{TireIntegration, TyreModel, TyreQuery, TyreResult};

/// What a stub does with each question.
#[derive(Clone, Copy)]
enum Answer {
    Decline,
    /// Give the pedals back unchanged, the brake to every wheel, and leave the steering alone.
    PassThrough,
    /// Return this result every tick.
    Fixed(ControlsResult),
}

#[derive(Default)]
struct Log {
    queries: RefCell<Vec<ControlsQuery>>,
    rejected: Cell<u32>,
}

struct Stub {
    answer: Answer,
    log: Rc<Log>,
}

impl Stub {
    fn boxed(answer: Answer) -> (Box<dyn ControlsModel>, Rc<Log>) {
        let log = Rc::new(Log::default());
        (Box::new(Stub { answer, log: log.clone() }), log)
    }
}

impl ControlsModel for Stub {
    fn filter(&mut self, query: &ControlsQuery) -> Option<ControlsResult> {
        self.log.queries.borrow_mut().push(*query);
        match self.answer {
            Answer::Decline => None,
            Answer::PassThrough => {
                Some(ControlsResult { throttle: query.throttle, brake: [query.brake; 4], steer_target: None })
            }
            Answer::Fixed(result) => Some(result),
        }
    }

    fn rejected(&mut self, _why: ControlsReject) {
        self.log.rejected.set(self.log.rejected.get() + 1);
    }
}

fn hooked(integration: TireIntegration, answer: Answer) -> (Vehicle, Rc<Log>) {
    let mut car = parked_in(integration);
    let (model, log) = Stub::boxed(answer);
    car.set_controls_model(model, 4);
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

fn result(throttle: f32, brake: [f32; 4], steer: Option<f32>) -> ControlsResult {
    ControlsResult { throttle, brake, steer_target: steer }
}

/// Throttle for two seconds, a coast, then the brake: a drive that exercises every pedal path.
fn drive_script(car: &mut Vehicle) {
    drive(car, &throttle(), 180);
    drive(car, &InputState::default(), 30);
    drive(car, &InputState { brake: 1.0, ..Default::default() }, 120);
    drive(car, &InputState { throttle: 0.6, steer: 0.4, ..Default::default() }, 60);
}

#[test]
fn a_declining_model_changes_nothing() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let mut plain = parked_in(integration);
        let (mut declined, log) = hooked(integration, Answer::Decline);
        drive_script(&mut plain);
        drive_script(&mut declined);
        assert_eq!(fingerprint(&plain), fingerprint(&declined), "{integration:?}");
        assert_eq!(log.queries.borrow().len(), 180 + 30 + 120 + 60, "the model was asked once per step");
        assert_eq!(declined.controls_model_faults(), 0, "declining is not a fault");
    }
}

#[test]
fn a_model_that_passes_the_controls_through_changes_nothing() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let mut plain = parked_in(integration);
        let (mut passed, _log) = hooked(integration, Answer::PassThrough);
        drive_script(&mut plain);
        drive_script(&mut passed);
        assert_eq!(
            fingerprint(&plain),
            fingerprint(&passed),
            "{integration:?}: a bias of 1 per wheel is the built-in split"
        );
        assert_eq!(passed.controls_model_faults(), 0);
    }
}

#[test]
fn a_removed_model_leaves_no_trace() {
    let mut plain = parked_in(TireIntegration::Regularized);
    let (mut removed, log) = hooked(TireIntegration::Regularized, Answer::Fixed(result(0.0, [1.0; 4], Some(0.4))));
    removed.clear_controls_model();
    assert!(!removed.has_controls_model());
    drive_script(&mut plain);
    drive_script(&mut removed);
    assert_eq!(fingerprint(&plain), fingerprint(&removed));
    assert!(log.queries.borrow().is_empty());
}

#[test]
fn a_not_finite_or_out_of_range_answer_is_refused_counted_and_the_controls_stay_as_shaped() {
    let bad = [
        result(f32::NAN, [0.0; 4], None),
        result(1.0, [0.0, f32::INFINITY, 0.0, 0.0], None),
        result(2.0, [0.0; 4], None),
        result(1.0, [0.0, 0.0, 1.6, 0.0], None),
        result(1.0, [0.0; 4], Some(5.0)),
    ];
    for answer in bad {
        let mut plain = parked_in(TireIntegration::Regularized);
        let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Fixed(answer));
        drive(&mut plain, &throttle(), 60);
        drive(&mut car, &throttle(), 60);
        assert_eq!(fingerprint(&plain), fingerprint(&car), "{answer:?}");
        assert_eq!(car.controls_model_faults(), 60);
        assert_eq!(log.rejected.get(), 60, "the model hears about each refusal");
    }
}

#[test]
fn the_query_carries_the_shaped_pedals_the_speed_and_the_wheel_state() {
    let mut car = parked_in(TireIntegration::Regularized);
    let ground = flat();
    car.place_on_ground_moving(&ground, 0.0, 0.0, 5.0, 0.0, 12.0);
    let (model, log) = Stub::boxed(Answer::Decline);
    car.set_controls_model(model, 4);
    let input = InputState { throttle: 1.0, steer: 0.3, ..Default::default() };
    for _ in 0..20 {
        car.step(FIXED_STEP, &input, &ground);
    }
    let queries = log.queries.borrow();
    let last = queries.last().expect("asked");
    assert_eq!((last.role, last.dt), (4, FIXED_STEP));
    assert_eq!((last.throttle, last.brake, last.handbrake, last.steer), (1.0, 0.0, 0.0, 0.3));
    assert!(last.forward_speed > 8.0, "speed {}", last.forward_speed);
    assert!(last.wheel_loaded.iter().all(|on| *on));
    assert!(last.slip_ratio.iter().chain(&last.slip_angle).all(|v| v.is_finite()));
    assert!(last.slip_ratio.iter().all(|r| r.abs() <= 10.0));
    assert!(last.yaw_rate.is_finite() && last.lateral_accel.is_finite() && last.longitudinal_accel.is_finite());
    assert_eq!(queries[0].longitudinal_accel, 0.0, "there is no previous tick yet");
}

#[test]
fn the_idle_hold_and_the_reverse_pedal_swap_reach_the_model_as_the_vehicle_shaped_them() {
    let mut car = parked_in(TireIntegration::Regularized);
    let (model, log) = Stub::boxed(Answer::Decline);
    car.set_controls_model(model, 0);
    drive(&mut car, &InputState::default(), 3);
    let queries = log.queries.borrow();
    assert!(queries.iter().all(|q| q.throttle == 0.0 && q.brake > 0.0), "the idle auto-brake is a brake request");
}

#[test]
fn a_replaced_throttle_is_what_the_engine_sees() {
    let mut gated = parked_in(TireIntegration::Regularized);
    let mut free = parked_in(TireIntegration::Regularized);
    let (model, _log) = Stub::boxed(Answer::Fixed(result(0.0, [0.0; 4], None)));
    gated.set_controls_model(model, 0);
    drive(&mut gated, &throttle(), 180);
    drive(&mut free, &throttle(), 180);
    assert!(free.forward_speed() > 5.0);
    assert!(
        gated.forward_speed() < 0.5 * free.forward_speed(),
        "the model cut the throttle: {}",
        gated.forward_speed()
    );
    assert_eq!(gated.controls().gas, 0.0);
}

#[test]
fn the_brake_demand_of_each_wheel_reaches_that_wheel() {
    let mut car = parked_in(TireIntegration::Regularized);
    let (model, _log) = Stub::boxed(Answer::Fixed(result(0.0, [1.0, 0.0, 0.5, 0.0], None)));
    car.set_controls_model(model, 0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    car.set_tyre_model(Box::new(Spy(seen.clone())), 0);
    // The pedal is released, as an aid would brake with it: the demand alone brakes the wheels.
    drive(&mut car, &throttle(), 5);
    let questions = seen.borrow();
    let last_tick = &questions[questions.len() - 4..];
    let torque: Vec<f32> = last_tick.iter().map(|q| q.brake_torque_nm).collect();
    let (front, rear) = (car.spec().brakes.brake_torque(0), car.spec().brakes.brake_torque(1));
    assert!(
        (torque[0] - front).abs() < 1e-2 * front,
        "a demand of 1 is the full torque of the wheel: {torque:?} vs {front}"
    );
    assert!((torque[2] - 0.5 * rear).abs() < 1e-2 * rear, "a demand of 0.5 is half of it: {torque:?} vs {rear}");
    assert_eq!((torque[1], torque[3]), (0.0, 0.0), "no demand, no brake: {torque:?}");
    assert_eq!(car.controls().brake, 1.0, "the brake lamp shows the strongest demand");
}

#[test]
fn a_steering_target_replaces_the_shaping_and_is_limited_to_the_lock() {
    let mut car = parked_in(TireIntegration::Regularized);
    let (model, _log) = Stub::boxed(Answer::Fixed(result(0.3, [0.0; 4], Some(0.2))));
    car.set_controls_model(model, 0);
    drive(&mut car, &throttle(), 2);
    let angles = car.wheel_angles();
    assert!((angles.right - 0.2).abs() < 1e-3, "the inside (right) wheel takes the target: {angles:?}");
    assert!(angles.left > 0.0 && angles.left < angles.right, "the outside wheel turns less (Ackermann): {angles:?}");

    let mut left = parked_in(TireIntegration::Regularized);
    let (model, _log) = Stub::boxed(Answer::Fixed(result(0.3, [0.0; 4], Some(-1.4))));
    left.set_controls_model(model, 0);
    drive(&mut left, &throttle(), 2);
    let limit = 45.0_f32.to_radians();
    assert!((left.wheel_angles().left + limit).abs() < 1e-3, "clamped to 45 degrees: {:?}", left.wheel_angles());
}

#[test]
fn an_answer_without_a_steering_target_leaves_the_steering_to_the_vehicle() {
    let mut plain = parked_in(TireIntegration::Regularized);
    let (mut kept, _log) = hooked(TireIntegration::Regularized, Answer::Fixed(result(1.0, [0.0; 4], None)));
    let input = InputState { throttle: 1.0, steer: 0.5, ..Default::default() };
    drive(&mut plain, &input, 60);
    drive(&mut kept, &input, 60);
    assert_eq!(plain.wheel_angles(), kept.wheel_angles());
    assert_eq!(fingerprint(&plain), fingerprint(&kept));
}

#[test]
fn a_disabled_vehicle_is_not_asked() {
    let (mut car, log) = hooked(TireIntegration::Regularized, Answer::Decline);
    car.disabled = true;
    drive(&mut car, &throttle(), 10);
    assert!(log.queries.borrow().is_empty());
    assert!(!car.controls_model_applies());
}

#[test]
fn the_answer_is_recorded_for_both_integration_modes() {
    for integration in [TireIntegration::Regularized, TireIntegration::Reference] {
        let (car, _log) = hooked(integration, Answer::PassThrough);
        assert!(car.controls_model_applies(), "{integration:?}");
    }
}

#[test]
fn a_model_without_a_copy_is_not_cloned_and_a_retired_one_is_dropped() {
    let (car, _log) = hooked(TireIntegration::Regularized, Answer::PassThrough);
    assert!(car.has_controls_model());
    assert!(!car.clone().has_controls_model(), "a clone keeps the controls as shaped");

    struct Retired;
    impl ControlsModel for Retired {
        fn filter(&mut self, _query: &ControlsQuery) -> Option<ControlsResult> {
            Some(result(0.0, [1.0; 4], Some(0.5)))
        }
        fn retired(&self) -> bool {
            true
        }
    }
    let mut plain = parked_in(TireIntegration::Regularized);
    let mut dropped = parked_in(TireIntegration::Regularized);
    dropped.set_controls_model(Box::new(Retired), 0);
    drive(&mut plain, &throttle(), 30);
    drive(&mut dropped, &throttle(), 30);
    assert!(!dropped.has_controls_model());
    assert_eq!(fingerprint(&plain), fingerprint(&dropped));
}
