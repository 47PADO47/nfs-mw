//! `bb.world` called directly with a fake backend: the track and the district reach the sink, the rain and the traffic reach
//! the backend checked, and the sink-less call and the bad values are refused.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::world::{MAX_COP_SHARE, WorldBackend};
use super::{Attached, attach_world};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbWorld {
    // SAFETY: the attachment owns a `BbWorld` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbWorld>() }
}

#[derive(Default)]
struct FakeWorld {
    rain: RefCell<Vec<(bool, f32)>>,
    traffic: RefCell<Vec<(u32, u32)>>,
}

impl WorldBackend for FakeWorld {
    fn track(&self, _owner: &str) -> BackendResult<String> {
        Ok("Bayview Track".to_owned())
    }

    fn section(&self, _owner: &str) -> BackendResult<String> {
        Ok("Old Harbor".to_owned())
    }

    fn set_rain(&self, _owner: &str, always: bool, amount: f32) -> BackendResult<()> {
        self.rain.borrow_mut().push((always, amount));
        Ok(())
    }

    fn set_traffic(&self, _owner: &str, cars: u32, cop_share: u32) -> BackendResult<()> {
        self.traffic.borrow_mut().push((cars, cop_share));
        Ok(())
    }
}

/// A sink that appends what the host writes to the `String` behind `user`.
unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: the test passes a live `RefCell<String>` as `user`, and the host's string is valid for the call.
    let out = unsafe { &*user.cast::<RefCell<String>>() };
    out.borrow_mut().push_str(unsafe { text.as_str() }.unwrap_or(""));
}

#[test]
fn the_track_and_the_section_reach_the_sink() {
    let _scope = callback_on_main();
    let attached = attach_world(Rc::new(FakeWorld::default()), "example.a");
    let table = table_of(&attached);
    let track = table.track.expect("track is offered");
    let section = table.section.expect("section is offered");
    let out = RefCell::new(String::new());
    let user = core::ptr::from_ref(&out).cast_mut().cast::<c_void>();
    assert_eq!(unsafe { track(table.ctx, Some(collect), user) }, BB_OK);
    assert_eq!(out.borrow().as_str(), "Bayview Track");
    out.borrow_mut().clear();
    assert_eq!(unsafe { section(table.ctx, Some(collect), user) }, BB_OK);
    assert_eq!(out.borrow().as_str(), "Old Harbor");
    assert_eq!(unsafe { track(table.ctx, None, core::ptr::null_mut()) }, BB_ERR_ARG, "a name needs a sink");
}

#[test]
fn rain_and_traffic_reach_the_backend_checked() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeWorld::default());
    let attached = attach_world(fake.clone(), "example.a");
    let table = table_of(&attached);
    let set_rain = table.set_rain.expect("set_rain is offered");
    let set_traffic = table.set_traffic.expect("set_traffic is offered");
    assert_eq!(unsafe { set_rain(table.ctx, 1, 0.8) }, BB_OK);
    assert_eq!(unsafe { set_rain(table.ctx, 0, 1.5) }, BB_ERR_ARG);
    assert_eq!(unsafe { set_traffic(table.ctx, 40, 25) }, BB_OK);
    assert_eq!(unsafe { set_traffic(table.ctx, 40, MAX_COP_SHARE + 1) }, BB_ERR_ARG);
    assert_eq!(*fake.rain.borrow(), vec![(true, 0.8)]);
    assert_eq!(*fake.traffic.borrow(), vec![(40, 25)]);
}
