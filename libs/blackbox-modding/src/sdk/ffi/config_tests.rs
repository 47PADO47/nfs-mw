//! The SDK's `ModConfig` wrapper round-tripped through the host's `bb.config` table: the merged text reaches the mod,
//! the sources bits become the two questions, and a broken source is an error the mod sees.

use core::ptr::NonNull;
use std::rc::Rc;

use super::config as sdk_config;
use crate::abi::{BbIface, IFACE_CONFIG, IFACE_CONFIG_VERSION};
use crate::host::registry::{HostCtx, ModPlace};
use crate::host::test_support::{Scratch, callback_on_main};
use crate::host::{BackendResult, ConfigBackend, InterfaceRegistry};
use crate::sdk::{Error, Interface, ModConfig};

/// A game whose main config holds `section` for every mod.
struct FakeGame(Option<&'static str>);

impl ConfigBackend for FakeGame {
    fn main_section(&self, _owner: &str) -> BackendResult<Option<String>> {
        Ok(self.0.map(str::to_owned))
    }
}

/// The wrapper over the table a registry builds for a mod living in `dir`. The registry offers the table through a
/// provider so the test controls the mod's folder, as the loader does.
fn config_over(
    game: FakeGame,
    dir: std::path::PathBuf,
    file: Option<&'static str>,
) -> (ModConfig, crate::host::Attached) {
    let game: Rc<dyn ConfigBackend> = Rc::new(game);
    let mut registry = InterfaceRegistry::new();
    registry.add_config(game);
    let place = ModPlace { dir, config_file: file.map(str::to_owned) };
    let attached = registry
        .build_in(&HostCtx::new("example.mw", &place), IFACE_CONFIG, IFACE_CONFIG_VERSION.0, 0)
        .expect("bb.config is offered");
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let header_value: BbIface = unsafe { *header };
    let iface = Interface::new(NonNull::new(header.cast_mut()).expect("non-null header"), header_value);
    (ModConfig::new(sdk_config::read(iface)), attached)
}

#[test]
fn the_mod_reads_the_merged_text_and_the_sources() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("sdk-config");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"source = 'file'\nrole = 'player'\n");
    let (config, _attached) = config_over(FakeGame(Some("source = 'main'\n")), dir, None);
    let table: toml::Table = config.text().expect("text").parse().expect("valid TOML");
    assert_eq!(table.get("source").and_then(toml::Value::as_str), Some("main"));
    assert_eq!(table.get("role").and_then(toml::Value::as_str), Some("player"));
    assert!(config.has_file() && config.has_main_section());
}

#[test]
fn a_mod_with_only_the_games_table_has_no_file() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("sdk-config-main");
    let (config, _attached) = config_over(FakeGame(Some("x = 1\n")), scratch.folder("mod"), None);
    assert!(!config.has_file() && config.has_main_section());
    assert_eq!(config.text().unwrap().trim(), "x = 1");
}

#[test]
fn a_mod_with_nothing_reads_an_empty_text() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("sdk-config-none");
    let (config, _attached) = config_over(FakeGame(None), scratch.folder("mod"), None);
    assert_eq!(config.text().unwrap(), "");
    assert!(!config.has_file() && !config.has_main_section());
}

#[test]
fn a_broken_file_is_a_host_error() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("sdk-config-broken");
    let dir = scratch.folder("mod");
    scratch.write("mod/mine.toml", b"not = [toml");
    let (config, _attached) = config_over(FakeGame(None), dir, Some("mine.toml"));
    assert_eq!(config.text(), Err(Error::Host));
    assert!(config.has_file(), "the file is there even if it is broken");
}
