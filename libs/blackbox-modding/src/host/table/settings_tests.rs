//! The `bb.settings` table, called directly with a fake backend: an option's description is read and checked, a change
//! reaches the mod's callback, the chosen value comes back through the sink, and what a mod leaves behind is removed when it
//! unloads.

use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;

use super::settings::{SettingHandler, SettingsBackend};
use super::{Attached, attach_settings};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::options::{OptionKind, OptionSpec};
use crate::host::registry::InterfaceRegistry;
use crate::host::test_support::callback_on_main;

fn table_of(attached: &Attached) -> &BbSettings {
    // SAFETY: the attachment owns a `BbSettings` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbSettings>() }
}

unsafe extern "C" fn record(user: *mut c_void, value: BbStr) {
    // SAFETY: the tests pass a live `RefCell<Vec<String>>` as `user` and a string valid for the call.
    unsafe { (*user.cast::<RefCell<Vec<String>>>()).borrow_mut().push(value.as_str().unwrap().to_owned()) };
}

unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: as for `record`.
    unsafe { (*user.cast::<RefCell<String>>()).borrow_mut().push_str(text.as_str().unwrap()) };
}

#[derive(Debug, Clone, PartialEq)]
struct Stored {
    id: String,
    kind: OptionKind,
    label: String,
    values: Vec<(String, String)>,
    default_on: bool,
}

#[derive(Default)]
struct Fake {
    options: RefCell<Vec<Stored>>,
    handlers: RefCell<Vec<(u64, SettingHandler)>>,
    removed: RefCell<Vec<u64>>,
}

impl SettingsBackend for Fake {
    fn add_option(&self, _owner: &str, spec: &OptionSpec<'_>, on_change: Option<SettingHandler>) -> BackendResult<u64> {
        let handle = self.options.borrow().len() as u64 + 1;
        self.options.borrow_mut().push(Stored {
            id: spec.id.to_owned(),
            kind: spec.kind,
            label: spec.label.to_owned(),
            values: spec.values.iter().map(|v| (v.id.to_owned(), v.label.to_owned())).collect(),
            default_on: spec.default_on,
        });
        if let Some(handler) = on_change {
            self.handlers.borrow_mut().push((handle, handler));
        }
        Ok(handle)
    }

    fn remove_option(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(handle);
        Ok(())
    }

    fn get(&self, _owner: &str, option: &str) -> BackendResult<String> {
        match option {
            "nitrous_variant" => Ok("original".into()),
            _ => Err(BB_ERR_NOT_FOUND),
        }
    }
}

fn option<'a>(kind: u32, id: &'a str, values: &'a [BbSettingValue]) -> BbSettingOption {
    BbSettingOption {
        size: size_of::<BbSettingOption>() as u32,
        kind,
        menu: BB_SETTING_MENU_GAMEPLAY,
        reserved: 0,
        id: BbStr::new(id),
        label: BbStr::new("Label"),
        values: if values.is_empty() { ptr::null() } else { values.as_ptr() },
        value_count: values.len() as u64,
        default_on: 0,
        pad: 0,
    }
}

fn add(
    table: &BbSettings,
    option: &BbSettingOption,
    user: *mut c_void,
    callback: Option<BbSettingCallbackFn>,
) -> (BbStatus, u64) {
    let mut id = 0;
    let add_option = table.add_option.expect("add_option is offered");
    // SAFETY: the table is the attachment's own, the option and the out pointer are valid for the call.
    let status = unsafe { add_option(table.ctx, option, user, callback, &mut id) };
    (status, id)
}

fn value<'a>(id: &'a str, label: &'a str) -> BbSettingValue {
    BbSettingValue { id: BbStr::new(id), label: BbStr::new(label) }
}

#[test]
fn a_list_option_is_read_and_a_change_reaches_the_mod() {
    let _scope = callback_on_main();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend.clone(), "example.a");
    let values = [value("mw2012", "NFS Most Wanted 2012")];
    let seen = RefCell::new(Vec::<String>::new());
    let user = (&seen as *const RefCell<Vec<String>>).cast_mut().cast::<c_void>();
    let (status, handle) =
        add(table_of(&attached), &option(BB_SETTING_LIST, "nitrous_variant", &values), user, Some(record));
    assert_eq!(status, BB_OK);
    assert_ne!(handle, 0);
    assert_eq!(
        backend.options.borrow()[0],
        Stored {
            id: "nitrous_variant".into(),
            kind: OptionKind::List,
            label: "Label".into(),
            values: vec![("mw2012".into(), "NFS Most Wanted 2012".into())],
            default_on: false,
        }
    );
    for (_, handler) in backend.handlers.borrow().iter() {
        handler("mw2012");
    }
    assert_eq!(*seen.borrow(), ["mw2012"]);
}

#[test]
fn a_toggle_needs_no_values_and_the_callback_is_optional() {
    let _scope = callback_on_main();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend.clone(), "example.a");
    let toggle = BbSettingOption { default_on: 1, ..option(BB_SETTING_TOGGLE, "extra_hud", &[]) };
    let (status, _) = add(table_of(&attached), &toggle, ptr::null_mut(), None);
    assert_eq!(status, BB_OK);
    let stored = backend.options.borrow()[0].clone();
    assert_eq!((stored.kind, stored.default_on), (OptionKind::Toggle, true));
    assert!(backend.handlers.borrow().is_empty());
}

#[test]
fn a_bad_option_is_refused_before_the_backend_sees_it() {
    let _scope = callback_on_main();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend.clone(), "example.a");
    let table = table_of(&attached);
    let values = [value("a", "A")];
    let refused = |option: &BbSettingOption| add(table, option, ptr::null_mut(), None).0;
    assert_eq!(refused(&option(9, "x", &values)), BB_ERR_ARG, "an unknown kind");
    assert_eq!(
        refused(&BbSettingOption { menu: 7, ..option(BB_SETTING_LIST, "x", &values) }),
        BB_ERR_ARG,
        "an unknown menu"
    );
    assert_eq!(
        refused(&BbSettingOption { size: 8, ..option(BB_SETTING_LIST, "x", &values) }),
        BB_ERR_ARG,
        "a short struct"
    );
    assert_eq!(
        refused(&BbSettingOption { value_count: 2, ..option(BB_SETTING_LIST, "x", &[]) }),
        BB_ERR_ARG,
        "a count without the array"
    );
    let many = vec![value("a", "A"); 65];
    assert_eq!(refused(&option(BB_SETTING_LIST, "x", &many)), BB_ERR_ARG, "more values than the host takes");
    let null_out = {
        let add_option = table.add_option.expect("add_option");
        // SAFETY: a null out pointer is refused before any write; the other pointers are valid.
        unsafe { add_option(table.ctx, &option(BB_SETTING_LIST, "x", &values), ptr::null_mut(), None, ptr::null_mut()) }
    };
    assert_eq!(null_out, BB_ERR_ARG);
    assert!(backend.options.borrow().is_empty(), "nothing registered");
}

#[test]
fn get_writes_the_value_through_the_sink_and_an_unknown_option_is_not_found() {
    let _scope = callback_on_main();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend, "example.a");
    let table = table_of(&attached);
    let get = table.get.expect("get");
    let out = RefCell::new(String::new());
    let user = (&out as *const RefCell<String>).cast_mut().cast::<c_void>();
    // SAFETY: the table is the attachment's own; the sink and `user` are valid for the call.
    let status = unsafe { get(table.ctx, BbStr::new("nitrous_variant"), Some(collect), user) };
    assert_eq!((status, out.borrow().as_str()), (BB_OK, "original"));
    // SAFETY: as above.
    assert_eq!(unsafe { get(table.ctx, BbStr::new("nope"), Some(collect), user) }, BB_ERR_NOT_FOUND);
    // SAFETY: as above; a missing sink is refused.
    assert_eq!(unsafe { get(table.ctx, BbStr::new("nitrous_variant"), None, user) }, BB_ERR_ARG);
}

#[test]
fn registrations_a_mod_left_are_removed_when_it_unloads() {
    let _scope = callback_on_main();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend.clone(), "example.a");
    let values = [value("a", "A")];
    let (_, left) = add(table_of(&attached), &option(BB_SETTING_LIST, "one", &values), ptr::null_mut(), None);
    let (_, gone) = add(table_of(&attached), &option(BB_SETTING_LIST, "two", &values), ptr::null_mut(), None);
    let remove = table_of(&attached).remove_option.expect("remove_option");
    // SAFETY: the table is the attachment's own.
    assert_eq!(unsafe { remove(table_of(&attached).ctx, gone) }, BB_OK);
    drop(attached);
    assert_eq!(*backend.removed.borrow(), [gone, left]);
}

#[test]
fn a_registry_offers_the_settings_interface_at_its_version() {
    let _scope = callback_on_main();
    let backend: Rc<dyn SettingsBackend> = Rc::new(Fake::default());
    let mut registry = InterfaceRegistry::new();
    registry.add_settings(backend);
    assert!(registry.build("example.a", IFACE_SETTINGS, 1, 0).is_some());
    assert!(registry.build("example.a", IFACE_SETTINGS, 1, 1).is_none());
    assert!(registry.build("example.a", IFACE_SETTINGS, 2, 0).is_none());
}

#[test]
fn a_call_outside_a_callback_is_refused() {
    crate::host::gate::mark_main_thread();
    let backend = Rc::new(Fake::default());
    let attached = attach_settings(backend.clone(), "example.a");
    let values = [value("a", "A")];
    let (status, _) = add(table_of(&attached), &option(BB_SETTING_LIST, "x", &values), ptr::null_mut(), None);
    assert_eq!(status, BB_ERR_NOT_IN_CALLBACK);
    assert!(backend.options.borrow().is_empty());
}
