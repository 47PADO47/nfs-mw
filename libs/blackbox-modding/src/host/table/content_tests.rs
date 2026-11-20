//! `bb.content` through its table: the backend sees the calling mod, and a mod's lazy files are read through the function it
//! gave until it unloads.

use core::ffi::c_void;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::BackendResult;
use super::content::{ContentBackend, LazyFiles, attach};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_STALE, BB_ERR_UNSUPPORTED, BB_OK, BbContent,
    BbContentBytesSinkFn, BbStatus, BbStr,
};

#[derive(Default)]
struct Fake {
    owners: RefCell<Vec<String>>,
    files: RefCell<Vec<(String, Arc<dyn LazyFiles>)>>,
}

impl ContentBackend for Fake {
    fn rescan(&self, owner: &str) -> BackendResult<()> {
        self.owners.borrow_mut().push(owner.to_owned());
        Ok(())
    }

    fn provide_lazy_files(&self, owner: &str, files: Arc<dyn LazyFiles>) -> BackendResult<()> {
        self.files.borrow_mut().push((owner.to_owned(), files));
        Ok(())
    }
}

/// A backend written before lazy files: it implements `rescan` only.
struct Old;

impl ContentBackend for Old {
    fn rescan(&self, _owner: &str) -> BackendResult<()> {
        Ok(())
    }
}

/// The mod's side: makes the key twice over, counts its calls through `user`, refuses `nothing`, hands over nothing for `silent`.
unsafe extern "C" fn make(
    user: *mut c_void,
    key: BbStr,
    sink: Option<BbContentBytesSinkFn>,
    sink_ctx: *mut c_void,
) -> BbStatus {
    // SAFETY: the tests pass the address of an `AtomicUsize` that outlives every call.
    let counter = unsafe { &*user.cast::<AtomicUsize>() };
    counter.fetch_add(1, Ordering::SeqCst);
    // SAFETY: the host passes a string valid for the call.
    let key = unsafe { key.as_str() }.unwrap_or("");
    if key == "nothing" {
        return BB_ERR_NOT_FOUND;
    }
    if key == "silent" {
        return BB_OK;
    }
    let bytes = key.repeat(2).into_bytes();
    let Some(sink) = sink else { return BB_ERR_ARG };
    // SAFETY: the sink is the host's, valid for this call, and `bytes` outlives it.
    unsafe { sink(sink_ctx, bytes.as_ptr(), bytes.len() as u64) };
    BB_OK
}

#[test]
fn a_rescan_reaches_the_backend_with_the_calling_mod() {
    let fake = Rc::new(Fake::default());
    let attached = attach(fake.clone(), "example.cars");
    // SAFETY: the header is the first field of the `BbContent` this module boxed, which `attached` keeps alive.
    let table = unsafe { &*attached.header().cast::<BbContent>() };
    let rescan = table.rescan.expect("the table has rescan");
    crate::host::gate::mark_main_thread();
    let _scope = crate::host::gate::enter();
    // SAFETY: `ctx` is the table's own slot pointer.
    assert_eq!(unsafe { rescan(table.ctx) }, BB_OK);
    assert_eq!(*fake.owners.borrow(), ["example.cars"]);
}

#[test]
fn a_mods_lazy_files_are_read_through_its_function_until_it_unloads() {
    let fake = Rc::new(Fake::default());
    let attached = attach(fake.clone(), "example.cars");
    // SAFETY: the header is the first field of the `BbContent` this module boxed, which `attached` keeps alive.
    let table = unsafe { &*attached.header().cast::<BbContent>() };
    assert_eq!(table.head.minor, 2, "lazy files came with 1.1, the capabilities and the report with 1.2");
    let provide = table.provide_lazy_files.expect("the table has provide_lazy_files");
    let calls = AtomicUsize::new(0);
    crate::host::gate::mark_main_thread();
    let _scope = crate::host::gate::enter();
    // SAFETY: `ctx` is the table's own slot pointer; `calls` outlives every read below.
    let status = unsafe { provide(table.ctx, Some(make), (&calls as *const AtomicUsize).cast_mut().cast()) };
    assert_eq!(status, BB_OK);
    let (owner, files) = fake.files.borrow_mut().pop().expect("the backend was given the files");
    assert_eq!(owner, "example.cars");
    assert_eq!(files.read("ab"), Ok(b"abab".to_vec()));
    assert_eq!(files.read("nothing"), Err(BB_ERR_NOT_FOUND), "the mod's status comes back");
    assert_eq!(files.read("silent"), Err(BB_ERR_HOST), "a mod that returns OK and hands over nothing made no file");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    drop(attached);
    assert_eq!(files.read("ab"), Err(BB_ERR_STALE), "after the mod unloads the function is never called");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn lazy_files_need_a_function_and_a_backend_that_takes_them() {
    let fake = Rc::new(Fake::default());
    let attached = attach(fake.clone(), "example.cars");
    // SAFETY: the header is the first field of the `BbContent` this module boxed, which `attached` keeps alive.
    let table = unsafe { &*attached.header().cast::<BbContent>() };
    let provide = table.provide_lazy_files.expect("the table has provide_lazy_files");
    crate::host::gate::mark_main_thread();
    let _scope = crate::host::gate::enter();
    // SAFETY: `ctx` is the table's own slot pointer; no function is passed.
    assert_eq!(unsafe { provide(table.ctx, None, core::ptr::null_mut()) }, BB_ERR_ARG);
    assert!(fake.files.borrow().is_empty());
    let old = attach(Rc::new(Old), "example.cars");
    // SAFETY: as above, for the other table.
    let old_table = unsafe { &*old.header().cast::<BbContent>() };
    let provide = old_table.provide_lazy_files.expect("every table has the entry");
    // SAFETY: `ctx` is that table's own slot pointer; `make` is never called by this backend.
    let status = unsafe { provide(old_table.ctx, Some(make), core::ptr::null_mut()) };
    assert_eq!(status, BB_ERR_UNSUPPORTED, "a game without lazy files says so");
}

unsafe extern "C" fn take_text(sink: *mut core::ffi::c_void, bytes: *const u8, len: u64) {
    // SAFETY: `sink` is the `Vec<u8>` the test passed; the host gives `len` readable bytes.
    let out = unsafe { &mut *sink.cast::<Vec<u8>>() };
    out.extend_from_slice(unsafe { core::slice::from_raw_parts(bytes, len as usize) });
}

#[test]
fn the_capabilities_and_the_report_of_a_backend_come_through_and_an_old_backend_says_none() {
    let fake = Rc::new(Fake::default());
    let attached = attach(fake.clone(), "example.cars");
    // SAFETY: the header is the first field of the `BbContent` this module boxed, which `attached` keeps alive.
    let table = unsafe { &*attached.header().cast::<BbContent>() };
    crate::host::gate::mark_main_thread();
    let _scope = crate::host::gate::enter();
    // SAFETY: `ctx` is the table's own slot pointer.
    assert_eq!(unsafe { table.capabilities.expect("capabilities")(table.ctx) }, 0, "a backend with none says 0");
    let mut text: Vec<u8> = Vec::new();
    let report = table.report.expect("report");
    // SAFETY: `ctx` is the table's own slot; `text` outlives the call and `take_text` only appends to it.
    let status = unsafe { report(table.ctx, Some(take_text), (&mut text as *mut Vec<u8>).cast()) };
    assert_eq!(status, BB_ERR_UNSUPPORTED, "the default backend has no report");
    assert!(text.is_empty());
}
