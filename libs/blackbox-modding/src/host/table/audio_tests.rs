//! `bb.audio` called directly with a fake backend: a play reaches the backend with checked names and volume, and the handle
//! comes back.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::rc::Rc;

use super::audio::AudioBackend;
use super::{Attached, attach_audio};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbAudio {
    // SAFETY: the attachment owns a `BbAudio` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbAudio>() }
}

#[derive(Default)]
struct FakeAudio {
    played: RefCell<Vec<(String, String, f32)>>,
}

impl AudioBackend for FakeAudio {
    fn play(&self, _owner: &str, bank: &str, event: &str, volume: f32) -> BackendResult<u64> {
        self.played.borrow_mut().push((bank.to_owned(), event.to_owned(), volume));
        Ok(7)
    }
}

#[test]
fn a_play_reaches_the_backend_and_returns_its_handle() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeAudio::default());
    let attached = attach_audio(fake.clone(), "example.a");
    let table = table_of(&attached);
    let play = table.play.expect("play is offered");
    let mut id = 0_u64;
    let status = unsafe { play(table.ctx, BbStr::new("IG_GLOBAL"), BbStr::new("Siren"), 0.5, &mut id) };
    assert_eq!(status, BB_OK);
    assert_eq!(id, 7);
    assert_eq!(*fake.played.borrow(), vec![("IG_GLOBAL".to_owned(), "Siren".to_owned(), 0.5)]);
}

#[test]
fn a_volume_out_of_range_or_a_missing_handle_is_refused() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeAudio::default());
    let attached = attach_audio(fake.clone(), "example.a");
    let table = table_of(&attached);
    let play = table.play.expect("play is offered");
    let mut id = 0_u64;
    let (bank, event) = (BbStr::new("B"), BbStr::new("E"));
    assert_eq!(unsafe { play(table.ctx, bank, event, 1.5, &mut id) }, BB_ERR_ARG);
    assert_eq!(unsafe { play(table.ctx, bank, event, f32::NAN, &mut id) }, BB_ERR_ARG);
    assert_eq!(unsafe { play(table.ctx, bank, event, 0.5, core::ptr::null_mut()) }, BB_ERR_ARG);
    assert!(fake.played.borrow().is_empty());
}
