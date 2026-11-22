//! The SDK's `Handling` wrapper, round-tripped through the host's `bb.handling` table: a closure registered through each of the
//! three `register_*` calls is called by the host's adapter with the query the vehicle built and its answer comes back, a panic
//! in the closure becomes a fault, and a table of an older minor (1.0, 32 bytes) offers no suspension or controls hook, so the
//! SDK answers `Error::Unsupported` instead of reading past it.

use core::mem::size_of;
use core::ptr::NonNull;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use blackbox_vehicle::input::ControlsQuery as VehicleControlsQuery;
use blackbox_vehicle::suspension::{SuspensionQuery as VehicleSuspensionQuery, SuspensionWheelQuery};
use blackbox_vehicle::tires::TyreQuery as VehicleTyreQuery;

use super::HostTable;
use super::handling as sdk_handling;
use crate::abi::{
    ABI_MAJOR, ABI_MINOR, BB_ERR_PANIC, BbHandling, BbHandlingRegisterTyreFn, BbHost, BbIface, BbStr, IFACE_HANDLING,
    IFACE_HANDLING_VERSION,
};
use crate::host::test_support::callback_on_main;
use crate::host::{
    Attached, BackendResult, ControlsHook, HandlingBackend, InterfaceRegistry, SuspensionHook, TyreHook,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{
    ControlsQuery, ControlsResult, Error, Handling, Interface, Role, SuspensionQuery, SuspensionResult,
    SuspensionWheelResult, TyreQuery, TyreResult,
};

#[derive(Default)]
struct FakeBackend {
    tyre: RefCell<Option<TyreHook>>,
    suspension: RefCell<Option<SuspensionHook>>,
    controls: RefCell<Option<ControlsHook>>,
}

impl HandlingBackend for FakeBackend {
    fn register_tyre(&self, _owner: &str, hook: TyreHook) -> BackendResult<u64> {
        *self.tyre.borrow_mut() = Some(hook);
        Ok(1)
    }

    fn unregister_tyre(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        if let Some(hook) = self.tyre.borrow_mut().take() {
            hook.retire();
        }
        Ok(())
    }

    fn register_suspension(&self, _owner: &str, hook: SuspensionHook) -> BackendResult<u64> {
        *self.suspension.borrow_mut() = Some(hook);
        Ok(2)
    }

    fn unregister_suspension(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        if let Some(hook) = self.suspension.borrow_mut().take() {
            hook.retire();
        }
        Ok(())
    }

    fn register_controls(&self, _owner: &str, hook: ControlsHook) -> BackendResult<u64> {
        *self.controls.borrow_mut() = Some(hook);
        Ok(3)
    }

    fn unregister_controls(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        if let Some(hook) = self.controls.borrow_mut().take() {
            hook.retire();
        }
        Ok(())
    }
}

fn fake_host() -> BbHost {
    BbHost {
        size: size_of::<BbHost>() as u32,
        abi_major: ABI_MAJOR,
        abi_minor: ABI_MINOR,
        ctx: core::ptr::null_mut(),
        game_id: BbStr::new("test"),
        game_version: BbStr::new("0"),
        log: None,
        query_interface: None,
        mod_dir: BbStr::new(""),
        game_dir: BbStr::new(""),
    }
}

fn interface_of(attached: &Attached) -> Interface {
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let value = unsafe { *header };
    Interface::new(NonNull::new(header.cast_mut()).expect("non-null header"), value)
}

/// The SDK's `Handling` over a registry that offers `backend` as `bb.handling`.
fn handling_over(backend: &Rc<FakeBackend>, host_struct: &mut BbHost) -> (Handling, Attached) {
    let mut registry = InterfaceRegistry::new();
    registry.add_handling(backend.clone());
    let attached = registry
        .build("example.handling", IFACE_HANDLING, IFACE_HANDLING_VERSION.0, 0)
        .expect("bb.handling is offered");
    // SAFETY: `host_struct` outlives the wrapper, which the test uses only while it is in scope.
    let host = unsafe { HostTable::new(NonNull::from(host_struct)) };
    let keep: Keep = Rc::new(RefCell::new(Vec::new()));
    (Handling::new(sdk_handling::read(interface_of(&attached)), host, keep), attached)
}

#[test]
fn a_suspension_closure_gets_the_axle_and_its_answer_reaches_the_vehicle() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (handling, _attached) = handling_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::<SuspensionQuery>::new()));
    let log = seen.clone();
    handling
        .register_suspension(move |_ctx, query| {
            log.borrow_mut().push(*query);
            let wheel = |force: f32| SuspensionWheelResult {
                body_force_n: force,
                tyre_load_n: force + 300.0,
                camber_rad: -0.03,
            };
            Some(SuspensionResult { wheels: [wheel(3000.0), wheel(3100.0)] })
        })
        .expect("registered");
    let hook = backend.suspension.borrow().clone().expect("the host holds the hook");
    let mut model = hook.model(Rc::new(Cell::new(5)));
    let wheel = |loaded: bool, c: f32| SuspensionWheelQuery {
        loaded,
        compression_m: c,
        compression_speed_mps: -0.1,
        steer_angle_rad: 0.2,
        body_accel_up_mps2: -9.8,
        tyre_lateral_force_n: 120.0,
        static_load_n: 3500.0,
    };
    let query = VehicleSuspensionQuery {
        axle: 0,
        role: 0,
        dt: 1.0 / 144.0,
        rest_travel_m: 0.1,
        max_travel_m: 0.16,
        corner_mass_kg: 380.0,
        wheels: [wheel(true, 0.08), wheel(true, 0.09)],
    };
    let answer = model.axle(&query).expect("handled");
    assert_eq!((answer.wheels[0].body_force_n, answer.wheels[1].tyre_load_n), (3000.0, 3400.0));
    assert_eq!(answer.wheels[0].camber_rad, -0.03);
    let first = seen.borrow()[0];
    assert_eq!((first.axle, first.role, first.vehicle_revision), (0, Some(Role::Player), 5));
    assert_eq!(
        (first.dt, first.rest_travel_m, first.max_travel_m, first.corner_mass_kg),
        (1.0 / 144.0, 0.1, 0.16, 380.0)
    );
    let expected = crate::sdk::SuspensionWheelQuery {
        loaded: true,
        compression_m: 0.08,
        compression_speed_mps: -0.1,
        steer_angle_rad: 0.2,
        body_accel_up_mps2: -9.8,
        tyre_lateral_force_n: 120.0,
        static_load_n: 3500.0,
    };
    assert_eq!(first.wheels[0], expected);
}

#[test]
fn a_controls_closure_gets_the_driver_and_the_car_and_may_leave_the_steering() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (handling, _attached) = handling_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::<ControlsQuery>::new()));
    let log = seen.clone();
    let steer = Rc::new(Cell::new(Some(0.3_f32)));
    let target = steer.clone();
    handling
        .register_controls(move |_ctx, query| {
            log.borrow_mut().push(*query);
            Some(ControlsResult { throttle: 0.5, brake: [0.1, 0.2, 0.3, 0.4], steer_target_rad: target.get() })
        })
        .expect("registered");
    let hook = backend.controls.borrow().clone().expect("the host holds the hook");
    let mut model = hook.model(Rc::new(Cell::new(2)));
    let query = VehicleControlsQuery {
        role: 0,
        dt: 1.0 / 60.0,
        throttle: 1.0,
        brake: 0.0,
        handbrake: 0.0,
        steer: 0.25,
        forward_speed: 30.0,
        yaw_rate: -0.1,
        lateral_accel: -1.0,
        longitudinal_accel: 3.0,
        wheel_loaded: [true, true, false, true],
        slip_ratio: [0.0, 0.1, 0.0, -0.2],
        slip_angle: [0.01, 0.02, 0.0, 0.03],
    };
    let answer = model.filter(&query).expect("handled");
    assert_eq!((answer.throttle, answer.brake, answer.steer_target), (0.5, [0.1, 0.2, 0.3, 0.4], Some(0.3)));
    let first = seen.borrow()[0];
    assert_eq!((first.role, first.vehicle_revision), (Some(Role::Player), 2));
    assert_eq!((first.throttle, first.steer, first.forward_speed, first.yaw_rate), (1.0, 0.25, 30.0, -0.1));
    assert_eq!(first.wheel_loaded, [true, true, false, true]);
    assert_eq!((first.slip_ratio, first.slip_angle), ([0.0, 0.1, 0.0, -0.2], [0.01, 0.02, 0.0, 0.03]));
    steer.set(None);
    let answer = model.filter(&query).expect("handled");
    assert_eq!(answer.steer_target, None, "no target, no flag: the host keeps its steering");
}

#[test]
fn the_tyre_closure_sees_the_camber() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (handling, _attached) = handling_over(&backend, &mut host_struct);
    let camber = Rc::new(Cell::new(0.0_f32));
    let log = camber.clone();
    handling
        .register_tyre(move |_ctx, query: &TyreQuery| {
            log.set(query.camber_rad);
            Some(TyreResult { traction: 1.0, ..TyreResult::default() })
        })
        .expect("registered");
    let hook = backend.tyre.borrow().clone().expect("the host holds the hook");
    let mut model = hook.model(Rc::new(Cell::new(1)));
    let query = VehicleTyreQuery {
        wheel_index: 0,
        role: 0,
        loaded: true,
        dt: 1.0 / 60.0,
        vertical_load_n: 3000.0,
        patch_lateral_velocity: 0.0,
        patch_forward_velocity: 10.0,
        body_speed: 10.0,
        wheel_spin: 30.0,
        drive_torque_nm: 0.0,
        brake_torque_nm: 0.0,
        wheel_radius_m: 0.33,
        wheel_inertia_kgm2: 10.0,
        corner_mass_kg: 375.0,
        surface_lateral: 1.0,
        surface_drive: 1.0,
        surface_rolling: 1.0,
        surface_tag: 0,
        tyre_condition: 1.0,
        ambient_temperature_c: 20.0,
        road_temperature_c: 25.0,
        camber_rad: -0.05,
    };
    assert!(model.wheel(&query).is_some());
    assert_eq!(camber.get(), -0.05);
}

#[test]
fn a_panic_in_a_new_hook_is_a_fault_not_an_abort() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (handling, _attached) = handling_over(&backend, &mut host_struct);
    handling.register_suspension(|_ctx, _query| panic!("suspension test: expected panic")).expect("registered");
    handling.register_controls(|_ctx, _query| panic!("controls test: expected panic")).expect("registered");
    let suspension = backend.suspension.borrow().clone().expect("held");
    let controls = backend.controls.borrow().clone().expect("held");
    let mut suspension_model = suspension.model(Rc::new(Cell::new(1)));
    let mut controls_model = controls.model(Rc::new(Cell::new(1)));
    assert!(suspension_model.axle(&VehicleSuspensionQuery::default()).is_none());
    assert!(controls_model.filter(&VehicleControlsQuery::default()).is_none());
    assert_eq!((suspension.stats().faults(), controls.stats().faults()), (1, 1));
    assert_eq!(Error::Panic.code(), BB_ERR_PANIC);
}

#[test]
fn a_second_registration_of_a_kind_is_exists_and_unregister_removes_it() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (handling, _attached) = handling_over(&backend, &mut host_struct);
    let id = handling.register_suspension(|_ctx, _query| None).expect("registered");
    assert_eq!(handling.register_suspension(|_ctx, _query| None), Err(Error::Exists));
    handling.register_controls(|_ctx, _query| None).expect("another kind is no conflict");
    handling.unregister_suspension(id).expect("removed");
    assert!(backend.suspension.borrow().is_none());
    assert!(backend.controls.borrow().is_some());
}

/// A table as a 1.0 host built it: the first 32 bytes of today's, with the header saying so.
fn one_zero_table() -> (Box<BbHandling>, Interface) {
    let register: BbHandlingRegisterTyreFn = {
        unsafe extern "C" fn register(_: *mut core::ffi::c_void, _: *const crate::abi::BbTyreHook, _: *mut u64) -> i32 {
            0
        }
        register
    };
    let table = Box::new(BbHandling {
        head: BbIface { size: 32, major: 1, minor: 0 },
        ctx: core::ptr::null_mut(),
        register_tyre: Some(register),
        unregister_tyre: None,
        // A 1.0 host never wrote these: the head's size does not cover them, so a reader must not look.
        register_suspension: None,
        unregister_suspension: None,
        register_controls: None,
        unregister_controls: None,
    });
    let header = NonNull::from(&table.head);
    let iface = Interface::new(header, table.head);
    (table, iface)
}

#[test]
fn a_table_of_minor_zero_offers_no_suspension_or_controls_hook() {
    let (_table, iface) = one_zero_table();
    assert_eq!((iface.major(), iface.minor(), iface.size()), (1, 0, 32));
    let fns = sdk_handling::read(iface);
    assert!(fns.register_tyre.is_some(), "the tyre hook of 1.0 is read");
    assert!(fns.register_suspension.is_none() && fns.unregister_suspension.is_none());
    assert!(fns.register_controls.is_none() && fns.unregister_controls.is_none());
}

#[test]
fn a_table_of_minor_one_offers_suspension_but_not_controls() {
    let (mut table, _) = one_zero_table();
    table.head = BbIface { size: 48, major: 1, minor: 1 };
    unsafe extern "C" fn register(
        _: *mut core::ffi::c_void,
        _: *const crate::abi::BbSuspensionHook,
        _: *mut u64,
    ) -> i32 {
        0
    }
    unsafe extern "C" fn unregister(_: *mut core::ffi::c_void, _: u64) -> i32 {
        0
    }
    table.register_suspension = Some(register);
    table.unregister_suspension = Some(unregister);
    let header = NonNull::from(&table.head);
    let fns = sdk_handling::read(Interface::new(header, table.head));
    assert!(fns.register_suspension.is_some() && fns.unregister_suspension.is_some());
    assert!(fns.register_controls.is_none() && fns.unregister_controls.is_none(), "1.1 ends before the controls pair");
}
