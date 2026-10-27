//! The SDK's `Settings` wrapper, round-tripped through the host's `bb.settings` table: `add_option` reaches a fake game
//! backend with the right values, a change by the player runs the closure through the trampoline, a clash of kinds comes
//! back as an error, and `get` and `remove_option` reach the backend.

use core::ptr::NonNull;
use std::cell::RefCell;
use std::rc::Rc;

use super::HostTable;
use super::settings as sdk_settings;
use crate::abi::{ABI_MAJOR, ABI_MINOR, BB_ERR_EXISTS, BbHost, BbStr, IFACE_SETTINGS, IFACE_SETTINGS_VERSION};
use crate::host::test_support::callback_on_main;
use crate::host::{
    AddError, Attached, BackendResult, InterfaceRegistry, OptionDef, OptionKind, OptionSpec, OptionsModel,
    SettingHandler, SettingsBackend,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{Error, Interface, SettingId, SettingValue, Settings};

/// A backend over the real merge rules, as the game has it: the nitrous variant list holds the game's value.
struct FakeBackend {
    model: RefCell<OptionsModel>,
    handlers: RefCell<Vec<(u64, SettingHandler)>>,
    chosen: RefCell<String>,
}

impl Default for FakeBackend {
    fn default() -> Self {
        let game = OptionDef::game_list("nitrous_variant", "Nitrous variant", &[("original", "Original")]);
        Self {
            model: RefCell::new(OptionsModel::new(vec![game, OptionDef::fixed("turn_signals", OptionKind::Toggle)])),
            handlers: RefCell::default(),
            chosen: RefCell::new("original".into()),
        }
    }
}

impl SettingsBackend for FakeBackend {
    fn add_option(&self, owner: &str, spec: &OptionSpec<'_>, on_change: Option<SettingHandler>) -> BackendResult<u64> {
        let (handle, _) = self.model.borrow_mut().add_option(owner, spec).map_err(|error| match error {
            AddError::KindMismatch { .. } => BB_ERR_EXISTS,
            AddError::Invalid(_) => crate::abi::BB_ERR_ARG,
        })?;
        if let Some(handler) = on_change {
            self.handlers.borrow_mut().push((handle, handler));
        }
        Ok(handle)
    }

    fn remove_option(&self, owner: &str, handle: u64) -> BackendResult<()> {
        self.model.borrow_mut().remove(owner, handle);
        Ok(())
    }

    fn get(&self, _owner: &str, option: &str) -> BackendResult<String> {
        match option {
            "nitrous_variant" => Ok(self.chosen.borrow().clone()),
            _ => Err(crate::abi::BB_ERR_NOT_FOUND),
        }
    }
}

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
        mod_dir: BbStr::new(""),
        game_dir: BbStr::new(""),
    }
}

fn settings_over(backend: &Rc<FakeBackend>, host_struct: &mut BbHost) -> (Settings, Keep, Attached) {
    let mut registry = InterfaceRegistry::new();
    registry.add_settings(backend.clone());
    let attached =
        registry.build("example.mw", IFACE_SETTINGS, IFACE_SETTINGS_VERSION.0, 0).expect("bb.settings is offered");
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let header_value = unsafe { *header };
    let iface = Interface::new(NonNull::new(header.cast_mut()).expect("non-null header"), header_value);
    // SAFETY: `host_struct` outlives the settings, which the test uses only while it is in scope.
    let host = unsafe { HostTable::new(NonNull::from(host_struct)) };
    let keep: Keep = Rc::new(RefCell::new(Vec::new()));
    (Settings::new(sdk_settings::read(iface), host, keep.clone()), keep, attached)
}

#[test]
fn a_mod_adds_a_value_to_the_games_list_and_it_shows() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (settings, _keep, _attached) = settings_over(&backend, &mut host_struct);
    assert!(backend.model.borrow().visible().is_empty(), "before the mod: one value, hidden");
    settings
        .add_option(
            "nitrous_variant",
            "Nitrous variant",
            &[SettingValue { id: "mw2012", label: "NFS Most Wanted 2012" }],
        )
        .expect("merged");
    let model = backend.model.borrow();
    let shown = model.visible();
    assert_eq!(shown.len(), 1);
    let ids: Vec<&str> = shown[0].values.iter().map(|value| value.id.as_str()).collect();
    assert_eq!(ids, ["original", "mw2012"]);
}

#[test]
fn a_clash_of_kinds_is_an_error_and_changes_nothing() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (settings, _keep, _attached) = settings_over(&backend, &mut host_struct);
    let clash = settings.add_toggle("nitrous_variant", "x", true, |_ctx, _on| {});
    assert_eq!(clash, Err(Error::Exists));
    assert_eq!(backend.model.borrow().get("nitrous_variant").unwrap().values.len(), 1);
}

#[test]
fn a_change_runs_the_closure_with_the_value_id() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (settings, _keep, _attached) = settings_over(&backend, &mut host_struct);
    let seen = Rc::new(RefCell::new(Vec::<String>::new()));
    let log = seen.clone();
    let id = settings
        .add_option_with(
            "flame_color",
            "Flame colour",
            &[SettingValue { id: "blue", label: "Blue" }],
            move |_ctx, value| log.borrow_mut().push(value.to_owned()),
        )
        .expect("added");
    let handle = id.raw();
    for (known, handler) in backend.handlers.borrow().iter() {
        if *known == handle {
            handler("blue");
        }
    }
    assert_eq!(*seen.borrow(), ["blue"]);
    let toggled = Rc::new(RefCell::new(Vec::<bool>::new()));
    let log = toggled.clone();
    settings.add_toggle("extra", "Extra", false, move |_ctx, on| log.borrow_mut().push(on)).expect("toggle added");
    for (_, handler) in backend.handlers.borrow().iter() {
        handler("on");
    }
    assert_eq!(*toggled.borrow(), [true]);
}

#[test]
fn get_and_remove_reach_the_backend() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let mut host_struct = fake_host();
    let (settings, _keep, _attached) = settings_over(&backend, &mut host_struct);
    assert_eq!(settings.get("nitrous_variant").as_deref(), Ok("original"));
    assert_eq!(settings.get("missing"), Err(Error::NotFound));
    let id = settings.add_option("nitrous_variant", "x", &[SettingValue { id: "mw2012", label: "M" }]).expect("merged");
    assert_ne!(id, SettingId::from_raw(0));
    settings.remove_option(id).expect("removed");
    assert_eq!(backend.model.borrow().get("nitrous_variant").unwrap().values.len(), 1);
}
