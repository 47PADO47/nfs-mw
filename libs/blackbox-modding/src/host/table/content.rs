//! `bb.content` on the host side: the backend the game implements, and the trampoline behind `BbContent`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of};
use crate::abi::{BbContent, BbIface, BbStatus, IFACE_CONTENT, IFACE_CONTENT_VERSION};

/// What the game implements for `bb.content`. `owner` is the id of the mod that made the call.
pub trait ContentBackend {
    /// Read the folder of mod `owner` again and add the content it holds that is new: its `files/` layer and its
    /// content manifests. Content already added is not added twice.
    fn rescan(&self, owner: &str) -> BackendResult<()>;
}

/// Build the `bb.content` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn ContentBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbContent {
        head: BbIface {
            size: size_of::<BbContent>() as u32,
            major: IFACE_CONTENT_VERSION.0,
            minor: IFACE_CONTENT_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        rescan: Some(rescan),
    });
    // The content a mod adds stays for the run, so nothing is removed on unload; the release keeps the slot alive.
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_CONTENT, IFACE_CONTENT_VERSION, table, release)
}

/// `rescan`.
unsafe extern "C" fn rescan(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn ContentBackend>(ctx)? };
        slot.backend.rescan(&slot.owner)
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::abi::BB_OK;

    #[derive(Default)]
    struct Fake {
        owners: RefCell<Vec<String>>,
    }

    impl ContentBackend for Fake {
        fn rescan(&self, owner: &str) -> BackendResult<()> {
            self.owners.borrow_mut().push(owner.to_owned());
            Ok(())
        }
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
}
