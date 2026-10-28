//! Layout snapshot of `bb.traffic` (ADR 0005 section 9). Changing a number here is an ABI change.

#![cfg(target_pointer_width = "64")]

use core::mem::{offset_of, size_of};

use crate::abi::*;

#[test]
fn the_id_version_and_field_separator_are_locked() {
    assert_eq!(IFACE_TRAFFIC, "bb.traffic");
    assert_eq!(IFACE_TRAFFIC_VERSION, (1, 0));
    assert_eq!(BB_TRAFFIC_FIELD, '\t');
}

#[test]
fn the_table_has_its_layout() {
    assert_eq!(size_of::<BbTraffic>(), 32);
    assert_eq!(offset_of!(BbTraffic, ctx), 8);
    assert_eq!(offset_of!(BbTraffic, list), 16);
    assert_eq!(offset_of!(BbTraffic, spawn), 24);
}
