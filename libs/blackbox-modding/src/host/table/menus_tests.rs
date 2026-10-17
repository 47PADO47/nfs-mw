//! The `bb.menus` table, called directly with a fake backend: a row's description is read and checked, a change reaches the
//! mod's callback, values round-trip, and the rows a mod leaves behind are removed when it unloads.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;

use super::menus::{MAX_CHOICES, MenuHandler, MenuKind, MenuRowSpec, MenusBackend};

use super::{Attached, attach_menus};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::registry::InterfaceRegistry;

/// The table an attachment holds. The header is the first field of every interface table.
fn table_of(attached: &Attached) -> &BbMenus {
    // SAFETY: the attachment owns a `BbMenus` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbMenus>() }
}

/// The callback a test row gives: it appends the value to the `RefCell<Vec<f32>>` behind `user`.
unsafe extern "C" fn record(user: *mut c_void, value: f32) {
    // SAFETY: the tests pass a live `RefCell<Vec<f32>>` as `user`.
    unsafe { (*user.cast::<RefCell<Vec<f32>>>()).borrow_mut().push(value) };
}

/// A row the backend was given, with its text copied out.
#[derive(Debug, Clone, PartialEq)]
struct Stored {
    kind: MenuKind,
    name: String,
    label: String,
    default: f32,
}

#[derive(Default)]
struct FakeMenus {
    rows: RefCell<Vec<Stored>>,
    handlers: RefCell<Vec<(u64, MenuHandler)>>,
    values: RefCell<Vec<(u64, f32)>>,
    removed: RefCell<Vec<u64>>,
}

impl MenusBackend for FakeMenus {
    fn register(&self, _owner: &str, row: &MenuRowSpec<'_>, on_change: MenuHandler) -> BackendResult<u64> {
        let handle = make_handle(self.rows.borrow().len() as u32 + 1, 1);
        let kind = row.kind.clone();
        let (name, label, default) = (row.name.to_owned(), row.label.to_owned(), row.default);
        self.rows.borrow_mut().push(Stored { kind, name, label, default });
        self.handlers.borrow_mut().push((handle, on_change));
        Ok(handle)
    }

    fn unregister(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(handle);
        Ok(())
    }

    fn get(&self, _owner: &str, handle: u64) -> BackendResult<f32> {
        let values = self.values.borrow();
        let found = values.iter().rev().find(|(known, _)| *known == handle);
        Ok(found.map_or(0.0, |(_, value)| *value))
    }

    fn set(&self, _owner: &str, handle: u64, value: f32) -> BackendResult<()> {
        self.values.borrow_mut().push((handle, value));
        Ok(())
    }
}

impl FakeMenus {
    /// What a player's change does: the row's callback runs with the value.
    fn change(&self, handle: u64, value: f32) {
        for (known, handler) in self.handlers.borrow().iter() {
            if *known == handle {
                handler(value);
            }
        }
    }
}

fn attached(backend: &Rc<FakeMenus>) -> Attached {
    attach_menus(backend.clone(), "example.a")
}

/// A row as the mod writes it, with the fields the kind does not use set to zero.
fn row(kind: u32) -> BbMenuRow {
    BbMenuRow {
        size: size_of::<BbMenuRow>() as u32,
        kind,
        name: BbStr::new("row"),
        label: BbStr::new("Row"),
        min: 0.0,
        max: 10.0,
        step: 1.0,
        default: 5.0,
        choices: ptr::null(),
        choice_count: 0,
    }
}

/// Register `row` through the table; the status and the handle.
fn register(
    table: &BbMenus,
    row: &BbMenuRow,
    user: *mut c_void,
    callback: Option<BbMenuCallbackFn>,
) -> (BbStatus, u64) {
    let mut id = 0;
    let register = table.register.expect("register is offered");
    // SAFETY: the table is the attachment's own, the row and the out pointer are valid for the call.
    let status = unsafe { register(table.ctx, row, user, callback, &mut id) };
    (status, id)
}

#[test]
fn a_slider_toggle_and_button_register_and_a_change_reaches_the_mod() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeMenus::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let seen = RefCell::new(Vec::<f32>::new());
    let user = (&seen as *const RefCell<Vec<f32>>).cast_mut().cast::<c_void>();

    let (status, slider) = register(table, &BbMenuRow { step: 2.0, ..row(BB_MENU_SLIDER) }, user, Some(record));
    assert_eq!(status, BB_OK);
    let (status, toggle) = register(table, &row(BB_MENU_TOGGLE), user, Some(record));
    assert_eq!(status, BB_OK);
    let (status, button) = register(table, &row(BB_MENU_BUTTON), user, Some(record));
    assert_eq!(status, BB_OK);
    assert!(slider != 0 && slider != toggle && toggle != button, "distinct non-zero handles");

    let kinds: Vec<MenuKind> = backend.rows.borrow().iter().map(|stored| stored.kind.clone()).collect();
    assert_eq!(kinds, [MenuKind::Slider { min: 0.0, max: 10.0, step: 2.0 }, MenuKind::Toggle, MenuKind::Button]);
    assert_eq!(backend.rows.borrow()[0].label, "Row");
    assert_eq!(backend.rows.borrow()[0].default, 5.0);
    backend.change(slider, 4.0);
    backend.change(button, 0.0);
    assert_eq!(*seen.borrow(), [4.0, 0.0], "the callback gets the value");
}

#[test]
fn a_list_reads_its_choices() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeMenus::default());
    let attach = attached(&backend);
    let texts = [BbStr::new("low"), BbStr::new("high")];
    let list = BbMenuRow { choices: texts.as_ptr(), choice_count: 2, ..row(BB_MENU_LIST) };
    let (status, _) = register(table_of(&attach), &list, ptr::null_mut(), Some(record));
    assert_eq!(status, BB_OK);
    assert_eq!(backend.rows.borrow()[0].kind, MenuKind::List { choices: vec!["low".into(), "high".into()] });
}

#[test]
fn a_bad_row_is_refused_before_the_backend_sees_it() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeMenus::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let refused = |row: &BbMenuRow| register(table, row, ptr::null_mut(), Some(record)).0;
    assert_eq!(refused(&row(9)), BB_ERR_ARG, "an unknown kind");
    assert_eq!(refused(&BbMenuRow { min: 5.0, max: 1.0, ..row(BB_MENU_SLIDER) }), BB_ERR_ARG, "min above max");
    assert_eq!(refused(&BbMenuRow { step: -1.0, ..row(BB_MENU_SLIDER) }), BB_ERR_ARG, "negative step");
    assert_eq!(refused(&BbMenuRow { default: f32::NAN, ..row(BB_MENU_TOGGLE) }), BB_ERR_ARG, "a non-finite default");
    assert_eq!(refused(&BbMenuRow { size: 8, ..row(BB_MENU_TOGGLE) }), BB_ERR_ARG, "a size too small for the row");
    assert_eq!(refused(&BbMenuRow { choice_count: 2, ..row(BB_MENU_LIST) }), BB_ERR_ARG, "a list without its array");
    let many = vec![BbStr::new("x"); MAX_CHOICES + 1];
    let long = BbMenuRow { choices: many.as_ptr(), choice_count: many.len() as u64, ..row(BB_MENU_LIST) };
    assert_eq!(refused(&long), BB_ERR_ARG, "more choices than the host takes");
    let no_callback = {
        let mut id = 0;
        let register = table.register.expect("register is offered");
        // SAFETY: a null callback is refused before any use; the other pointers are valid.
        unsafe { register(table.ctx, &row(BB_MENU_TOGGLE), ptr::null_mut(), None, &mut id) }
    };
    assert_eq!(no_callback, BB_ERR_ARG, "a row needs a callback");
    assert!(backend.rows.borrow().is_empty(), "nothing registered");
}

#[test]
fn get_and_set_reach_the_backend_and_a_null_out_is_refused() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeMenus::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let (_, toggle) = register(table, &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record));
    let (get, set) = (table.get.expect("get"), table.set.expect("set"));
    // SAFETY: the table is the attachment's own; the out pointer is valid for one write.
    let (status, value) = unsafe {
        let mut value = -1.0;
        let status = get(table.ctx, toggle, &mut value);
        (status, value)
    };
    assert_eq!((status, value), (BB_OK, 0.0));
    // SAFETY: as above.
    assert_eq!(unsafe { set(table.ctx, toggle, 1.0) }, BB_OK);
    // SAFETY: as above.
    assert_eq!(unsafe { set(table.ctx, toggle, f32::INFINITY) }, BB_ERR_ARG, "a non-finite value is refused");
    // SAFETY: as above; a null out is refused.
    assert_eq!(unsafe { get(table.ctx, toggle, ptr::null_mut()) }, BB_ERR_ARG);
    assert_eq!(*backend.values.borrow(), [(toggle, 1.0)]);
}

#[test]
fn rows_a_mod_left_registered_are_removed_when_it_unloads() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeMenus::default());
    let attach = attached(&backend);
    let (_, left) = register(table_of(&attach), &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record));
    let (_, gone) = register(table_of(&attach), &row(BB_MENU_BUTTON), ptr::null_mut(), Some(record));
    let unregister = table_of(&attach).unregister.expect("unregister");
    // SAFETY: the table is the attachment's own.
    assert_eq!(unsafe { unregister(table_of(&attach).ctx, gone) }, BB_OK);
    drop(attach);
    assert_eq!(*backend.removed.borrow(), [gone, left], "the explicit removal, then the unload's");
}

#[test]
fn a_registry_offers_the_menus_interface_at_its_version() {
    let _scope = callback_on_main();
    let backend: Rc<dyn MenusBackend> = Rc::new(FakeMenus::default());
    let mut registry = InterfaceRegistry::new();
    registry.add_menus(backend);
    assert!(registry.build("example.a", IFACE_MENUS, 1, 0).is_some());
    assert!(registry.build("example.a", IFACE_MENUS, 1, 1).is_none(), "a newer minor is not offered");
    assert!(registry.build("example.a", IFACE_MENUS, 2, 0).is_none(), "another major is not offered");
}

#[test]
fn a_call_outside_a_callback_on_the_main_thread_is_refused() {
    crate::host::gate::mark_main_thread();
    let backend = Rc::new(FakeMenus::default());
    let attached = attached(&backend);
    let (status, _) = register(table_of(&attached), &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record));
    assert_eq!(status, BB_ERR_NOT_IN_CALLBACK);
    assert!(backend.rows.borrow().is_empty(), "the backend never sees a refused row");
}

#[test]
fn a_call_from_another_thread_is_refused_even_inside_a_callback_scope() {
    let backend = Rc::new(FakeMenus::default());
    let attached = attached(&backend);
    // The address travels as a number, so the closure is `Send`; the other thread only reads the table.
    let table = table_of(&attached) as *const BbMenus as usize;
    let status = std::thread::spawn(move || {
        // SAFETY: `attached` outlives the thread, which joins below, and the table is only read.
        let table = unsafe { &*(table as *const BbMenus) };
        let _scope = crate::host::gate::enter();
        register(table, &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record)).0
    })
    .join()
    .expect("the worker thread does not panic");
    assert_eq!(status, BB_ERR_WRONG_THREAD);
    assert!(backend.rows.borrow().is_empty(), "the backend never sees a call from another thread");
}

#[test]
fn a_call_is_refused_again_once_its_callback_returns() {
    let backend = Rc::new(FakeMenus::default());
    let attached = attached(&backend);
    {
        let _scope = callback_on_main();
        let (status, _) = register(table_of(&attached), &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record));
        assert_eq!(status, BB_OK);
    }
    let (status, _) = register(table_of(&attached), &row(BB_MENU_TOGGLE), ptr::null_mut(), Some(record));
    assert_eq!(status, BB_ERR_NOT_IN_CALLBACK);
    assert_eq!(backend.rows.borrow().len(), 1, "only the call inside the callback reached the backend");
}
