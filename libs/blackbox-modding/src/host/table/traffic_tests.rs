//! `bb.traffic` called directly with a fake backend: the pool reaches the mod as lines, a spawn reaches the backend checked,
//! and a bad name or number is refused.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::traffic::list_text;
use super::{Attached, TrafficBackend, TrafficCarInfo, attach_traffic};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::test_support::callback_on_main;

fn table_of(attached: &Attached) -> &BbTraffic {
    // SAFETY: the attachment owns a `BbTraffic` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbTraffic>() }
}

fn car(id: &str, class: &str, weight: u32, patterns: &[&str]) -> TrafficCarInfo {
    TrafficCarInfo {
        id: id.to_owned(),
        class: class.to_owned(),
        weight,
        max_instances: 0,
        speed_factor: 1.0,
        patterns: patterns.iter().map(|name| (*name).to_owned()).collect(),
    }
}

#[derive(Default)]
struct FakeTraffic {
    spawned: RefCell<Vec<(String, [f32; 2], f32)>>,
}

impl TrafficBackend for FakeTraffic {
    fn cars(&self, _owner: &str) -> BackendResult<Vec<TrafficCarInfo>> {
        Ok(vec![car("AUDA3_TR", "compact", 10, &[]), car("GMCTOPKIC_TR", "truck", 4, &["cityhighway", "industrial"])])
    }

    fn spawn(&self, _owner: &str, car: &str, at: [f32; 2], heading: f32) -> BackendResult<()> {
        if !car.eq_ignore_ascii_case("AUDA3_TR") {
            return Err(BB_ERR_NOT_FOUND);
        }
        self.spawned.borrow_mut().push((car.to_owned(), at, heading));
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
fn the_table_says_its_interface_and_version() {
    let attached = attach_traffic(Rc::new(FakeTraffic::default()), "example.a");
    assert_eq!(attached.id(), IFACE_TRAFFIC);
    assert_eq!(attached.id(), "bb.traffic");
    assert_eq!(attached.version(), IFACE_TRAFFIC_VERSION);
    assert_eq!(IFACE_TRAFFIC_VERSION, (1, 0));
    assert_eq!(table_of(&attached).head.size as usize, core::mem::size_of::<BbTraffic>());
}

#[test]
fn the_pool_reaches_the_mod_as_one_line_per_car() {
    let _scope = callback_on_main();
    let attached = attach_traffic(Rc::new(FakeTraffic::default()), "example.a");
    let table = table_of(&attached);
    let text = RefCell::new(String::new());
    let user = core::ptr::from_ref(&text).cast_mut().cast::<c_void>();
    // SAFETY: the sink and `user` live for the call.
    assert_eq!(unsafe { (table.list.expect("list is offered"))(table.ctx, Some(collect), user) }, BB_OK);
    let text = text.borrow();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "AUDA3_TR\tcompact\t10\t0\t1\t");
    assert_eq!(lines[1], "GMCTOPKIC_TR\ttruck\t4\t0\t1\tcityhighway,industrial");
}

#[test]
fn an_empty_pool_is_an_empty_text() {
    assert_eq!(list_text(&[]), "");
    assert_eq!(list_text(&[car("A_TR", "sedan", 20, &[])]), "A_TR\tsedan\t20\t0\t1\t");
}

#[test]
fn a_spawn_reaches_the_backend_with_the_point_and_the_heading() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeTraffic::default());
    let attached = attach_traffic(backend.clone(), "example.a");
    let table = table_of(&attached);
    let spawn = table.spawn.expect("spawn is offered");
    let at = BbVec3 { x: 1200.0, y: -300.0, z: 9.0 };
    // SAFETY: a string that lives for the call and plain values.
    unsafe {
        assert_eq!(spawn(table.ctx, BbStr::new("auda3_tr"), at, 1.5), BB_OK);
        assert_eq!(
            spawn(table.ctx, BbStr::new("NOSUCH"), at, 0.0),
            BB_ERR_NOT_FOUND,
            "the backend says what is in the pool"
        );
    }
    assert_eq!(*backend.spawned.borrow(), [("auda3_tr".to_owned(), [1200.0, -300.0], 1.5)]);
}

#[test]
fn a_bad_name_or_a_number_that_is_not_finite_is_refused_before_the_backend() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeTraffic::default());
    let attached = attach_traffic(backend.clone(), "example.a");
    let table = table_of(&attached);
    let spawn = table.spawn.expect("spawn is offered");
    let at = BbVec3 { x: 0.0, y: 0.0, z: 0.0 };
    let long = "A".repeat(MAX_NAME_BYTES + 1);
    // SAFETY: strings that live for the call and plain values.
    unsafe {
        assert_eq!(spawn(table.ctx, BbStr::EMPTY, at, 0.0), BB_ERR_ARG, "no name");
        assert_eq!(spawn(table.ctx, BbStr::new(&long), at, 0.0), BB_ERR_ARG, "too long a name");
        assert_eq!(spawn(table.ctx, BbStr::new("AUDA3_TR"), at, f32::NAN), BB_ERR_ARG);
        let far = BbVec3 { x: f32::INFINITY, ..at };
        assert_eq!(spawn(table.ctx, BbStr::new("AUDA3_TR"), far, 0.0), BB_ERR_ARG);
    }
    assert!(backend.spawned.borrow().is_empty());
}
