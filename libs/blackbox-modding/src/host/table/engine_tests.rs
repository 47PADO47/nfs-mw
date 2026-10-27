//! `bb.engine` called directly with a fake backend: the state and the car id reach the mod, the switch reaches the backend
//! checked, and a bad sound path, gain or delay is refused.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::engine::{EnginePhase, EngineState, plain_path};
use super::{Attached, EngineBackend, attach_engine};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::test_support::callback_on_main;

fn table_of(attached: &Attached) -> &BbEngine {
    // SAFETY: the attachment owns a `BbEngine` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbEngine>() }
}

#[derive(Default)]
struct FakeEngine {
    started: RefCell<Vec<(String, f32, f32)>>,
    stops: RefCell<u32>,
    released: RefCell<Vec<String>>,
}

impl EngineBackend for FakeEngine {
    fn state(&self, _owner: &str) -> BackendResult<EngineState> {
        Ok(EngineState { phase: EnginePhase::Starting, free_roam: true, throttle: 0.5, brake: 0.25, speed: -1.5 })
    }

    fn car(&self, _owner: &str) -> BackendResult<String> {
        Ok("POR911CARR12".to_owned())
    }

    fn stop(&self, _owner: &str) -> BackendResult<()> {
        *self.stops.borrow_mut() += 1;
        Ok(())
    }

    fn start(&self, _owner: &str, sound: &str, gain: f32, delay: f32) -> BackendResult<()> {
        self.started.borrow_mut().push((sound.to_owned(), gain, delay));
        Ok(())
    }

    fn release(&self, owner: &str) {
        self.released.borrow_mut().push(owner.to_owned());
    }
}

/// A sink that appends what the host writes to the `String` behind `user`.
unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: the test passes a live `RefCell<String>` as `user`, and the host's string is valid for the call.
    let out = unsafe { &*user.cast::<RefCell<String>>() };
    out.borrow_mut().push_str(unsafe { text.as_str() }.unwrap_or(""));
}

#[test]
fn the_table_says_its_interface_and_version() {
    let attached = attach_engine(Rc::new(FakeEngine::default()), "example.a");
    assert_eq!(attached.id(), IFACE_ENGINE);
    assert_eq!(attached.version(), IFACE_ENGINE_VERSION);
    assert_eq!(table_of(&attached).head.size as usize, core::mem::size_of::<BbEngine>());
}

#[test]
fn the_state_and_the_car_reach_the_mod() {
    let _scope = callback_on_main();
    let attached = attach_engine(Rc::new(FakeEngine::default()), "example.a");
    let table = table_of(&attached);
    let mut out = BbEngineState { size: core::mem::size_of::<BbEngineState>() as u32, ..BbEngineState::default() };
    // SAFETY: `out` is valid for a whole `BbEngineState`, which its size word says.
    assert_eq!(unsafe { (table.state.expect("state is offered"))(table.ctx, &mut out) }, BB_OK);
    assert_eq!(
        (out.phase, out.free_roam, out.throttle, out.brake, out.speed),
        (BB_ENGINE_STARTING, 1, 0.5, 0.25, -1.5)
    );
    let name = RefCell::new(String::new());
    let user = core::ptr::from_ref(&name).cast_mut().cast::<c_void>();
    // SAFETY: the sink and `user` live for the call.
    assert_eq!(unsafe { (table.car.expect("car is offered"))(table.ctx, Some(collect), user) }, BB_OK);
    assert_eq!(name.borrow().as_str(), "POR911CARR12");
}

#[test]
fn a_short_state_struct_gets_only_the_fields_it_allows() {
    let _scope = callback_on_main();
    let attached = attach_engine(Rc::new(FakeEngine::default()), "example.a");
    let table = table_of(&attached);
    // A caller that knows the size word and the phase only.
    let mut out = BbEngineState { size: 8, phase: 99, free_roam: 77, ..BbEngineState::default() };
    // SAFETY: `out` is larger than the 8 bytes its size word allows the host to write.
    assert_eq!(unsafe { (table.state.expect("state is offered"))(table.ctx, &mut out) }, BB_OK);
    assert_eq!(out.phase, BB_ENGINE_STARTING);
    assert_eq!(out.free_roam, 77, "a field beyond the size word is left alone");
}

#[test]
fn stop_and_start_reach_the_backend_with_checked_arguments() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeEngine::default());
    let attached = attach_engine(backend.clone(), "example.a");
    let table = table_of(&attached);
    let (stop, start) = (table.stop.expect("stop is offered"), table.start.expect("start is offered"));
    // SAFETY: plain values and a string that lives for the call.
    unsafe {
        assert_eq!(stop(table.ctx), BB_OK);
        let sound = "SOUND/MODENGINE/CAR/IGNITION.WAV";
        assert_eq!(start(table.ctx, BbStr::new(sound), 0.8, 0.6), BB_OK);
        assert_eq!(start(table.ctx, BbStr::EMPTY, 1.0, 0.0), BB_OK, "no sound is allowed");
        assert_eq!(start(table.ctx, BbStr::new(sound), 9.0, 0.0), BB_ERR_ARG, "too loud");
        assert_eq!(start(table.ctx, BbStr::new(sound), f32::NAN, 0.0), BB_ERR_ARG);
        assert_eq!(start(table.ctx, BbStr::new(sound), 1.0, 11.0), BB_ERR_ARG, "too long a delay");
        assert_eq!(start(table.ctx, BbStr::new(sound), 1.0, -0.5), BB_ERR_ARG);
        assert_eq!(start(table.ctx, BbStr::new("../x.wav"), 1.0, 0.0), BB_ERR_ARG);
    }
    assert_eq!(*backend.stops.borrow(), 1);
    assert_eq!(
        *backend.started.borrow(),
        [("SOUND/MODENGINE/CAR/IGNITION.WAV".to_owned(), 0.8, 0.6), (String::new(), 1.0, 0.0)]
    );
}

#[test]
fn only_plain_relative_sound_paths_are_taken() {
    assert!(plain_path(""));
    assert!(plain_path("SOUND/MODENGINE/CAR/IGNITION.WAV"));
    for bad in ["/etc/x.wav", "..\\x.wav", "a/../b.wav", "C:/x.wav", "a\\b.wav", ".hidden", "a\nb.wav"] {
        assert!(!plain_path(bad), "{bad:?}");
    }
}

#[test]
fn dropping_the_table_tells_the_backend_which_mod_unloaded() {
    let backend = Rc::new(FakeEngine::default());
    let attached = attach_engine(backend.clone(), "example.a");
    assert!(backend.released.borrow().is_empty());
    drop(attached);
    assert_eq!(*backend.released.borrow(), ["example.a"]);
}
