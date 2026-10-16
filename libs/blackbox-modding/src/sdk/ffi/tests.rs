//! Trampoline tests against a fake host table. A panic must come back as a status code, never as an unwind
//! (an unwind out of `extern "C"` would abort the test binary).

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

use std::cell::RefCell;
use std::rc::Rc;

use super::{HostTable, create};
use crate::abi::{
    ABI_MAJOR, ABI_MINOR, BB_ERR_ABI, BB_ERR_ARG, BB_ERR_DENIED, BB_ERR_PANIC, BB_EVENT_CONTINUE, BB_OK, BbEvent,
    BbHost, BbIface, BbModApi, BbModInfo, BbStr,
};
use crate::sdk::{Ctx, Error, Event, Level, Mod, ModResult, SDK_VERSION};

/// A mod that loads, counts its events and records the last frame. Its counters are used by one test only.
struct Probe;

static PROBE_LOADS: AtomicU32 = AtomicU32::new(0);
static PROBE_EVENTS: AtomicU32 = AtomicU32::new(0);
static PROBE_FRAME: AtomicU64 = AtomicU64::new(0);
static PROBE_UNLOADS: AtomicU32 = AtomicU32::new(0);

impl Mod for Probe {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        ctx.log(Level::Info, "probe loaded");
        PROBE_LOADS.fetch_add(1, Relaxed);
        Ok(Probe)
    }

    fn event(&mut self, _ctx: &mut Ctx, event: &Event) {
        PROBE_EVENTS.fetch_add(1, Relaxed);
        PROBE_FRAME.store(event.frame(), Relaxed);
    }

    fn unload(&mut self, _ctx: &mut Ctx) {
        PROBE_UNLOADS.fetch_add(1, Relaxed);
    }
}

/// A mod whose load returns an error.
struct Refuses;

impl Mod for Refuses {
    fn load(_ctx: &mut Ctx) -> ModResult<Self> {
        Err(Error::Denied)
    }
}

/// A mod that panics while loading.
struct LoadPanics;

impl Mod for LoadPanics {
    fn load(_ctx: &mut Ctx) -> ModResult<Self> {
        panic!("load test: expected panic")
    }
}

/// A mod that loads and then panics on every event.
struct Exploder;

impl Mod for Exploder {
    fn load(_ctx: &mut Ctx) -> ModResult<Self> {
        Ok(Exploder)
    }

    fn event(&mut self, _ctx: &mut Ctx, _event: &Event) {
        panic!("event test: expected panic")
    }
}

crate::declare_mod!(Probe);

static FAKE_IFACE: BbIface = BbIface { size: size_of::<BbIface>() as u32, major: 1, minor: 2 };

unsafe extern "C" fn fake_log(_ctx: *mut c_void, _level: u32, _msg: BbStr) {}

unsafe extern "C" fn fake_query(_ctx: *mut c_void, id: BbStr, major: u16, _min_minor: u16) -> *const BbIface {
    // SAFETY: the test passes a string literal, valid for the call.
    let known = unsafe { id.as_str() } == Some("known");
    if known && major == 1 {
        return ptr::addr_of!(FAKE_IFACE);
    }
    ptr::null()
}

fn fake_host() -> BbHost {
    BbHost {
        size: size_of::<BbHost>() as u32,
        abi_major: ABI_MAJOR,
        abi_minor: ABI_MINOR,
        ctx: ptr::null_mut(),
        game_id: BbStr::new("testgame"),
        game_version: BbStr::new("1.0"),
        log: Some(fake_log),
        query_interface: Some(fake_query),
    }
}

fn empty_api() -> BbModApi {
    BbModApi { size: 0, abi_major: 0, abi_minor: 0, state: ptr::null_mut(), on_event: None, destroy: None }
}

fn host_table(host: &BbHost) -> HostTable {
    // SAFETY: `host` is a live reference for the whole test, and its header is readable.
    unsafe { HostTable::new(NonNull::from(host)) }
}

/// A `Ctx` over the fake host, with an empty keep-alive list.
fn test_ctx(host: &BbHost) -> Ctx {
    Ctx::new(host_table(host), Rc::new(RefCell::new(Vec::new())))
}

#[test]
fn declared_entry_points_round_trip() {
    let info_ptr = bb_mod_info();
    // SAFETY: the exported info is a pointer to a static `BbModInfo`.
    let info: BbModInfo = unsafe { *info_ptr };
    assert_eq!(info.size as usize, size_of::<BbModInfo>());
    assert_eq!((info.abi_major, info.abi_minor), (ABI_MAJOR, ABI_MINOR));
    // SAFETY: the SDK version is a static string.
    assert_eq!(unsafe { info.sdk_version.as_str() }, Some(SDK_VERSION));

    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { bb_mod_create(&host, &mut api) }, BB_OK);
    assert_eq!(PROBE_LOADS.load(Relaxed), 1);

    let on_event = api.on_event.expect("create fills on_event");
    let event = BbEvent { size: size_of::<BbEvent>() as u32, kind: 3, frame: 42, payload: ptr::null() };
    // SAFETY: the state and the event come from this create and are valid for the call.
    assert_eq!(unsafe { on_event(api.state, &event) }, BB_EVENT_CONTINUE);
    assert_eq!(PROBE_EVENTS.load(Relaxed), 1);
    assert_eq!(PROBE_FRAME.load(Relaxed), 42);

    let destroy = api.destroy.expect("create fills destroy");
    // SAFETY: the state came from this create and is destroyed once.
    unsafe { destroy(api.state) };
    assert_eq!(PROBE_UNLOADS.load(Relaxed), 1);
}

#[test]
fn create_requires_the_same_minor_while_the_major_is_zero() {
    // The mod is built for 0.ABI_MINOR. While the major is 0, a host on any other minor is refused.
    let mut host = fake_host();
    host.abi_minor = ABI_MINOR;
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    let status = unsafe { create::<Refuses>(&host, &mut api) };
    // The handshake passed and `load` ran, so the mod's own error comes back.
    assert_eq!(status, BB_ERR_DENIED);

    for minor in [0, ABI_MINOR + 1] {
        let mut host = fake_host();
        host.abi_minor = minor;
        let mut api = empty_api();
        // SAFETY: both pointers are valid for the call.
        assert_eq!(unsafe { create::<Refuses>(&host, &mut api) }, BB_ERR_ABI, "host minor {minor}");
        assert_eq!(api.size, 0, "the mod table must stay untouched");
    }
}

#[test]
fn create_rejects_a_major_mismatch_before_loading() {
    let mut host = fake_host();
    host.abi_major = ABI_MAJOR + 1;
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { create::<Refuses>(&host, &mut api) }, BB_ERR_ABI);
    assert_eq!(api.size, 0, "the mod table must stay untouched");
}

#[test]
fn create_rejects_null_pointers() {
    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: a null host is rejected before any read.
    assert_eq!(unsafe { create::<Refuses>(ptr::null(), &mut api) }, BB_ERR_ARG);
    // SAFETY: a null output is rejected before any write.
    assert_eq!(unsafe { create::<Refuses>(&host, ptr::null_mut()) }, BB_ERR_ARG);
}

#[test]
fn a_failed_load_writes_nothing() {
    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { create::<Refuses>(&host, &mut api) }, BB_ERR_DENIED);
    assert_eq!(api.size, 0);
    assert!(api.state.is_null());
}

#[test]
fn a_panic_in_load_comes_back_as_a_status() {
    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { create::<LoadPanics>(&host, &mut api) }, BB_ERR_PANIC);
    assert_eq!(api.size, 0);
}

#[test]
fn a_panic_in_event_comes_back_as_a_status_and_the_mod_still_unloads() {
    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { create::<Exploder>(&host, &mut api) }, BB_OK);
    let event = BbEvent { size: size_of::<BbEvent>() as u32, kind: 1, frame: 9, payload: ptr::null() };
    let on_event = api.on_event.expect("create fills on_event");
    // SAFETY: the state and the event come from this create and are valid for the call.
    assert_eq!(unsafe { on_event(api.state, &event) }, BB_ERR_PANIC);
    let destroy = api.destroy.expect("create fills destroy");
    // SAFETY: the state came from this create and is destroyed once.
    unsafe { destroy(api.state) };
}

#[test]
fn an_event_too_short_for_its_frame_is_refused_before_dispatch() {
    let host = fake_host();
    let mut api = empty_api();
    // SAFETY: both pointers are valid for the call.
    assert_eq!(unsafe { create::<Exploder>(&host, &mut api) }, BB_OK);
    // The size covers `kind` but not `frame`. Dispatch would panic in `Exploder`, so ARG proves it was refused.
    let event = BbEvent { size: 8, kind: 1, frame: 0, payload: ptr::null() };
    let on_event = api.on_event.expect("create fills on_event");
    // SAFETY: the state and the event come from this create and are valid for the call.
    assert_eq!(unsafe { on_event(api.state, &event) }, BB_ERR_ARG);
    let destroy = api.destroy.expect("create fills destroy");
    // SAFETY: the state came from this create and is destroyed once.
    unsafe { destroy(api.state) };
}

#[test]
fn query_interface_hands_out_only_a_matching_version() {
    let host = fake_host();
    let ctx = test_ctx(&host);
    let iface = ctx.query_interface("known", 1, 2).expect("offered at 1.2");
    assert_eq!((iface.major(), iface.minor()), (1, 2));
    assert!(ctx.query_interface("known", 1, 3).is_none(), "minor too old");
    assert!(ctx.query_interface("known", 2, 0).is_none(), "major differs");
    assert!(ctx.query_interface("unknown", 1, 0).is_none(), "absent");
}

#[test]
fn a_short_host_table_hides_the_fields_it_does_not_have() {
    let mut host = fake_host();
    host.size = offset_of!(BbHost, query_interface) as u32;
    let ctx = test_ctx(&host);
    assert!(ctx.query_interface("known", 1, 0).is_none());
    assert_eq!(ctx.game_id(), Ok("testgame".to_owned()));
    ctx.log(Level::Info, "still logs: the log field is covered");
}

#[test]
fn game_id_and_version_are_copied_from_the_host() {
    let host = fake_host();
    let ctx = test_ctx(&host);
    assert_eq!(ctx.game_id(), Ok("testgame".to_owned()));
    assert_eq!(ctx.game_version(), Ok("1.0".to_owned()));
}
