//! `bb.handling` called directly with a fake backend: registration and its refusals, the adapter between the mod's callback and
//! the vehicle's `TyreModel` (handled, not handled, a fault, the quarantine after five), the retirement on unregister and
//! unload, and the budget count.

use core::ffi::c_void;
use core::mem::size_of;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use blackbox_vehicle::tires::{TyreQuery, TyreResult};

use super::handling::HandlingBackend;
use super::handling_model::TyreHook;
use super::{Attached, BackendResult, attach_handling};
use crate::abi::*;
use crate::host::fault::QUARANTINE_AFTER;
use crate::host::test_support::callback_on_main;

fn table_of(attached: &Attached) -> &BbHandling {
    // SAFETY: the attachment owns a `BbHandling` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbHandling>() }
}

/// A backend that keeps the hook it was given, and can refuse (an online session).
#[derive(Default)]
struct FakeHandling {
    hooks: RefCell<Vec<(u64, TyreHook)>>,
    refuse: Cell<bool>,
    removed: RefCell<Vec<u64>>,
}

impl HandlingBackend for FakeHandling {
    fn register_tyre(&self, _owner: &str, hook: TyreHook) -> BackendResult<u64> {
        if self.refuse.get() {
            return Err(BB_ERR_DENIED);
        }
        let handle = self.hooks.borrow().len() as u64 + 1;
        self.hooks.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_tyre(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let mut hooks = self.hooks.borrow_mut();
        let Some(index) = hooks.iter().position(|(known, _)| *known == handle) else {
            return Err(BB_ERR_STALE);
        };
        let (_, hook) = hooks.remove(index);
        hook.retire();
        self.removed.borrow_mut().push(handle);
        Ok(())
    }
}

/// What the test callback does, and what it saw.
struct Script {
    status: Cell<BbStatus>,
    result: Cell<BbTyreResult>,
    calls: Cell<u32>,
    last: Cell<BbTyreQuery>,
    sleep: Cell<Duration>,
}

impl Script {
    fn handled(result: BbTyreResult) -> Script {
        Script {
            status: Cell::new(BB_TYRE_HANDLED),
            result: Cell::new(result),
            calls: Cell::new(0),
            last: Cell::new(BbTyreQuery::default()),
            sleep: Cell::new(Duration::ZERO),
        }
    }

    fn user(&self) -> *mut c_void {
        (self as *const Script).cast_mut().cast()
    }
}

unsafe extern "C" fn scripted(user: *mut c_void, query: *const BbTyreQuery, result: *mut BbTyreResult) -> BbStatus {
    // SAFETY: the tests pass a `Script` that outlives the call.
    let script = unsafe { &*user.cast::<Script>() };
    script.calls.set(script.calls.get() + 1);
    // SAFETY: the adapter passes a valid query and result.
    unsafe {
        script.last.set(*query);
        let size = (*result).size;
        *result = script.result.get();
        (*result).size = size;
    }
    std::thread::sleep(script.sleep.get());
    script.status.get()
}

fn good_result() -> BbTyreResult {
    BbTyreResult {
        longitudinal_force_n: 1200.0,
        lateral_force_n: -800.0,
        aligning_torque_nm: 15.0,
        wheel_spin: 42.0,
        traction: 0.9,
        slip: 0.25,
        slip_angle: 0.02,
        ..BbTyreResult::default()
    }
}

fn registration(script: &Script) -> BbTyreHook {
    BbTyreHook {
        size: size_of::<BbTyreHook>() as u32,
        flags: 0,
        user: script.user(),
        callback: Some(scripted),
        reserved: [0; 4],
    }
}

fn register(attached: &Attached, hook: &BbTyreHook) -> (BbStatus, u64) {
    let table = table_of(attached);
    let mut id = 0_u64;
    let register = table.register_tyre.expect("register_tyre is offered");
    (unsafe { register(table.ctx, hook, &mut id) }, id)
}

fn fixture() -> (Rc<FakeHandling>, Attached) {
    let fake = Rc::new(FakeHandling::default());
    let attached = attach_handling(fake.clone(), "example.a");
    (fake, attached)
}

fn query(wheel: u32) -> TyreQuery {
    TyreQuery {
        wheel_index: wheel,
        role: 0,
        loaded: true,
        dt: 1.0 / 60.0,
        vertical_load_n: 3500.0,
        patch_lateral_velocity: -0.5,
        patch_forward_velocity: 20.0,
        body_speed: 20.5,
        wheel_spin: 60.0,
        drive_torque_nm: 300.0,
        brake_torque_nm: 0.0,
        wheel_radius_m: 0.33,
        wheel_inertia_kgm2: 10.0,
        corner_mass_kg: 375.0,
        surface_lateral: 1.0,
        surface_drive: 1.0,
        surface_rolling: 1.0,
        surface_tag: 5,
        tyre_condition: 1.0,
        ambient_temperature_c: 20.0,
        road_temperature_c: 25.0,
        camber_rad: -0.04,
    }
}

#[test]
fn a_registered_hook_reaches_the_backend_and_answers() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    let (status, id) = register(&attached, &registration(&script));
    assert_eq!(status, BB_OK);
    assert_eq!(id, 1);
    let hook = fake.hooks.borrow()[0].1.clone();
    assert_eq!(hook.owner(), "example.a");
    let mut model = hook.model(Rc::new(Cell::new(9)));
    let answer = model.wheel(&query(2)).expect("handled");
    assert_eq!(answer.longitudinal_force_n, 1200.0);
    assert_eq!(answer.lateral_force_n, -800.0);
    assert_eq!(answer.aligning_torque_nm, 15.0);
    assert_eq!(answer.wheel_spin, 42.0);
    assert_eq!((answer.traction, answer.slip, answer.slip_angle), (0.9, 0.25, 0.02));
    let seen = script.last.get();
    assert_eq!(seen.size as usize, size_of::<BbTyreQuery>());
    assert_eq!((seen.wheel_index, seen.loaded, seen.vehicle_revision, seen.surface_tag), (2, 1, 9, 5));
    assert_eq!((seen.vertical_load_n, seen.wheel_spin, seen.wheel_radius_m), (3500.0, 60.0, 0.33));
    assert_eq!((seen.drive_torque_nm, seen.patch_lateral_velocity), (300.0, -0.5));
    assert_eq!(seen.camber_rad, -0.04, "the camber the suspension hook answered reaches the tyre hook");
    assert_eq!(hook.stats().calls(), 1);
    assert_eq!(hook.stats().faults(), 0);
}

#[test]
fn not_handled_is_not_a_fault() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    script.status.set(BB_TYRE_NOT_HANDLED);
    register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    for _ in 0..10 {
        assert!(model.wheel(&query(0)).is_none());
    }
    assert_eq!(hook.stats().faults(), 0);
    assert!(!model.retired());
    assert_eq!(script.calls.get(), 10);
}

#[test]
fn a_negative_status_is_a_fault_and_the_fifth_quarantines_the_hook() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    script.status.set(BB_ERR_PANIC);
    register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    for fault in 1..=QUARANTINE_AFTER {
        assert!(model.wheel(&query(0)).is_none());
        assert_eq!(hook.stats().faults(), fault);
    }
    assert!(hook.stats().quarantined());
    assert!(model.retired(), "the vehicle drops a quarantined model");
    let calls = script.calls.get();
    assert!(model.wheel(&query(0)).is_none());
    assert_eq!(script.calls.get(), calls, "a quarantined hook is not called again");
}

#[test]
fn a_result_the_vehicle_refuses_counts_as_a_fault_too() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    register(&attached, &registration(&Script::handled(good_result())));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    model.rejected(blackbox_vehicle::tires::TyreReject::NotFinite);
    model.rejected(blackbox_vehicle::tires::TyreReject::OutOfRange);
    assert_eq!(hook.stats().faults(), 2);
}

#[test]
fn a_slow_callback_is_counted_against_the_budget() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    script.sleep.set(Duration::from_millis(4));
    register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    let _ = model.wheel(&query(0));
    assert_eq!(hook.stats().over_budget(), 1);
    assert_eq!(hook.stats().faults(), 0, "slow is a warning, not a fault");
}

#[test]
fn registration_is_checked() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let script = Script::handled(good_result());
    let table = table_of(&attached);
    let register_fn = table.register_tyre.expect("offered");
    let mut id = 0_u64;

    let mut no_callback = registration(&script);
    no_callback.callback = None;
    assert_eq!(unsafe { register_fn(table.ctx, &no_callback, &mut id) }, BB_ERR_ARG);

    let mut too_small = registration(&script);
    too_small.size = 8;
    assert_eq!(unsafe { register_fn(table.ctx, &too_small, &mut id) }, BB_ERR_ARG);

    assert_eq!(unsafe { register_fn(table.ctx, std::ptr::null(), &mut id) }, BB_ERR_ARG);
    assert_eq!(unsafe { register_fn(table.ctx, &registration(&script), std::ptr::null_mut()) }, BB_ERR_ARG);

    assert_eq!(unsafe { register_fn(table.ctx, &registration(&script), &mut id) }, BB_OK);
    assert_eq!(unsafe { register_fn(table.ctx, &registration(&script), &mut id) }, BB_ERR_EXISTS, "one hook per mod");
}

#[test]
fn a_smaller_registration_from_an_older_layout_is_accepted() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    let mut older = registration(&script);
    older.size = 24;
    older.reserved = [7; 4];
    let (status, _) = register(&attached, &older);
    assert_eq!(status, BB_OK, "the callback ends at byte 24, which covers what the host needs");
    assert_eq!(fake.hooks.borrow().len(), 1);
}

#[test]
fn the_backend_may_refuse() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    fake.refuse.set(true);
    let script = Script::handled(good_result());
    let (status, _) = register(&attached, &registration(&script));
    assert_eq!(status, BB_ERR_DENIED);
    assert!(fake.hooks.borrow().is_empty());
    fake.refuse.set(false);
    assert_eq!(register(&attached, &registration(&script)).0, BB_OK, "a refused registration leaves no hook behind");
}

#[test]
fn unregister_retires_the_hook_and_allows_another() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    let (_, id) = register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    assert!(model.wheel(&query(0)).is_some());
    let table = table_of(&attached);
    let unregister = table.unregister_tyre.expect("offered");
    assert_eq!(unsafe { unregister(table.ctx, id) }, BB_OK);
    assert!(model.retired());
    let calls = script.calls.get();
    assert!(model.wheel(&query(0)).is_none());
    assert_eq!(script.calls.get(), calls, "a retired hook is not called");
    assert_eq!(unsafe { unregister(table.ctx, id) }, BB_ERR_STALE);
    assert_eq!(register(&attached, &registration(&script)).0, BB_OK);
}

#[test]
fn unloading_the_mod_removes_and_retires_the_hook() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    drop(attached);
    assert_eq!(*fake.removed.borrow(), vec![1]);
    assert!(fake.hooks.borrow().is_empty());
    assert!(model.retired());
    assert!(model.wheel(&query(0)).is_none());
    assert_eq!(script.calls.get(), 0);
}

#[test]
fn the_table_refuses_a_call_outside_a_callback() {
    crate::host::gate::mark_main_thread();
    let (_fake, attached) = fixture();
    let script = Script::handled(good_result());
    assert_eq!(register(&attached, &registration(&script)).0, BB_ERR_NOT_IN_CALLBACK);
}

#[test]
fn a_clone_of_the_model_calls_the_same_hook() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = Script::handled(good_result());
    register(&attached, &registration(&script));
    let hook = fake.hooks.borrow()[0].1.clone();
    let revision = Rc::new(Cell::new(3));
    let model = hook.model(revision.clone());
    let mut copy = model.duplicate().expect("the adapter duplicates itself");
    assert!(copy.wheel(&query(1)).is_some());
    assert_eq!(script.last.get().vehicle_revision, 3);
    revision.set(4);
    assert!(copy.wheel(&query(1)).is_some());
    assert_eq!(script.last.get().vehicle_revision, 4, "the revision is read at each call");
    let result: TyreResult = copy.wheel(&query(1)).expect("handled");
    assert_eq!(result.wheel_spin, 42.0);
}
