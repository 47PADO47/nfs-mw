//! Layout snapshot of the interface tables (ADR 0005 section 9). Changing a number here is an ABI change.

#![cfg(target_pointer_width = "64")]

use core::mem::{align_of, offset_of, size_of};

use crate::abi::*;

#[test]
fn ids_and_versions_are_locked() {
    assert_eq!(IFACE_COMMANDS, "bb.commands");
    assert_eq!(IFACE_INPUT, "bb.input");
    assert_eq!(IFACE_TIME, "bb.time");
    assert_eq!(IFACE_MAP, "bb.map");
    assert_eq!(IFACE_MENUS, "bb.menus");
    assert_eq!(
        [IFACE_COMMANDS_VERSION, IFACE_INPUT_VERSION, IFACE_TIME_VERSION, IFACE_MAP_VERSION, IFACE_MENUS_VERSION],
        [(1, 0), (1, 0), (1, 0), (1, 0), (1, 0)]
    );
}

#[test]
fn menus_table_and_row() {
    assert_eq!(size_of::<BbMenuRow>(), 72);
    assert_eq!(offset_of!(BbMenuRow, name), 8);
    assert_eq!(offset_of!(BbMenuRow, label), 24);
    assert_eq!(offset_of!(BbMenuRow, min), 40);
    assert_eq!(offset_of!(BbMenuRow, default), 52);
    assert_eq!(offset_of!(BbMenuRow, choices), 56);
    assert_eq!(offset_of!(BbMenuRow, choice_count), 64);
    assert_eq!(size_of::<BbMenus>(), 48);
    assert_eq!(offset_of!(BbMenus, ctx), 8);
    assert_eq!(offset_of!(BbMenus, register), 16);
    assert_eq!(offset_of!(BbMenus, unregister), 24);
    assert_eq!(offset_of!(BbMenus, get), 32);
    assert_eq!(offset_of!(BbMenus, set), 40);
    assert_eq!([BB_MENU_TOGGLE, BB_MENU_SLIDER, BB_MENU_LIST, BB_MENU_BUTTON], [0, 1, 2, 3]);
}

#[test]
fn commands_table() {
    assert_eq!(size_of::<BbCommands>(), 40);
    assert_eq!(align_of::<BbCommands>(), 8);
    assert_eq!(offset_of!(BbCommands, head), 0);
    assert_eq!(offset_of!(BbCommands, ctx), 8);
    assert_eq!(offset_of!(BbCommands, run), 16);
    assert_eq!(offset_of!(BbCommands, register), 24);
    assert_eq!(offset_of!(BbCommands, unregister), 32);
}

#[test]
fn input_table() {
    assert_eq!(size_of::<BbInput>(), 32);
    assert_eq!(offset_of!(BbInput, ctx), 8);
    assert_eq!(offset_of!(BbInput, register_hotkey), 16);
    assert_eq!(offset_of!(BbInput, unregister), 24);
}

#[test]
fn time_table_and_clock() {
    assert_eq!(size_of::<BbTime>(), 48);
    assert_eq!(offset_of!(BbTime, get), 16);
    assert_eq!(offset_of!(BbTime, set_hour), 24);
    assert_eq!(offset_of!(BbTime, set_day_length), 32);
    assert_eq!(offset_of!(BbTime, freeze), 40);
    assert_eq!(size_of::<BbTimeOfDay>(), 12);
    assert_eq!(offset_of!(BbTimeOfDay, frozen), 8);
    assert_eq!(offset_of!(BbTimeOfDay, lights_on), 9);
}

#[test]
fn map_table_and_blip() {
    assert_eq!(size_of::<BbMap>(), 40);
    assert_eq!(offset_of!(BbMap, add), 16);
    assert_eq!(offset_of!(BbMap, update), 24);
    assert_eq!(offset_of!(BbMap, remove), 32);
    assert_eq!(size_of::<BbRgba>(), 4);
    assert_eq!(size_of::<BbBlip>(), 40);
    assert_eq!(offset_of!(BbBlip, active), 4);
    assert_eq!(offset_of!(BbBlip, style), 8);
    assert_eq!(offset_of!(BbBlip, position), 24);
}
