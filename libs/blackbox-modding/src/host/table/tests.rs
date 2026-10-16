//! The host's interface tables, called directly with fake backends (no cdylib): arguments are checked, output reaches the
//! sink, a panic comes back as `BB_ERR_PANIC`, and the registrations a mod leaves behind are removed when it unloads.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use super::{Attached, BlipSpec, CommandHandler, CommandsBackend, HostTable, InputBackend, MapBackend, TimeBackend};
use super::{attach_commands, attach_input, attach_map, attach_time};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::registry::InterfaceRegistry;

/// The table an attachment holds. The header is the first field of every interface table.
fn table_of<T>(attached: &Attached) -> &T {
    // SAFETY: the attachment owns a table of type `T` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<T>() }
}

/// A `BbSinkFn` that appends the text to the `String` behind `user`.
unsafe extern "C" fn into_string(user: *mut c_void, text: BbStr) {
    // SAFETY: the tests pass a live `String` as `user`, and the text is valid for the call.
    unsafe { (*user.cast::<String>()).push_str(text.as_str().unwrap_or("")) };
}

#[derive(Default)]
struct FakeCommands {
    handlers: RefCell<Vec<(u64, CommandHandler)>>,
    removed: RefCell<Vec<u64>>,
    panic_on_run: bool,
}

impl CommandsBackend for FakeCommands {
    fn run(&self, owner: &str, line: &str) -> BackendResult<String> {
        if self.panic_on_run {
            panic!("fake backend panic");
        }
        Ok(format!("{owner} ran {line}"))
    }

    fn register(&self, _owner: &str, _name: &str, _help: &str, handler: CommandHandler) -> BackendResult<u64> {
        let handle = make_handle(self.handlers.borrow().len() as u32 + 1, 1);
        self.handlers.borrow_mut().push((handle, handler));
        Ok(handle)
    }

    fn unregister(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(handle);
        Ok(())
    }
}

/// A mod command that counts its calls and echoes the first word through the sink.
unsafe extern "C" fn echo(
    user: *mut c_void,
    argv: *const BbStr,
    argc: u64,
    sink: Option<BbSinkFn>,
    sink_user: *mut c_void,
) -> BbStatus {
    // SAFETY: the tests pass a live `Cell<u32>` as `user`.
    let calls = unsafe { &*user.cast::<Cell<u32>>() };
    calls.set(calls.get() + 1);
    // SAFETY: the host passes `argc` strings at `argv`; the test passes a non-empty list.
    let words = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    let first = words.first().and_then(|word| unsafe { word.as_str() }).unwrap_or("");
    if let Some(sink) = sink {
        // SAFETY: the sink is the host's collector, and `first` is valid for the call.
        unsafe { sink(sink_user, BbStr::new(first)) };
    }
    BB_OK
}

fn run_line(table: &BbCommands, line: &str) -> (BbStatus, String) {
    let mut out = String::new();
    let run = table.run.expect("bb.commands has run");
    // SAFETY: the table is the one the attachment holds; `out` outlives the call.
    let status = unsafe { run(table.ctx, BbStr::new(line), Some(into_string), (&mut out as *mut String).cast()) };
    (status, out)
}

#[test]
fn run_writes_the_output_to_the_sink() {
    let attached = attach_commands(Rc::new(FakeCommands::default()), "example.hello");
    let (status, out) = run_line(table_of(&attached), "drive");
    assert_eq!((status, out.as_str()), (BB_OK, "example.hello ran drive"));
}

#[test]
fn run_without_a_sink_is_an_argument_error() {
    let attached = attach_commands(Rc::new(FakeCommands::default()), "example.hello");
    let table: &BbCommands = table_of(&attached);
    let run = table.run.expect("run");
    // SAFETY: the table is live; the sink is null on purpose.
    let status = unsafe { run(table.ctx, BbStr::new("drive"), None, std::ptr::null_mut()) };
    assert_eq!(status, BB_ERR_ARG);
}

#[test]
fn a_panicking_backend_comes_back_as_a_status() {
    let backend = FakeCommands { panic_on_run: true, ..FakeCommands::default() };
    let attached = attach_commands(Rc::new(backend), "example.hello");
    assert_eq!(run_line(table_of(&attached), "drive").0, BB_ERR_PANIC);
}

#[test]
fn a_registered_command_calls_the_mod_callback_and_can_be_removed() {
    let backend = Rc::new(FakeCommands::default());
    let attached = attach_commands(backend.clone(), "example.hello");
    let table: &BbCommands = table_of(&attached);
    let calls = Cell::new(0_u32);
    let mut id = 0_u64;
    let register = table.register.expect("register");
    // SAFETY: `calls` and the callback outlive the test; the table is live.
    let status = unsafe {
        register(
            table.ctx,
            BbStr::new("echo"),
            BbStr::new("say a word"),
            (&calls as *const Cell<u32>).cast_mut().cast(),
            Some(echo),
            &mut id,
        )
    };
    assert_eq!((status, id), (BB_OK, make_handle(1, 1)));

    let handlers = backend.handlers.borrow();
    let (_, handler) = &handlers[0];
    assert_eq!(handler(&["hi", "there"]).as_deref(), Ok("hi"));
    assert_eq!(calls.get(), 1);
    drop(handlers);

    let unregister = table.unregister.expect("unregister");
    // SAFETY: the table is live.
    assert_eq!(unsafe { unregister(table.ctx, id) }, BB_OK);
    assert_eq!(*backend.removed.borrow(), [id]);
}

#[test]
fn a_long_command_name_is_refused() {
    let attached = attach_commands(Rc::new(FakeCommands::default()), "example.hello");
    let table: &BbCommands = table_of(&attached);
    let long = "x".repeat(MAX_NAME_BYTES + 1);
    let mut id = 0_u64;
    let register = table.register.expect("register");
    // SAFETY: the name lives for the call; the callback is never reached.
    let status =
        unsafe { register(table.ctx, BbStr::new(&long), BbStr::new(""), std::ptr::null_mut(), Some(echo), &mut id) };
    assert_eq!(status, BB_ERR_ARG);
}

#[test]
fn what_a_mod_left_registered_is_removed_when_it_unloads() {
    let backend = Rc::new(FakeCommands::default());
    let attached = attach_commands(backend.clone(), "example.hello");
    let table: &BbCommands = table_of(&attached);
    let register = table.register.expect("register");
    let mut kept = 0_u64;
    let mut gone = 0_u64;
    let calls = Cell::new(0_u32);
    // SAFETY: the callback's user pointer is a live cell for the test.
    unsafe {
        register(
            table.ctx,
            BbStr::new("a"),
            BbStr::EMPTY,
            (&calls as *const Cell<u32>).cast_mut().cast(),
            Some(echo),
            &mut kept,
        );
        register(
            table.ctx,
            BbStr::new("b"),
            BbStr::EMPTY,
            (&calls as *const Cell<u32>).cast_mut().cast(),
            Some(echo),
            &mut gone,
        );
        (table.unregister.expect("unregister"))(table.ctx, gone);
    }
    drop(attached);
    assert_eq!(*backend.removed.borrow(), [gone, kept]);
}

#[derive(Default)]
struct FakeInput {
    hotkeys: RefCell<Vec<(String, String, u64)>>,
    removed: RefCell<Vec<u64>>,
}

impl InputBackend for FakeInput {
    fn register_hotkey(
        &self,
        _owner: &str,
        name: &str,
        _label: &str,
        default_binding: &str,
        _on_press: Box<dyn Fn()>,
    ) -> BackendResult<u64> {
        let handle = make_handle(self.hotkeys.borrow().len() as u32 + 1, 1);
        self.hotkeys.borrow_mut().push((name.to_owned(), default_binding.to_owned(), handle));
        Ok(handle)
    }

    fn unregister(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(handle);
        Ok(())
    }
}

#[test]
fn a_hotkey_is_registered_with_its_default_binding() {
    let backend = Rc::new(FakeInput::default());
    let attached = attach_input(backend.clone(), "example.hello");
    let table: &BbInput = table_of(&attached);
    let register = table.register_hotkey.expect("register_hotkey");
    let mut id = 0_u64;
    unsafe extern "C" fn on_press(_user: *mut c_void) {}
    // SAFETY: the callback needs no user state; the table is live.
    let status = unsafe {
        register(
            table.ctx,
            BbStr::new("nitro"),
            BbStr::new("Nitro"),
            BbStr::new("F5"),
            std::ptr::null_mut(),
            Some(on_press),
            &mut id,
        )
    };
    assert_eq!((status, id), (BB_OK, make_handle(1, 1)));
    assert_eq!(backend.hotkeys.borrow()[0].0, "nitro");
    assert_eq!(backend.hotkeys.borrow()[0].1, "F5");
}

#[derive(Default)]
struct FakeTime {
    hour: Cell<f32>,
    frozen: Cell<bool>,
    seconds_per_hour: Cell<f32>,
}

impl TimeBackend for FakeTime {
    fn get(&self, _owner: &str) -> BackendResult<BbTimeOfDay> {
        Ok(BbTimeOfDay {
            hour: self.hour.get(),
            seconds_per_hour: self.seconds_per_hour.get(),
            frozen: u8::from(self.frozen.get()),
            lights_on: 0,
            _pad: [0; 2],
        })
    }

    fn set_hour(&self, _owner: &str, hour: f32) -> BackendResult<()> {
        self.hour.set(hour);
        Ok(())
    }

    fn set_day_length(&self, _owner: &str, seconds_per_hour: f32) -> BackendResult<()> {
        self.seconds_per_hour.set(seconds_per_hour);
        Ok(())
    }

    fn freeze(&self, _owner: &str, on: bool) -> BackendResult<()> {
        self.frozen.set(on);
        Ok(())
    }
}

#[test]
fn the_clock_round_trips_and_bad_values_are_refused() {
    let backend = Rc::new(FakeTime::default());
    let attached = attach_time(backend.clone(), "example.hello");
    let table: &BbTime = table_of(&attached);
    let (set_hour, set_day_length, freeze, get) = (
        table.set_hour.expect("set_hour"),
        table.set_day_length.expect("set_day_length"),
        table.freeze.expect("freeze"),
        table.get.expect("get"),
    );
    // SAFETY: the table is live; each argument is a plain value.
    unsafe {
        assert_eq!(set_hour(table.ctx, 7.5), BB_OK);
        assert_eq!(set_day_length(table.ctx, 30.0), BB_OK);
        assert_eq!(freeze(table.ctx, 1), BB_OK);
        assert_eq!(set_hour(table.ctx, f32::NAN), BB_ERR_ARG);
        assert_eq!(set_day_length(table.ctx, -1.0), BB_ERR_ARG);
    }
    let mut clock = BbTimeOfDay { hour: 0.0, seconds_per_hour: 0.0, frozen: 0, lights_on: 0, _pad: [0; 2] };
    // SAFETY: `clock` is valid for one write.
    assert_eq!(unsafe { get(table.ctx, &mut clock) }, BB_OK);
    assert_eq!((clock.hour, clock.seconds_per_hour, clock.frozen), (7.5, 30.0, 1));
    assert!(backend.frozen.get());
}

#[derive(Default)]
struct FakeMap {
    added: RefCell<Vec<(String, [f32; 3], bool)>>,
}

impl MapBackend for FakeMap {
    fn add(&self, _owner: &str, blip: &BlipSpec<'_>) -> BackendResult<u64> {
        self.added.borrow_mut().push((blip.style.to_owned(), blip.position, blip.active));
        Ok(make_handle(self.added.borrow().len() as u32, 1))
    }

    fn update(&self, _owner: &str, _handle: u64, _blip: &BlipSpec<'_>) -> BackendResult<()> {
        Ok(())
    }

    fn remove(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }
}

#[test]
fn a_blip_is_read_by_its_size_and_non_finite_positions_are_refused() {
    let backend = Rc::new(FakeMap::default());
    let attached = attach_map(backend.clone(), "example.hello");
    let table: &BbMap = table_of(&attached);
    let add = table.add.expect("add");
    let full = BbBlip {
        size: std::mem::size_of::<BbBlip>() as u32,
        active: 0,
        _pad: [0; 3],
        style: BbStr::new("marker"),
        position: [1.0, 2.0, 3.0],
    };
    let mut id = 0_u64;
    // SAFETY: the blip and its style live for the call; `id` is valid for one write.
    assert_eq!(unsafe { add(table.ctx, &full, &mut id) }, BB_OK);
    assert_eq!(backend.added.borrow()[0], ("marker".to_owned(), [1.0, 2.0, 3.0], false));

    let too_small = BbBlip { size: 20, ..full };
    // SAFETY: as above.
    assert_eq!(unsafe { add(table.ctx, &too_small, &mut id) }, BB_ERR_ARG);

    let not_finite = BbBlip { position: [f32::INFINITY, 0.0, 0.0], ..full };
    // SAFETY: as above.
    assert_eq!(unsafe { add(table.ctx, &not_finite, &mut id) }, BB_ERR_ARG);
    assert_eq!(backend.added.borrow().len(), 1);
}

#[test]
fn a_registry_offers_each_interface_only_at_its_version() {
    let mut registry = InterfaceRegistry::new();
    registry.add_time(Rc::new(FakeTime::default()));
    let offered: Vec<String> = registry.offers().into_iter().map(|offer| offer.id).collect();
    assert_eq!(offered, ["bb.time"]);
    assert!(registry.build("m", IFACE_TIME, 1, 0).is_some());
    assert!(registry.build("m", IFACE_TIME, 1, 1).is_none(), "minor 0 is older than the request");
    assert!(registry.build("m", IFACE_TIME, 2, 0).is_none(), "major differs");
    assert!(registry.build("m", IFACE_MAP, 1, 0).is_none(), "not offered");
}

/// A table an outside crate builds: its header first, then a value the test reads back.
#[repr(C)]
struct ExternalTable {
    head: BbIface,
    value: u32,
}

/// The id of the external interface the test offers.
const EXTERNAL_ID: &str = "test.external";

#[test]
fn an_external_provider_is_built_per_mod_and_queried_through_the_host() {
    let mut registry = InterfaceRegistry::new();
    registry.add_commands(Rc::new(FakeCommands::default()));
    registry.add_provider(EXTERNAL_ID, (1, 2), |ctx| {
        // The value is the length of the owner's id, so the test sees which mod the table was built for.
        let table = Box::new(ExternalTable {
            head: BbIface { size: core::mem::size_of::<ExternalTable>() as u32, major: 1, minor: 2 },
            value: ctx.owner().len() as u32,
        });
        let header = core::ptr::from_ref(&table.head);
        Attached::from_owned(EXTERNAL_ID, (1, 2), header, table)
    });
    let offered: Vec<String> = registry.offers().into_iter().map(|offer| offer.id).collect();
    assert_eq!(offered, [IFACE_COMMANDS, EXTERNAL_ID]);

    let offer = crate::host::HostOffer::new("testgame", "test");
    let host = HostTable::new("example.one", &offer, Rc::new(registry));
    let table = host.as_ptr();
    // SAFETY: the host table lives for the test, and `HostTable::new` sets its `ctx` and `query_interface`.
    let (ctx, query) = unsafe { ((*table).ctx, (*table).query_interface) };
    let query = query.expect("the host sets query_interface");

    // SAFETY: the id is valid for the call; the host builds the table on first use and keeps it while `host` lives.
    let header = unsafe { query(ctx, BbStr::new(EXTERNAL_ID), 1, 2) };
    assert!(!header.is_null(), "the external interface is offered at its version");
    // SAFETY: the header is the first field of the `ExternalTable` the provider boxed, kept alive by the host table.
    let value = unsafe { (*header.cast::<ExternalTable>()).value };
    assert_eq!(value, "example.one".len() as u32, "the provider saw the mod it was built for");

    // SAFETY: as above.
    let newer_minor = unsafe { query(ctx, BbStr::new(EXTERNAL_ID), 1, 3) };
    assert!(newer_minor.is_null(), "a minor older than the request is refused");
    // SAFETY: as above.
    let other_major = unsafe { query(ctx, BbStr::new(EXTERNAL_ID), 2, 0) };
    assert!(other_major.is_null(), "another major is refused");
    // SAFETY: as above.
    let absent = unsafe { query(ctx, BbStr::new("test.absent"), 1, 0) };
    assert!(absent.is_null(), "an interface nobody offers gives null");
}

#[test]
fn an_internal_provider_still_builds_its_table_for_its_owner() {
    let mut registry = InterfaceRegistry::new();
    registry.add_time(Rc::new(FakeTime::default()));
    let attached = registry.build("example.two", IFACE_TIME, 1, 0).expect("bb.time is offered");
    assert_eq!((attached.id(), attached.version()), (IFACE_TIME, IFACE_TIME_VERSION));
}
