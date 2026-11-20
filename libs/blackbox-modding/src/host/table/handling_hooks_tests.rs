//! The suspension (1.1) and controls (1.2) hooks of `bb.handling` called directly with a fake backend: the table, the kinds being
//! independent of one another, the adapters between the mod's callback and the vehicle's traits (the layout of both directions),
//! the fault policy of each, the retirement on unregister and unload, and a backend that implements only the tyre hook.

use core::ffi::c_void;
use core::mem::size_of;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use blackbox_vehicle::input::{ControlsQuery, ControlsReject};
use blackbox_vehicle::suspension::{SuspensionQuery, SuspensionReject, SuspensionWheelQuery};

use super::handling::HandlingBackend;
use super::handling_model::{ControlsHook, SuspensionHook, TyreHook};
use super::{Attached, BackendResult, attach_handling};
use crate::abi::*;
use crate::host::fault::QUARANTINE_AFTER;
use crate::host::test_support::callback_on_main;

fn table_of(attached: &Attached) -> &BbHandling {
    // SAFETY: the attachment owns a `BbHandling` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbHandling>() }
}

/// A backend that implements all three hooks and keeps what it was given.
#[derive(Default)]
pub(super) struct FakeHandling {
    tyre: RefCell<Vec<(u64, TyreHook)>>,
    pub(super) suspension: RefCell<Vec<(u64, SuspensionHook)>>,
    pub(super) controls: RefCell<Vec<(u64, ControlsHook)>>,
    removed: RefCell<Vec<String>>,
    next: Cell<u64>,
}

impl FakeHandling {
    fn handle(&self) -> u64 {
        self.next.set(self.next.get() + 1);
        self.next.get()
    }
}

fn take<H: Clone>(list: &RefCell<Vec<(u64, H)>>, handle: u64) -> Option<H> {
    let mut list = list.borrow_mut();
    let index = list.iter().position(|(known, _)| *known == handle)?;
    Some(list.remove(index).1)
}

impl HandlingBackend for FakeHandling {
    fn register_tyre(&self, _owner: &str, hook: TyreHook) -> BackendResult<u64> {
        let handle = self.handle();
        self.tyre.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_tyre(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let hook = take(&self.tyre, handle).ok_or(BB_ERR_STALE)?;
        hook.retire();
        self.removed.borrow_mut().push("tyre".to_owned());
        Ok(())
    }

    fn register_suspension(&self, _owner: &str, hook: SuspensionHook) -> BackendResult<u64> {
        let handle = self.handle();
        self.suspension.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_suspension(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let hook = take(&self.suspension, handle).ok_or(BB_ERR_STALE)?;
        hook.retire();
        self.removed.borrow_mut().push("suspension".to_owned());
        Ok(())
    }

    fn register_controls(&self, _owner: &str, hook: ControlsHook) -> BackendResult<u64> {
        let handle = self.handle();
        self.controls.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_controls(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let hook = take(&self.controls, handle).ok_or(BB_ERR_STALE)?;
        hook.retire();
        self.removed.borrow_mut().push("controls".to_owned());
        Ok(())
    }
}

/// A backend written for 1.0: it implements the tyre hook only and keeps the defaults of the rest.
struct TyreOnly;

impl HandlingBackend for TyreOnly {
    fn register_tyre(&self, _owner: &str, _hook: TyreHook) -> BackendResult<u64> {
        Ok(1)
    }

    fn unregister_tyre(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }
}

pub(super) fn fixture() -> (Rc<FakeHandling>, Attached) {
    let fake = Rc::new(FakeHandling::default());
    let attached = attach_handling(fake.clone(), "example.a");
    (fake, attached)
}

/// What a scripted callback answers and what it saw.
pub(super) struct Script<Q, R> {
    pub(super) status: Cell<BbStatus>,
    pub(super) result: Cell<R>,
    pub(super) calls: Cell<u32>,
    pub(super) last: Cell<Q>,
}

impl<Q: Copy + Default, R: Copy> Script<Q, R> {
    pub(super) fn new(status: BbStatus, result: R) -> Self {
        Script {
            status: Cell::new(status),
            result: Cell::new(result),
            calls: Cell::new(0),
            last: Cell::new(Q::default()),
        }
    }

    fn user(&self) -> *mut c_void {
        (self as *const Self).cast_mut().cast()
    }
}

unsafe extern "C" fn suspension_cb(
    user: *mut c_void,
    query: *const BbSuspensionQuery,
    result: *mut BbSuspensionResult,
) -> BbStatus {
    // SAFETY: the tests pass a `Script` that outlives the call, and the adapter a valid query and result.
    let script = unsafe { &*user.cast::<Script<BbSuspensionQuery, BbSuspensionResult>>() };
    script.calls.set(script.calls.get() + 1);
    unsafe {
        script.last.set(*query);
        let size = (*result).size;
        *result = script.result.get();
        (*result).size = size;
    }
    script.status.get()
}

unsafe extern "C" fn controls_cb(
    user: *mut c_void,
    query: *const BbControlsQuery,
    result: *mut BbControlsResult,
) -> BbStatus {
    // SAFETY: the tests pass a `Script` that outlives the call, and the adapter a valid query and result.
    let script = unsafe { &*user.cast::<Script<BbControlsQuery, BbControlsResult>>() };
    script.calls.set(script.calls.get() + 1);
    unsafe {
        script.last.set(*query);
        let size = (*result).size;
        *result = script.result.get();
        (*result).size = size;
    }
    script.status.get()
}

pub(super) type SuspensionScript = Script<BbSuspensionQuery, BbSuspensionResult>;
pub(super) type ControlsScript = Script<BbControlsQuery, BbControlsResult>;

pub(super) fn good_suspension() -> BbSuspensionResult {
    let left =
        BbSuspensionWheelResult { body_force_n: 3400.0, tyre_load_n: 3700.0, camber_rad: -0.03, reserved: [0; 2] };
    let right =
        BbSuspensionWheelResult { body_force_n: -200.0, tyre_load_n: 3900.0, camber_rad: -0.04, reserved: [0; 2] };
    BbSuspensionResult { wheels: [left, right], ..BbSuspensionResult::default() }
}

pub(super) fn good_controls() -> BbControlsResult {
    BbControlsResult {
        flags: BB_CONTROLS_FLAG_STEER,
        throttle: 0.4,
        brake: [0.1, 0.2, 0.3, 0.4],
        steer_target_rad: -0.25,
        ..BbControlsResult::default()
    }
}

pub(super) fn suspension_registration(script: &SuspensionScript) -> BbSuspensionHook {
    BbSuspensionHook {
        size: size_of::<BbSuspensionHook>() as u32,
        flags: 0,
        user: script.user(),
        callback: Some(suspension_cb),
        reserved: [0; 4],
    }
}

pub(super) fn controls_registration(script: &ControlsScript) -> BbControlsHook {
    BbControlsHook {
        size: size_of::<BbControlsHook>() as u32,
        flags: 0,
        user: script.user(),
        callback: Some(controls_cb),
        reserved: [0; 4],
    }
}

pub(super) fn register_suspension_hook(attached: &Attached, hook: &BbSuspensionHook) -> (BbStatus, u64) {
    let table = table_of(attached);
    let mut id = 0_u64;
    let register = table.register_suspension.expect("register_suspension is offered");
    (unsafe { register(table.ctx, hook, &mut id) }, id)
}

pub(super) fn register_controls_hook(attached: &Attached, hook: &BbControlsHook) -> (BbStatus, u64) {
    let table = table_of(attached);
    let mut id = 0_u64;
    let register = table.register_controls.expect("register_controls is offered");
    (unsafe { register(table.ctx, hook, &mut id) }, id)
}

pub(super) fn suspension_query() -> SuspensionQuery {
    let wheel = |loaded: bool, c: f32| SuspensionWheelQuery {
        loaded,
        compression_m: c,
        compression_speed_mps: 0.2,
        steer_angle_rad: 0.1,
        body_accel_up_mps2: -1.5,
        tyre_lateral_force_n: 900.0,
        static_load_n: 3700.0,
    };
    SuspensionQuery {
        axle: 1,
        role: 0,
        dt: 1.0 / 60.0,
        rest_travel_m: 0.09,
        max_travel_m: 0.15,
        corner_mass_kg: 375.0,
        wheels: [wheel(true, 0.07), wheel(false, 0.0)],
    }
}

pub(super) fn controls_query() -> ControlsQuery {
    ControlsQuery {
        role: 0,
        dt: 1.0 / 60.0,
        throttle: 0.9,
        brake: 0.0,
        handbrake: 0.2,
        steer: -0.5,
        forward_speed: 22.0,
        yaw_rate: 0.3,
        lateral_accel: 4.0,
        longitudinal_accel: -2.0,
        wheel_loaded: [true, false, true, true],
        slip_ratio: [0.01, 0.0, -0.3, 0.2],
        slip_angle: [0.02, 0.0, -0.01, 0.03],
    }
}

#[test]
fn the_table_is_one_two_sized_and_offers_all_six_functions() {
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    assert_eq!((table.head.major, table.head.minor), (1, 2));
    assert_eq!(table.head.size as usize, size_of::<BbHandling>());
    assert_eq!(table.head.size, 64);
    assert!(table.register_tyre.is_some() && table.unregister_tyre.is_some());
    assert!(table.register_suspension.is_some() && table.unregister_suspension.is_some());
    assert!(table.register_controls.is_some() && table.unregister_controls.is_some());
}

#[test]
fn each_kind_is_one_per_mod_and_independent_of_the_others() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let suspension = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    assert_eq!(register_suspension_hook(&attached, &suspension_registration(&suspension)).0, BB_OK);
    assert_eq!(
        register_controls_hook(&attached, &controls_registration(&controls)).0,
        BB_OK,
        "another kind is no conflict"
    );
    assert_eq!(register_suspension_hook(&attached, &suspension_registration(&suspension)).0, BB_ERR_EXISTS);
    assert_eq!(register_controls_hook(&attached, &controls_registration(&controls)).0, BB_ERR_EXISTS);
    assert_eq!((fake.suspension.borrow().len(), fake.controls.borrow().len(), fake.tyre.borrow().len()), (1, 1, 0));
}

#[test]
fn unregister_removes_only_its_own_kind_and_allows_another() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let suspension = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    let (_, suspension_id) = register_suspension_hook(&attached, &suspension_registration(&suspension));
    register_controls_hook(&attached, &controls_registration(&controls));
    let hook = fake.suspension.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    assert!(model.axle(&suspension_query()).is_some());
    let table = table_of(&attached);
    let unregister = table.unregister_suspension.expect("offered");
    assert_eq!(unsafe { unregister(table.ctx, suspension_id) }, BB_OK);
    assert!(model.retired());
    let calls = suspension.calls.get();
    assert!(model.axle(&suspension_query()).is_none());
    assert_eq!(suspension.calls.get(), calls, "a retired hook is not called");
    assert_eq!(unsafe { unregister(table.ctx, suspension_id) }, BB_ERR_STALE);
    assert_eq!(fake.controls.borrow().len(), 1, "the controls hook stays");
    assert_eq!(register_suspension_hook(&attached, &suspension_registration(&suspension)).0, BB_OK);
}

#[test]
fn a_negative_status_is_a_fault_and_the_fifth_quarantines_each_hook() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let suspension = SuspensionScript::new(BB_ERR_PANIC, good_suspension());
    let controls = ControlsScript::new(BB_ERR_PANIC, good_controls());
    register_suspension_hook(&attached, &suspension_registration(&suspension));
    register_controls_hook(&attached, &controls_registration(&controls));
    let suspension_hook = fake.suspension.borrow()[0].1.clone();
    let controls_hook = fake.controls.borrow()[0].1.clone();
    let mut suspension_model = suspension_hook.model(Rc::new(Cell::new(1)));
    let mut controls_model = controls_hook.model(Rc::new(Cell::new(1)));
    for fault in 1..=QUARANTINE_AFTER {
        assert!(suspension_model.axle(&suspension_query()).is_none());
        assert!(controls_model.filter(&controls_query()).is_none());
        assert_eq!((suspension_hook.stats().faults(), controls_hook.stats().faults()), (fault, fault));
    }
    assert!(suspension_model.retired() && controls_model.retired(), "the vehicle drops a quarantined model");
    let calls = (suspension.calls.get(), controls.calls.get());
    assert!(suspension_model.axle(&suspension_query()).is_none());
    assert!(controls_model.filter(&controls_query()).is_none());
    assert_eq!((suspension.calls.get(), controls.calls.get()), calls, "a quarantined hook is not called again");
}

#[test]
fn not_handled_is_not_a_fault_and_a_refused_answer_is() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let suspension = SuspensionScript::new(BB_SUSPENSION_NOT_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_NOT_HANDLED, good_controls());
    register_suspension_hook(&attached, &suspension_registration(&suspension));
    register_controls_hook(&attached, &controls_registration(&controls));
    let suspension_hook = fake.suspension.borrow()[0].1.clone();
    let controls_hook = fake.controls.borrow()[0].1.clone();
    let mut suspension_model = suspension_hook.model(Rc::new(Cell::new(1)));
    let mut controls_model = controls_hook.model(Rc::new(Cell::new(1)));
    for _ in 0..10 {
        assert!(suspension_model.axle(&suspension_query()).is_none());
        assert!(controls_model.filter(&controls_query()).is_none());
    }
    assert_eq!((suspension_hook.stats().faults(), controls_hook.stats().faults()), (0, 0));
    suspension_model.rejected(SuspensionReject::NotFinite);
    controls_model.rejected(ControlsReject::OutOfRange);
    assert_eq!((suspension_hook.stats().faults(), controls_hook.stats().faults()), (1, 1));
}

#[test]
fn registration_is_checked_for_both_new_kinds() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    let suspension = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    let mut id = 0_u64;
    let register_suspension = table.register_suspension.expect("offered");
    let register_controls = table.register_controls.expect("offered");

    let mut no_callback = suspension_registration(&suspension);
    no_callback.callback = None;
    assert_eq!(unsafe { register_suspension(table.ctx, &no_callback, &mut id) }, BB_ERR_ARG);
    let mut too_small = suspension_registration(&suspension);
    too_small.size = 8;
    assert_eq!(unsafe { register_suspension(table.ctx, &too_small, &mut id) }, BB_ERR_ARG);
    assert_eq!(unsafe { register_suspension(table.ctx, std::ptr::null(), &mut id) }, BB_ERR_ARG);
    assert_eq!(
        unsafe { register_suspension(table.ctx, &suspension_registration(&suspension), std::ptr::null_mut()) },
        BB_ERR_ARG
    );

    let mut no_callback = controls_registration(&controls);
    no_callback.callback = None;
    assert_eq!(unsafe { register_controls(table.ctx, &no_callback, &mut id) }, BB_ERR_ARG);
    let mut too_small = controls_registration(&controls);
    too_small.size = 8;
    assert_eq!(unsafe { register_controls(table.ctx, &too_small, &mut id) }, BB_ERR_ARG);
    assert_eq!(unsafe { register_controls(table.ctx, std::ptr::null(), &mut id) }, BB_ERR_ARG);
}

#[test]
fn unloading_the_mod_removes_and_retires_all_its_hooks() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let suspension = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    register_suspension_hook(&attached, &suspension_registration(&suspension));
    register_controls_hook(&attached, &controls_registration(&controls));
    let mut suspension_model = fake.suspension.borrow()[0].1.model(Rc::new(Cell::new(1)));
    let mut controls_model = fake.controls.borrow()[0].1.model(Rc::new(Cell::new(1)));
    drop(attached);
    let mut removed = fake.removed.borrow().clone();
    removed.sort();
    assert_eq!(removed, ["controls", "suspension"]);
    assert!(suspension_model.retired() && controls_model.retired());
    assert!(suspension_model.axle(&suspension_query()).is_none() && controls_model.filter(&controls_query()).is_none());
    assert_eq!((suspension.calls.get(), controls.calls.get()), (0, 0));
}

#[test]
fn a_backend_that_implements_only_the_tyre_hook_answers_unsupported_for_the_others() {
    let _scope = callback_on_main();
    let attached = attach_handling(Rc::new(TyreOnly), "example.a");
    let suspension = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let controls = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    assert_eq!(register_suspension_hook(&attached, &suspension_registration(&suspension)).0, BB_ERR_UNSUPPORTED);
    assert_eq!(register_controls_hook(&attached, &controls_registration(&controls)).0, BB_ERR_UNSUPPORTED);
    let table = table_of(&attached);
    let unregister = table.unregister_suspension.expect("offered");
    assert_eq!(unsafe { unregister(table.ctx, 1) }, BB_ERR_UNSUPPORTED);
}

#[test]
fn the_registry_offers_one_two_to_a_one_zero_mod_and_refuses_a_newer_minor() {
    let (fake, _attached) = fixture();
    let mut registry = crate::host::InterfaceRegistry::new();
    registry.add_handling(fake);
    assert!(registry.build("example.old", IFACE_HANDLING, 1, 0).is_some(), "a mod built for 1.0 still gets the table");
    assert!(registry.build("example.new", IFACE_HANDLING, 1, 2).is_some());
    assert!(registry.build("example.future", IFACE_HANDLING, 1, 3).is_none(), "a newer minor than the host is refused");
    assert!(registry.build("example.other", IFACE_HANDLING, 2, 0).is_none());
}

#[test]
fn a_one_zero_host_refuses_a_mod_that_needs_one_two_and_serves_one_that_needs_one_zero() {
    let mut registry = crate::host::InterfaceRegistry::new();
    registry.add_provider(IFACE_HANDLING, (1, 0), |_ctx| attach_handling(Rc::new(TyreOnly), "example.a"));
    assert!(registry.build("example.old", IFACE_HANDLING, 1, 0).is_some());
    assert!(registry.build("example.new", IFACE_HANDLING, 1, 1).is_none(), "1.1 needs the suspension pair");
    assert!(registry.build("example.new", IFACE_HANDLING, 1, 2).is_none(), "1.2 needs the controls pair");
}
