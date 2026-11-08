//! The SDK's `Content` wrapper, round-tripped through the host's `bb.content` table: a mod's lazy files are made by its
//! `LazyFiles` when a host thread asks, errors and panics come back as statuses, and a mod's files stop when it unloads.

use core::ptr::NonNull;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use super::content as sdk_content;
use crate::abi::{BB_ERR_NOT_FOUND, BB_ERR_PANIC, BB_ERR_STALE, IFACE_CONTENT, IFACE_CONTENT_VERSION};
use crate::host::test_support::callback_on_main;
use crate::host::{Attached, BackendResult, ContentBackend, InterfaceRegistry, LazyFiles as HostLazyFiles};
use crate::sdk::ctx::Keep;
use crate::sdk::{Content, Error, Interface, LazyFiles, ModResult};

/// A game that keeps the lazy files of the mods it is given.
#[derive(Default)]
struct FakeBackend {
    files: RefCell<Vec<(String, Arc<dyn HostLazyFiles>)>>,
}

impl ContentBackend for FakeBackend {
    fn rescan(&self, _owner: &str) -> BackendResult<()> {
        Ok(())
    }

    fn provide_lazy_files(&self, owner: &str, files: Arc<dyn HostLazyFiles>) -> BackendResult<()> {
        self.files.borrow_mut().push((owner.to_owned(), files));
        Ok(())
    }
}

/// A mod's maker: repeats the key, refuses `nothing`, panics for `boom`.
struct Maker;

impl LazyFiles for Maker {
    fn read(&self, key: &str) -> ModResult<Vec<u8>> {
        match key {
            "nothing" => Err(Error::NotFound),
            "boom" => panic!("lazy files test: expected panic"),
            "empty" => Ok(Vec::new()),
            _ => Ok(key.repeat(2).into_bytes()),
        }
    }
}

fn content_over(backend: &Rc<FakeBackend>) -> (Content, Keep, Attached) {
    let mut registry = InterfaceRegistry::new();
    registry.add_content(backend.clone());
    let attached =
        registry.build("example.cars", IFACE_CONTENT, IFACE_CONTENT_VERSION.0, 0).expect("bb.content is offered");
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let header_value = unsafe { *header };
    let iface = Interface::new(NonNull::new(header.cast_mut()).expect("non-null header"), header_value);
    let keep: Keep = Rc::new(RefCell::new(Vec::new()));
    (Content::new(sdk_content::read(iface), keep.clone()), keep, attached)
}

#[test]
fn a_mod_makes_its_lazy_files_for_a_host_thread_and_its_errors_come_back_as_statuses() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeBackend::default());
    let (content, _keep, attached) = content_over(&backend);
    assert!(content.supports_lazy_files());
    content.provide_lazy_files(Maker).expect("the host takes them");
    let (owner, files) = backend.files.borrow_mut().pop().expect("the game was given the files");
    assert_eq!(owner, "example.cars");
    // The game reads from any thread, outside a mod callback.
    let from_thread = std::thread::scope(|scope| {
        let files = &files;
        scope.spawn(move || (files.read("ab"), files.read("empty"), files.read("nothing"), files.read("boom"))).join()
    })
    .expect("the reading thread did not panic: the SDK contains the maker's panic");
    assert_eq!(from_thread.0, Ok(b"abab".to_vec()));
    assert_eq!(from_thread.1, Ok(Vec::new()));
    assert_eq!(from_thread.2, Err(BB_ERR_NOT_FOUND));
    assert_eq!(from_thread.3, Err(BB_ERR_PANIC));
    drop(attached);
    assert_eq!(files.read("ab"), Err(BB_ERR_STALE), "the maker is not called after the mod unloads");
}
