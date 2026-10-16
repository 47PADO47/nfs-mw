//! `bb.hud` called directly with a fake backend: a message reaches the backend with its text, style and time checked, and a
//! hide reaches it too.

use std::cell::RefCell;
use std::rc::Rc;

use super::hud::{HudBackend, MessageStyle};
use super::{Attached, attach_hud};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbHud {
    // SAFETY: the attachment owns a `BbHud` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbHud>() }
}

#[derive(Default)]
struct FakeHud {
    shown: RefCell<Vec<(String, MessageStyle, f32)>>,
    hidden: RefCell<u32>,
}

impl HudBackend for FakeHud {
    fn message(&self, _owner: &str, text: &str, style: MessageStyle, secs: f32) -> BackendResult<()> {
        self.shown.borrow_mut().push((text.to_owned(), style, secs));
        Ok(())
    }

    fn hide(&self, _owner: &str) -> BackendResult<()> {
        *self.hidden.borrow_mut() += 1;
        Ok(())
    }
}

#[test]
fn a_message_reaches_the_backend_with_its_style_and_time() {
    let fake = Rc::new(FakeHud::default());
    let attached = attach_hud(fake.clone(), "example.a");
    let table = table_of(&attached);
    let message = table.message.expect("message is offered");
    let text = "Pursuit";
    let status = unsafe { message(table.ctx, BbStr::new(text), BB_HUD_WARNING, 2.5) };
    assert_eq!(status, BB_OK);
    assert_eq!(*fake.shown.borrow(), vec![("Pursuit".to_owned(), MessageStyle::Warning, 2.5)]);
}

#[test]
fn bad_style_time_or_text_is_refused_before_the_backend() {
    let fake = Rc::new(FakeHud::default());
    let attached = attach_hud(fake.clone(), "example.a");
    let table = table_of(&attached);
    let message = table.message.expect("message is offered");
    let ok = BbStr::new("hi");
    assert_eq!(unsafe { message(table.ctx, ok, 99, 1.0) }, BB_ERR_ARG, "an unknown style");
    assert_eq!(unsafe { message(table.ctx, ok, BB_HUD_INFO, 0.0) }, BB_ERR_ARG, "no time");
    assert_eq!(unsafe { message(table.ctx, ok, BB_HUD_INFO, f32::NAN) }, BB_ERR_ARG, "a NaN time");
    assert_eq!(unsafe { message(table.ctx, ok, BB_HUD_INFO, 4000.0) }, BB_ERR_ARG, "longer than an hour");
    let long = "x".repeat(MAX_MESSAGE_BYTES + 1);
    assert_eq!(unsafe { message(table.ctx, BbStr::new(&long), BB_HUD_INFO, 1.0) }, BB_ERR_ARG, "a long text");
    assert!(fake.shown.borrow().is_empty());
}

#[test]
fn hide_reaches_the_backend() {
    let fake = Rc::new(FakeHud::default());
    let attached = attach_hud(fake.clone(), "example.a");
    let table = table_of(&attached);
    let hide = table.hide.expect("hide is offered");
    assert_eq!(unsafe { hide(table.ctx) }, BB_OK);
    assert_eq!(*fake.hidden.borrow(), 1);
}
