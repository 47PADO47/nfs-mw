//! Layout snapshot (ADR 0005 section 9) and the `BbStr` round trip.
//!
//! The numbers are for 64-bit targets. Changing a field means editing these numbers on purpose, in the same
//! commit, and the change is then an ABI change that needs the rules in `mod.rs`.

use core::ptr;

use super::*;

#[test]
fn abi_version_is_locked() {
    assert_eq!(ABI_VERSION, AbiVersion { major: 0, minor: 1 });
}

#[test]
fn status_codes_are_locked() {
    assert_eq!(BB_OK, 0);
    assert_eq!(BB_ERR_ARG, -1);
    assert_eq!(BB_ERR_ABI, -2);
    assert_eq!(BB_ERR_UNSUPPORTED, -3);
    assert_eq!(BB_ERR_STALE, -4);
    assert_eq!(BB_ERR_PHASE, -5);
    assert_eq!(BB_ERR_NOT_READY, -6);
    assert_eq!(BB_ERR_NOT_IN_CALLBACK, -7);
    assert_eq!(BB_ERR_WRONG_THREAD, -8);
    assert_eq!(BB_ERR_DENIED, -9);
    assert_eq!(BB_ERR_NOT_FOUND, -10);
    assert_eq!(BB_ERR_EXISTS, -11);
    assert_eq!(BB_ERR_REMOVED, -12);
    assert_eq!(BB_ERR_PANIC, -13);
    assert_eq!(BB_ERR_HOST, -14);
    assert_eq!(BB_ERR_CORRUPT, -15);
}

#[test]
fn event_results_and_log_levels_are_locked() {
    assert_eq!(BB_EVENT_CONTINUE, 0);
    assert_eq!(BB_EVENT_CANCEL, 1);
    assert_eq!([BB_LOG_ERROR, BB_LOG_WARN, BB_LOG_INFO, BB_LOG_DEBUG, BB_LOG_TRACE], [1, 2, 3, 4, 5]);
}

#[test]
fn symbol_names_are_locked() {
    assert_eq!(MOD_INFO_SYMBOL, "bb_mod_info");
    assert_eq!(MOD_CREATE_SYMBOL, "bb_mod_create");
}

#[test]
fn status_is_a_four_byte_integer() {
    assert_eq!(core::mem::size_of::<BbStatus>(), 4);
}

#[test]
fn optional_function_pointers_are_one_pointer_wide() {
    assert_eq!(core::mem::size_of::<Option<BbLogFn>>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<Option<BbQueryInterfaceFn>>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<Option<BbOnEventFn>>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<Option<BbDestroyFn>>(), core::mem::size_of::<usize>());
}

#[cfg(target_pointer_width = "64")]
mod layout64 {
    use core::mem::{align_of, offset_of, size_of};

    use super::super::*;

    #[test]
    fn bbstr() {
        assert_eq!(size_of::<BbStr>(), 16);
        assert_eq!(align_of::<BbStr>(), 8);
        assert_eq!(offset_of!(BbStr, ptr), 0);
        assert_eq!(offset_of!(BbStr, len), 8);
    }

    #[test]
    fn iface_header() {
        assert_eq!(size_of::<BbIface>(), 8);
        assert_eq!(align_of::<BbIface>(), 4);
        assert_eq!(offset_of!(BbIface, size), 0);
        assert_eq!(offset_of!(BbIface, major), 4);
        assert_eq!(offset_of!(BbIface, minor), 6);
    }

    #[test]
    fn mod_info() {
        assert_eq!(size_of::<BbModInfo>(), 24);
        assert_eq!(align_of::<BbModInfo>(), 8);
        assert_eq!(offset_of!(BbModInfo, size), 0);
        assert_eq!(offset_of!(BbModInfo, abi_major), 4);
        assert_eq!(offset_of!(BbModInfo, abi_minor), 6);
        assert_eq!(offset_of!(BbModInfo, sdk_version), 8);
    }

    #[test]
    fn host_table() {
        assert_eq!(size_of::<BbHost>(), 64);
        assert_eq!(align_of::<BbHost>(), 8);
        assert_eq!(offset_of!(BbHost, size), 0);
        assert_eq!(offset_of!(BbHost, abi_major), 4);
        assert_eq!(offset_of!(BbHost, abi_minor), 6);
        assert_eq!(offset_of!(BbHost, ctx), 8);
        assert_eq!(offset_of!(BbHost, game_id), 16);
        assert_eq!(offset_of!(BbHost, game_version), 32);
        assert_eq!(offset_of!(BbHost, log), 48);
        assert_eq!(offset_of!(BbHost, query_interface), 56);
    }

    #[test]
    fn mod_table() {
        assert_eq!(size_of::<BbModApi>(), 32);
        assert_eq!(align_of::<BbModApi>(), 8);
        assert_eq!(offset_of!(BbModApi, size), 0);
        assert_eq!(offset_of!(BbModApi, abi_major), 4);
        assert_eq!(offset_of!(BbModApi, abi_minor), 6);
        assert_eq!(offset_of!(BbModApi, state), 8);
        assert_eq!(offset_of!(BbModApi, on_event), 16);
        assert_eq!(offset_of!(BbModApi, destroy), 24);
    }

    #[test]
    fn event() {
        assert_eq!(size_of::<BbEvent>(), 24);
        assert_eq!(align_of::<BbEvent>(), 8);
        assert_eq!(offset_of!(BbEvent, size), 0);
        assert_eq!(offset_of!(BbEvent, kind), 4);
        assert_eq!(offset_of!(BbEvent, frame), 8);
        assert_eq!(offset_of!(BbEvent, payload), 16);
    }
}

#[test]
fn has_field_covers_exactly_the_bytes_declared() {
    assert!(has_field(16, 16));
    assert!(!has_field(15, 16));
    assert!(has_field(64, 8));
    assert!(!has_field(0, 8));
}

#[test]
fn bbstr_round_trips_utf8() {
    let text = "caf\u{e9} 42";
    let s = BbStr::new(text);
    // SAFETY: `s` borrows `text`, which outlives this read.
    assert_eq!(unsafe { s.as_str() }, Some(text));
}

#[test]
fn bbstr_empty_is_the_empty_string() {
    // SAFETY: the empty string reads no bytes.
    assert_eq!(unsafe { BbStr::EMPTY.as_str() }, Some(""));
}

#[test]
fn bbstr_null_with_a_length_is_rejected() {
    let s = BbStr { ptr: ptr::null(), len: 3 };
    // SAFETY: a null pointer is rejected before any read.
    assert_eq!(unsafe { s.as_str() }, None);
}

#[test]
fn bbstr_invalid_utf8_is_rejected() {
    let bytes = [0xff_u8, 0x00];
    let s = BbStr { ptr: bytes.as_ptr(), len: 1 };
    // SAFETY: `bytes` is valid for one read and outlives the call.
    assert_eq!(unsafe { s.as_str() }, None);
}
