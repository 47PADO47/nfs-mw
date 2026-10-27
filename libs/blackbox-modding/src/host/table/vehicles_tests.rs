//! `bb.vehicles` called directly with a fake backend: the state is written up to the size the mod allows, moves reach the
//! backend checked, and a non-finite value or an unsupported spawn is answered with its status.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;

use super::vehicles::{VehicleState, VehiclesBackend};
use super::{Attached, attach_vehicles};
use crate::abi::*;
use crate::host::BackendResult;

/// The table an attachment holds. The header is the first field of every interface table.
fn table_of(attached: &Attached) -> &BbVehicles {
    // SAFETY: the attachment owns a `BbVehicles` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbVehicles>() }
}

#[derive(Default)]
struct FakeVehicles {
    car: VehicleState,
    moves: RefCell<Vec<([f32; 3], f32)>>,
    repairs: RefCell<u32>,
    nos: RefCell<Vec<f32>>,
}

impl VehiclesBackend for FakeVehicles {
    fn state(&self, _owner: &str) -> BackendResult<VehicleState> {
        Ok(self.car)
    }

    fn teleport(&self, _owner: &str, position: [f32; 3], heading: f32) -> BackendResult<()> {
        self.moves.borrow_mut().push((position, heading));
        Ok(())
    }

    fn repair(&self, _owner: &str) -> BackendResult<()> {
        *self.repairs.borrow_mut() += 1;
        Ok(())
    }

    fn spawn_traffic(&self, _owner: &str, _position: [f32; 3], _heading: f32) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }

    fn add_nos(&self, _owner: &str, amount: f32) -> BackendResult<()> {
        self.nos.borrow_mut().push(amount);
        Ok(())
    }
}

fn fixture() -> (Rc<FakeVehicles>, Attached) {
    let car = VehicleState {
        gear: 3,
        position: [1.0, 2.0, 0.5],
        velocity: [30.0, 0.0, 0.0],
        heading: 0.25,
        speed: 30.0,
        rpm: 4200.0,
        nos: 0.5,
        damage: 0.1,
        slip_angle: 0.2,
        steering: -0.4,
        impact: 0.3,
        on_ground: true,
    };
    let fake = Rc::new(FakeVehicles { car, ..FakeVehicles::default() });
    let attached = attach_vehicles(fake.clone(), "example.a");
    (fake, attached)
}

fn whole_state() -> BbVehicleState {
    BbVehicleState { size: size_of::<BbVehicleState>() as u32, ..BbVehicleState::default() }
}

#[test]
fn state_is_written_in_full_when_the_size_allows() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    let mut out = whole_state();
    let read = table.state.expect("state is offered");
    assert_eq!(unsafe { read(table.ctx, &mut out) }, BB_OK);
    assert_eq!(out.size as usize, size_of::<BbVehicleState>(), "the size word is the caller's");
    assert_eq!(out.gear, 3);
    assert_eq!(out.position, BbVec3 { x: 1.0, y: 2.0, z: 0.5 });
    assert_eq!(out.velocity.x, 30.0);
    assert_eq!((out.heading, out.speed, out.rpm, out.nos, out.damage), (0.25, 30.0, 4200.0, 0.5, 0.1));
    assert_eq!((out.slip_angle, out.steering, out.impact, out.on_ground), (0.2, -0.4, 0.3, 1));
}

#[test]
fn a_caller_built_for_1_0_gets_the_1_0_fields_and_nothing_past_them() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    // The 1.0 struct ended after `damage`, 52 bytes.
    let mut out = BbVehicleState { size: 52, ..BbVehicleState::default() };
    let read = table.state.expect("state is offered");
    assert_eq!(unsafe { read(table.ctx, &mut out) }, BB_OK);
    assert_eq!((out.nos, out.damage), (0.5, 0.1));
    assert_eq!(
        (out.slip_angle, out.steering, out.impact, out.on_ground),
        (0.0, 0.0, 0.0, 0),
        "1.1 fields stay untouched"
    );
}

#[test]
fn state_stops_at_the_size_the_caller_allows() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    // A caller built for the first two fields only: the size word and the gear.
    let mut out = BbVehicleState { size: 8, ..BbVehicleState::default() };
    let read = table.state.expect("state is offered");
    assert_eq!(unsafe { read(table.ctx, &mut out) }, BB_OK);
    assert_eq!(out.gear, 3, "the field inside the size is written");
    assert_eq!(out.rpm, 0.0, "a field past the size is left alone");
}

#[test]
fn a_size_below_the_size_word_is_refused() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    let mut out = BbVehicleState { size: 2, ..BbVehicleState::default() };
    let read = table.state.expect("state is offered");
    assert_eq!(unsafe { read(table.ctx, &mut out) }, BB_ERR_ARG);
    assert_eq!(unsafe { read(table.ctx, core::ptr::null_mut()) }, BB_ERR_ARG);
}

#[test]
fn teleport_reaches_the_backend_once_the_point_is_finite() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let table = table_of(&attached);
    let teleport = table.teleport.expect("teleport is offered");
    assert_eq!(unsafe { teleport(table.ctx, BbVec3 { x: 5.0, y: -2.0, z: 0.0 }, 1.5) }, BB_OK);
    assert_eq!(*fake.moves.borrow(), vec![([5.0, -2.0, 0.0], 1.5)]);
    assert_eq!(unsafe { teleport(table.ctx, BbVec3 { x: f32::NAN, y: 0.0, z: 0.0 }, 0.0) }, BB_ERR_ARG);
    assert_eq!(unsafe { teleport(table.ctx, BbVec3::default(), f32::INFINITY) }, BB_ERR_ARG);
    assert_eq!(fake.moves.borrow().len(), 1, "a refused move reaches nothing");
}

#[test]
fn repair_reaches_the_backend() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let table = table_of(&attached);
    let repair = table.repair.expect("repair is offered");
    assert_eq!(unsafe { repair(table.ctx) }, BB_OK);
    assert_eq!(*fake.repairs.borrow(), 1);
}

#[test]
fn add_nos_reaches_the_backend_for_a_part_of_a_tank_and_nothing_else() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let table = table_of(&attached);
    let add = table.add_nos.expect("add_nos is offered");
    assert_eq!(unsafe { add(table.ctx, 0.25) }, BB_OK);
    assert_eq!(unsafe { add(table.ctx, 0.0) }, BB_OK);
    assert_eq!(unsafe { add(table.ctx, 1.0) }, BB_OK);
    for bad in [-0.1, 1.5, f32::NAN, f32::INFINITY] {
        assert_eq!(unsafe { add(table.ctx, bad) }, BB_ERR_ARG, "{bad}");
    }
    assert_eq!(*fake.nos.borrow(), vec![0.25, 0.0, 1.0], "a refused amount reaches nothing");
}

#[test]
fn the_table_says_it_is_1_1() {
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    assert_eq!((table.head.major, table.head.minor), (1, 1));
    assert_eq!(table.head.size as usize, size_of::<BbVehicles>());
}

#[test]
fn spawning_traffic_answers_unsupported_in_1_0() {
    let _scope = callback_on_main();
    let (_fake, attached) = fixture();
    let table = table_of(&attached);
    let spawn = table.spawn_traffic.expect("spawn_traffic is offered");
    let mut id = 0_u64;
    assert_eq!(unsafe { spawn(table.ctx, BbVec3::default(), 0.0, &mut id) }, BB_ERR_UNSUPPORTED);
    assert_eq!(id, 0);
}
