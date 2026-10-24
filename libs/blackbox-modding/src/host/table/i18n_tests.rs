//! `bb.i18n` called directly with a fake backend: the message, its variables and the owner reach the backend, the text
//! reaches the sink, and bad variables are refused.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::i18n::{I18nArg, I18nBackend, MAX_ARGS};
use super::{Attached, attach_i18n};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbI18n {
    // SAFETY: the attachment owns a `BbI18n` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbI18n>() }
}

/// One call the fake saw: the owner, the message id and the variables.
type Call = (String, String, Vec<(String, I18nArg)>);

#[derive(Default)]
struct FakeI18n {
    calls: RefCell<Vec<Call>>,
}

impl I18nBackend for FakeI18n {
    fn text(&self, owner: &str, id: &str, args: &[(String, I18nArg)]) -> BackendResult<String> {
        self.calls.borrow_mut().push((owner.to_owned(), id.to_owned(), args.to_vec()));
        Ok(format!("{id} x{}", args.len()))
    }

    fn locale(&self, _owner: &str) -> BackendResult<String> {
        Ok("de".to_owned())
    }

    fn generation(&self, _owner: &str) -> BackendResult<u64> {
        Ok(3)
    }
}

/// A sink that appends what the host writes to the `String` behind `user`.
unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: the test passes a live `RefCell<String>` as `user`, and the host's string is valid for the call.
    let out = unsafe { &*user.cast::<RefCell<String>>() };
    out.borrow_mut().push_str(unsafe { text.as_str() }.unwrap_or(""));
}

fn int(name: &'static str, value: i64) -> BbI18nArg {
    BbI18nArg { name: BbStr::new(name), kind: BB_I18N_INT, _pad: 0, int: value, text: BbStr::EMPTY }
}

#[test]
fn a_message_reaches_the_backend_and_the_sink() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeI18n::default());
    let attached = attach_i18n(fake.clone(), "example.a");
    let table = table_of(&attached);
    let text = table.text.expect("text is offered");
    let out = RefCell::new(String::new());
    let user = core::ptr::from_ref(&out).cast_mut().cast::<c_void>();
    let args = [
        int("laps", 3),
        BbI18nArg { name: BbStr::new("who"), kind: BB_I18N_TEXT, _pad: 0, int: 0, text: BbStr::new("Razor") },
    ];
    let status = unsafe { text(table.ctx, BbStr::new("race-laps"), args.as_ptr(), 2, Some(collect), user) };
    assert_eq!(status, BB_OK);
    assert_eq!(out.borrow().as_str(), "race-laps x2");
    let calls = fake.calls.borrow();
    assert_eq!(calls[0].0, "example.a");
    assert_eq!(
        calls[0].2,
        vec![("laps".to_owned(), I18nArg::Int(3)), ("who".to_owned(), I18nArg::Text("Razor".into()))]
    );
}

#[test]
fn bad_variables_are_refused() {
    let _scope = callback_on_main();
    let attached = attach_i18n(Rc::new(FakeI18n::default()), "example.a");
    let table = table_of(&attached);
    let text = table.text.expect("text is offered");
    let out = RefCell::new(String::new());
    let user = core::ptr::from_ref(&out).cast_mut().cast::<c_void>();
    let id = BbStr::new("x");
    assert_eq!(unsafe { text(table.ctx, id, core::ptr::null(), 1, Some(collect), user) }, BB_ERR_ARG);
    let mut bad = int("n", 1);
    bad.kind = 9;
    assert_eq!(unsafe { text(table.ctx, id, &bad, 1, Some(collect), user) }, BB_ERR_ARG);
    let many = vec![int("n", 1); MAX_ARGS + 1];
    assert_eq!(unsafe { text(table.ctx, id, many.as_ptr(), many.len() as u64, Some(collect), user) }, BB_ERR_ARG);
    assert_eq!(unsafe { text(table.ctx, id, core::ptr::null(), 0, Some(collect), user) }, BB_OK, "no variables");
}

#[test]
fn the_locale_and_the_generation_are_reported() {
    let _scope = callback_on_main();
    let attached = attach_i18n(Rc::new(FakeI18n::default()), "example.a");
    let table = table_of(&attached);
    let out = RefCell::new(String::new());
    let user = core::ptr::from_ref(&out).cast_mut().cast::<c_void>();
    assert_eq!(unsafe { (table.locale.unwrap())(table.ctx, Some(collect), user) }, BB_OK);
    assert_eq!(out.borrow().as_str(), "de");
    let mut generation = 0;
    assert_eq!(unsafe { (table.generation.unwrap())(table.ctx, &mut generation) }, BB_OK);
    assert_eq!(generation, 3);
    assert_eq!(unsafe { (table.generation.unwrap())(table.ctx, core::ptr::null_mut()) }, BB_ERR_ARG);
}
