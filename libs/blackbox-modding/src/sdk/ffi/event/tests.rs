//! Decoding a `BbEvent` into `Event`: typed payloads, unknown kinds, and payloads the host did not fill in far enough.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr;

use super::decode;
use crate::abi::*;
use crate::sdk::{Collider, Event, Role};

fn header(kind: u32, frame: u64, payload: *const c_void) -> BbEvent {
    BbEvent { size: size_of::<BbEvent>() as u32, kind, frame, payload }
}

#[test]
fn a_null_event_or_a_short_header_is_refused() {
    // SAFETY: a null pointer is rejected before any read.
    assert_eq!(unsafe { decode(ptr::null()) }, None);
    // The size covers `kind` and `frame`, but not `payload`, so the header is complete enough and the payload is absent.
    let short = BbEvent {
        size: offset_of!(BbEvent, frame) as u32 + 8,
        kind: BB_EVENT_FRAME_START,
        frame: 1,
        payload: ptr::null(),
    };
    // SAFETY: the event lives for the call; its size covers the fields it is read for.
    assert_eq!(unsafe { decode(&short) }, Some(Event::Unknown { kind: BB_EVENT_FRAME_START, frame: 1 }));
    let too_short = BbEvent { size: 8, kind: 1, frame: 0, payload: ptr::null() };
    // SAFETY: as above.
    assert_eq!(unsafe { decode(&too_short) }, None);
}

#[test]
fn an_unknown_kind_decodes_as_unknown_with_its_kind() {
    let event = header(9_999, 42, ptr::null());
    // SAFETY: the event lives for the call.
    assert_eq!(unsafe { decode(&event) }, Some(Event::Unknown { kind: 9_999, frame: 42 }));
}

#[test]
fn a_payload_kind_without_a_payload_is_unknown() {
    let event = header(BB_EVENT_VEHICLE_SPAWNED, 5, ptr::null());
    // SAFETY: the event lives for the call.
    assert_eq!(unsafe { decode(&event) }, Some(Event::Unknown { kind: BB_EVENT_VEHICLE_SPAWNED, frame: 5 }));
}

#[test]
fn a_payload_shorter_than_the_struct_is_unknown() {
    let payload = BbFrameStart { size: 4, dt_real: 0.5 };
    let event = header(BB_EVENT_FRAME_START, 6, ptr::addr_of!(payload).cast());
    // SAFETY: the payload lives for the call and its size word says four bytes, so nothing beyond is read.
    assert_eq!(unsafe { decode(&event) }, Some(Event::Unknown { kind: BB_EVENT_FRAME_START, frame: 6 }));
}

#[test]
fn frame_start_and_scene_tick_decode_their_fields() {
    let frame = BbFrameStart { size: size_of::<BbFrameStart>() as u32, dt_real: 0.25 };
    let event = header(BB_EVENT_FRAME_START, 7, ptr::addr_of!(frame).cast());
    // SAFETY: the payload lives for the call.
    assert_eq!(unsafe { decode(&event) }, Some(Event::FrameStart { frame: 7, dt_real: 0.25 }));

    let tick =
        BbSceneTick { size: size_of::<BbSceneTick>() as u32, dt_sim: 0.5, time_scale: 2.0, paused: 1, _pad: [0; 3] };
    let event = header(BB_EVENT_SCENE_TICK, 8, ptr::addr_of!(tick).cast());
    // SAFETY: as above.
    assert_eq!(
        unsafe { decode(&event) },
        Some(Event::SceneTick { frame: 8, dt_sim: 0.5, time_scale: 2.0, paused: true })
    );
}

#[test]
fn vehicle_events_decode_roles_and_colliders() {
    let spawned = BbVehicleSpawned {
        size: size_of::<BbVehicleSpawned>() as u32,
        role: BB_ROLE_COP,
        vehicle: 0x0000_0001_0000_0003,
        car_id: 77,
    };
    let event = header(BB_EVENT_VEHICLE_SPAWNED, 9, ptr::addr_of!(spawned).cast());
    // SAFETY: the payload lives for the call.
    let decoded = unsafe { decode(&event) };
    assert_eq!(decoded.map(|event| event.kind()), Some(BB_EVENT_VEHICLE_SPAWNED));
    let Some(Event::VehicleSpawned { vehicle, role, car_id, .. }) = decoded else {
        panic!("expected VehicleSpawned, got {decoded:?}");
    };
    assert_eq!((vehicle.raw(), role, car_id), (0x0000_0001_0000_0003, Role::Cop, 77));

    let collision = BbVehicleCollision {
        size: size_of::<BbVehicleCollision>() as u32,
        other_kind: BB_COLLIDE_WALL,
        vehicle: 5,
        other_id: 0,
        point: [1.0, 2.0, 3.0],
        impulse: 9.5,
    };
    let event = header(BB_EVENT_VEHICLE_COLLISION, 10, ptr::addr_of!(collision).cast());
    // SAFETY: as above.
    let decoded = unsafe { decode(&event) };
    let Some(Event::VehicleCollision { other, impulse, point, .. }) = decoded else {
        panic!("expected VehicleCollision, got {decoded:?}");
    };
    assert_eq!((other, impulse, point), (Collider::Wall, 9.5, [1.0, 2.0, 3.0]));
}

#[test]
fn an_unknown_role_makes_the_event_unknown_not_wrong() {
    let spawned = BbVehicleSpawned { size: size_of::<BbVehicleSpawned>() as u32, role: 99, vehicle: 1, car_id: 0 };
    let event = header(BB_EVENT_VEHICLE_SPAWNED, 11, ptr::addr_of!(spawned).cast());
    // SAFETY: the payload lives for the call.
    assert_eq!(unsafe { decode(&event) }, Some(Event::Unknown { kind: BB_EVENT_VEHICLE_SPAWNED, frame: 11 }));
}

#[test]
fn action_pressed_and_mod_unloading_borrow_their_names() {
    let action =
        BbActionPressed { size: size_of::<BbActionPressed>() as u32, _pad: 0, action: 0, name: BbStr::new("nitro") };
    let event = header(BB_EVENT_ACTION_PRESSED, 12, ptr::addr_of!(action).cast());
    // SAFETY: the payload and the name it borrows live for the call.
    assert_eq!(
        unsafe { decode(&event) },
        Some(Event::ActionPressed { frame: 12, action: crate::sdk::ActionId::from_raw(0), name: "nitro" })
    );

    let unloading =
        BbModUnloading { size: size_of::<BbModUnloading>() as u32, _pad: 0, mod_id: BbStr::new("example.hello") };
    let event = header(BB_EVENT_MOD_UNLOADING, 13, ptr::addr_of!(unloading).cast());
    // SAFETY: as above.
    assert_eq!(unsafe { decode(&event) }, Some(Event::ModUnloading { frame: 13, mod_id: "example.hello" }));
}

#[test]
fn kind_and_frame_accessors_match_the_decoded_event() {
    let hour = BbHourChanged { size: size_of::<BbHourChanged>() as u32, hour: 22 };
    let event = header(BB_EVENT_HOUR_CHANGED, 14, ptr::addr_of!(hour).cast());
    // SAFETY: the payload lives for the call.
    let decoded = unsafe { decode(&event) }.expect("an hour event decodes");
    assert_eq!((decoded.kind(), decoded.frame()), (BB_EVENT_HOUR_CHANGED, 14));
    assert_eq!(Event::Unknown { kind: 500, frame: 3 }.kind(), 500);
}
