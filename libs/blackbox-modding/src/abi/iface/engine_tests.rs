//! Layout snapshot of `bb.engine` (ADR 0005 section 9). Changing a number here is an ABI change.

#![cfg(target_pointer_width = "64")]

use core::mem::{offset_of, size_of};

use crate::abi::*;

#[test]
fn the_id_version_and_phases_are_locked() {
    assert_eq!(IFACE_ENGINE, "bb.engine");
    assert_eq!(IFACE_ENGINE_VERSION, (1, 0));
    assert_eq!([BB_ENGINE_OFF, BB_ENGINE_STARTING, BB_ENGINE_RUNNING], [0, 1, 2]);
}

#[test]
fn the_table_and_the_state_have_their_layout() {
    assert_eq!(size_of::<BbEngine>(), 48);
    assert_eq!(offset_of!(BbEngine, ctx), 8);
    assert_eq!(offset_of!(BbEngine, state), 16);
    assert_eq!(offset_of!(BbEngine, car), 24);
    assert_eq!(offset_of!(BbEngine, stop), 32);
    assert_eq!(offset_of!(BbEngine, start), 40);
    assert_eq!(size_of::<BbEngineState>(), 24);
    assert_eq!(offset_of!(BbEngineState, phase), 4);
    assert_eq!(offset_of!(BbEngineState, free_roam), 8);
    assert_eq!(offset_of!(BbEngineState, throttle), 12);
    assert_eq!(offset_of!(BbEngineState, brake), 16);
    assert_eq!(offset_of!(BbEngineState, speed), 20);
}
