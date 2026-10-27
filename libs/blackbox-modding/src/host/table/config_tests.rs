//! `bb.config` called directly with a fake backend: the mod's file and the game's table are merged into the sink, the
//! sources bits are right, the owner reaches the backend, and a broken source is `BB_ERR_HOST`.

use crate::host::test_support::{Scratch, callback_on_main};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::config::ConfigBackend;
use super::{Attached, attach_config};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbConfig {
    // SAFETY: the attachment owns a `BbConfig` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbConfig>() }
}

/// A game whose main config holds `section` for every mod, and remembers who asked.
struct FakeGame {
    section: Option<String>,
    asked: RefCell<Vec<String>>,
}

impl FakeGame {
    fn new(section: Option<&str>) -> Rc<Self> {
        Rc::new(FakeGame { section: section.map(str::to_owned), asked: RefCell::default() })
    }
}

impl ConfigBackend for FakeGame {
    fn main_section(&self, owner: &str) -> BackendResult<Option<String>> {
        self.asked.borrow_mut().push(owner.to_owned());
        Ok(self.section.clone())
    }
}

/// A sink that appends what the host writes to the `String` behind `user`.
unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: the test passes a live `RefCell<String>` as `user`, and the host's string is valid for the call.
    let out = unsafe { &*user.cast::<RefCell<String>>() };
    out.borrow_mut().push_str(unsafe { text.as_str() }.unwrap_or(""));
}

fn read_text(table: &BbConfig) -> (BbStatus, String) {
    let out = RefCell::new(String::new());
    let user = core::ptr::from_ref(&out).cast_mut().cast::<c_void>();
    let status = unsafe { (table.read.expect("read is offered"))(table.ctx, Some(collect), user) };
    (status, out.into_inner())
}

fn bits(table: &BbConfig) -> (BbStatus, u32) {
    let mut out = 0;
    let status = unsafe { (table.sources.expect("sources is offered"))(table.ctx, &mut out) };
    (status, out)
}

#[test]
fn the_file_and_the_games_table_are_merged_for_the_calling_mod() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-table");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"a = 'file'\nb = 'file'\n");
    let game = FakeGame::new(Some("b = 'main'\n"));
    let attached = attach_config(game.clone(), "example.a", dir, None);
    let table = table_of(&attached);
    let (status, text) = read_text(table);
    assert_eq!(status, BB_OK);
    assert_eq!(text.parse::<toml::Table>().unwrap(), "a = 'file'\nb = 'main'\n".parse::<toml::Table>().unwrap());
    assert_eq!(bits(table), (BB_OK, BB_CONFIG_FILE | BB_CONFIG_MAIN));
    assert_eq!(game.asked.borrow().as_slice(), ["example.a", "example.a"]);
}

#[test]
fn a_mod_with_no_source_reads_an_empty_text() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-none");
    let attached = attach_config(FakeGame::new(None), "example.a", scratch.folder("mod"), None);
    let table = table_of(&attached);
    assert_eq!(read_text(table), (BB_OK, String::new()));
    assert_eq!(bits(table), (BB_OK, 0));
}

#[test]
fn only_the_games_table_is_a_source_too() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-main");
    let attached = attach_config(FakeGame::new(Some("x = 1\n")), "example.a", scratch.folder("mod"), None);
    let table = table_of(&attached);
    assert_eq!(bits(table), (BB_OK, BB_CONFIG_MAIN));
    assert_eq!(read_text(table).1.trim(), "x = 1");
}

#[test]
fn the_manifests_file_replaces_config_toml() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-named");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"a = 'default'\n");
    scratch.write("mod/mine.toml", b"a = 'mine'\n");
    let attached = attach_config(FakeGame::new(None), "example.a", dir, Some("mine.toml".into()));
    assert_eq!(read_text(table_of(&attached)).1.trim(), "a = \"mine\"");
}

#[test]
fn a_broken_source_is_a_host_error_but_still_counts_as_present() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-broken");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"not = [toml");
    let attached = attach_config(FakeGame::new(None), "example.a", dir, None);
    let table = table_of(&attached);
    assert_eq!(read_text(table).0, BB_ERR_HOST);
    assert_eq!(bits(table), (BB_OK, BB_CONFIG_FILE));
}

#[test]
fn a_missing_sink_or_out_pointer_is_refused() {
    let _scope = callback_on_main();
    let scratch = Scratch::new("cfg-args");
    let attached = attach_config(FakeGame::new(None), "example.a", scratch.folder("mod"), None);
    let table = table_of(&attached);
    assert_eq!(unsafe { (table.read.unwrap())(table.ctx, None, core::ptr::null_mut()) }, BB_ERR_ARG);
    assert_eq!(unsafe { (table.sources.unwrap())(table.ctx, core::ptr::null_mut()) }, BB_ERR_ARG);
}
