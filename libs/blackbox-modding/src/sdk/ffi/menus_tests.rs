//! The SDK's `Menus` wrapper, round-tripped through the host's `bb.menus` table: each `add` method reaches a fake game
//! backend as the right row, a change by the player runs the closure through the trampoline, and get, set and
//! unregister reach the backend. The host side runs in a callback scope, as the loader would run it.

use core::ptr::NonNull;
use std::cell::RefCell;
use std::rc::Rc;

use super::HostTable;
use super::menus as sdk_menus;
use crate::abi::{ABI_MAJOR, ABI_MINOR, BbHost, BbStr, IFACE_MENUS, IFACE_MENUS_VERSION, make_handle};
use crate::host::test_support::callback_on_main;
use crate::host::{Attached, BackendResult, InterfaceRegistry, MenuHandler, MenuKind, MenuRowSpec, MenusBackend};
use crate::sdk::ctx::Keep;
use crate::sdk::{Interface, MenuId, Menus, SliderRange};

/// A row the backend was given, with its text copied out.
#[derive(Debug, Clone, PartialEq)]
struct Stored {
    kind: MenuKind,
    name: String,
    label: String,
    default: f32,
}

#[derive(Default)]
struct FakeBackend {
    rows: RefCell<Vec<Stored>>,
    handlers: RefCell<Vec<(u64, MenuHandler)>>,
    values: RefCell<Vec<(u64, f32)>>,
    removed: RefCell<Vec<u64>>,
}

impl MenusBackend for FakeBackend {
    fn register(&self, _owner: &str, row: &MenuRowSpec<'_>, on_change: MenuHandler) -> BackendResult<u64> {
        let handle = make_handle(self.rows.borrow().len() as u32 + 1, 1);
        let stored = Stored {
            kind: row.kind.clone(),
            name: row.name.to_owned(),
            label: row.label.to_owned(),
            default: row.default,
        };
        self.rows.borrow_mut().push(stored);
        self.handlers.borrow_mut().push((handle, on_change));
        Ok(handle)
    }

    fn unregister(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(handle);
        Ok(())
    }

    fn get(&self, _owner: &str, handle: u64) -> BackendResult<f32> {
        let values = self.values.borrow();
        Ok(values.iter().rev().find(|(known, _)| *known == handle).map_or(0.0, |(_, value)| *value))
    }

    fn set(&self, _owner: &str, handle: u64, value: f32) -> BackendResult<()> {
        self.values.borrow_mut().push((handle, value));
        Ok(())
    }
}

impl FakeBackend {
    /// What the player does: the row's closure in the host runs with `value`.
    fn change(&self, handle: u64, value: f32) {
        for (known, handler) in self.handlers.borrow().iter() {
            if *known == handle {
                handler(value);
            }
        }
    }

    /// The handle of the `index`-th row the backend received.
    fn handle_of(&self, index: usize) -> u64 {
        self.handlers.borrow()[index].0
    }
}

/// A host table with no logger and no interfaces of its own; the SDK reads only what the menus need from it.
fn fake_host() -> BbHost {
    BbHost {
        size: core::mem::size_of::<BbHost>() as u32,
        abi_major: ABI_MAJOR,
        abi_minor: ABI_MINOR,
        ctx: core::ptr::null_mut(),
        game_id: BbStr::new("test"),
        game_version: BbStr::new("0"),
        log: None,
        query_interface: None,
    }
}

/// The SDK's `Menus` over a registry that offers `backend` as `bb.menus`, for mod `example.menus`. The attachment is
/// returned too: it owns the host's table, so the caller keeps it alive for the test.
fn menus_over(backend: &Rc<FakeBackend>, host_struct: &mut BbHost) -> (Menus, Keep, Attached) {
    let mut registry = InterfaceRegistry::new();
    registry.add_menus(backend.clone());
    let attached = registry.build("example.menus", IFACE_MENUS, IFACE_MENUS_VERSION.0, 0).expect("bb.menus is offered");
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let header_value = unsafe { *header };
    let iface = Interface::new(NonNull::new(header.cast_mut()).expect("non-null header"), header_value);
    // SAFETY: `host_struct` outlives the menus, which the test uses only while it is in scope.
    let host = unsafe { HostTable::new(NonNull::from(host_struct)) };
    let keep: Keep = Rc::new(RefCell::new(Vec::new()));
    (Menus::new(sdk_menus::read(iface), host, keep.clone()), keep, attached)
}

#[test]
fn a_toggle_reaches_the_backend_and_a_change_runs_the_closure_with_its_state() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (menus, _keep, _attached) = menus_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let id = menus.toggle("gate", "Gate", true, move |_ctx, on| log.borrow_mut().push(on)).expect("row added");
    let stored = backend.rows.borrow()[0].clone();
    assert_eq!(stored.kind, MenuKind::Toggle);
    assert_eq!((stored.name.as_str(), stored.label.as_str(), stored.default), ("gate", "Gate", 1.0));
    backend.change(backend.handle_of(0), 0.0);
    assert_eq!(*seen.borrow(), vec![false]);
    assert_eq!(id, MenuId::from_raw(backend.handle_of(0)));
}

#[test]
fn a_slider_passes_its_range_and_the_new_value() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (menus, _keep, _attached) = menus_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let range = SliderRange { min: 10.0, max: 600.0, step: 10.0 };
    menus.slider("spd", "Speed", range, 60.0, move |_ctx, value| log.borrow_mut().push(value)).expect("row added");
    let stored = backend.rows.borrow()[0].clone();
    assert_eq!(stored.kind, MenuKind::Slider { min: 10.0, max: 600.0, step: 10.0 });
    assert_eq!(stored.default, 60.0);
    backend.change(backend.handle_of(0), 120.0);
    assert_eq!(*seen.borrow(), vec![120.0]);
}

#[test]
fn a_list_passes_its_choices_and_the_chosen_index() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (menus, _keep, _attached) = menus_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let choices = ["dry", "wet", "snow"];
    menus.list("wx", "Weather", &choices, 1, move |_ctx, index| log.borrow_mut().push(index)).expect("row added");
    let stored = backend.rows.borrow()[0].clone();
    assert_eq!(stored.kind, MenuKind::List { choices: vec!["dry".into(), "wet".into(), "snow".into()] });
    assert_eq!(stored.default, 1.0);
    backend.change(backend.handle_of(0), 2.0);
    assert_eq!(*seen.borrow(), vec![2]);
}

#[test]
fn a_button_runs_its_closure_with_no_value() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (menus, _keep, _attached) = menus_over(&backend, &mut host_struct);
    let presses = Rc::new(RefCell::new(0_u32));
    let count = presses.clone();
    menus.button("go", "Go", move |_ctx| *count.borrow_mut() += 1).expect("row added");
    assert_eq!(backend.rows.borrow()[0].kind, MenuKind::Button);
    backend.change(backend.handle_of(0), 0.0);
    backend.change(backend.handle_of(0), 0.0);
    assert_eq!(*presses.borrow(), 2);
}

#[test]
fn get_set_and_unregister_reach_the_backend() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (menus, _keep, _attached) = menus_over(&backend, &mut host_struct);
    let range = SliderRange { min: 0.0, max: 1.0, step: 0.1 };
    let id = menus.slider("spd", "Speed", range, 0.5, |_ctx, _value| {}).expect("row added");
    menus.set(id, 0.25).expect("set");
    assert_eq!(menus.get(id).expect("get"), 0.25);
    menus.unregister(id).expect("unregister");
    assert_eq!(*backend.removed.borrow(), vec![id.raw()]);
}
