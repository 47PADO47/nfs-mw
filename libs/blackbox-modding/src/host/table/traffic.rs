//! `bb.traffic` on the host side: the backend the game implements, and the trampolines behind `BbTraffic`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_TRAFFIC_FIELD, BbIface, BbSinkFn, BbStatus, BbStr, BbTraffic, BbVec3, IFACE_TRAFFIC,
    IFACE_TRAFFIC_VERSION, MAX_NAME_BYTES,
};

/// One car of the traffic pool, as the game reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficCarInfo {
    /// The car's id (its folder), upper case.
    pub id: String,
    /// The class name (`compact`, `sedan`, `truck`, ...).
    pub class: String,
    /// The spawn weight and share cap, percent.
    pub weight: u32,
    /// At most this many at once; 0 is no limit.
    pub max_instances: u32,
    /// The factor on the pattern's cruising speeds.
    pub speed_factor: f32,
    /// The traffic patterns the car appears in, lower case; empty is every pattern.
    pub patterns: Vec<String>,
}

impl TrafficCarInfo {
    /// The line `list` writes for the car: the fields joined by [`BB_TRAFFIC_FIELD`].
    pub fn line(&self) -> String {
        let fields = [
            self.id.clone(),
            self.class.clone(),
            self.weight.to_string(),
            self.max_instances.to_string(),
            self.speed_factor.to_string(),
            self.patterns.join(","),
        ];
        let separator = BB_TRAFFIC_FIELD.to_string();
        fields.join(separator.as_str())
    }
}

/// What the game implements for `bb.traffic`. `owner` is the id of the mod that made the call.
pub trait TrafficBackend {
    /// The cars of the traffic pool: the traffic cars the mods added, in registration order.
    fn cars(&self, owner: &str) -> BackendResult<Vec<TrafficCarInfo>>;
    /// Ask for the traffic car `car` at the map position `at` (x, y), heading `heading` radians from +x. `car` is already
    /// checked to be a name of at most [`MAX_NAME_BYTES`], the numbers to be finite. `NotFound` for a car that is not in the
    /// pool; `NotReady` while no world is loaded.
    fn spawn(&self, owner: &str, car: &str, at: [f32; 2], heading: f32) -> BackendResult<()>;
}

/// Build the `bb.traffic` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn TrafficBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbTraffic {
        head: BbIface {
            size: size_of::<BbTraffic>() as u32,
            major: IFACE_TRAFFIC_VERSION.0,
            minor: IFACE_TRAFFIC_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        list: Some(list),
        spawn: Some(spawn),
    });
    Attached::new(IFACE_TRAFFIC, IFACE_TRAFFIC_VERSION, table, Box::new(|| {}))
}

/// The text of the list: a line per car, separated by `\n`.
pub(crate) fn list_text(cars: &[TrafficCarInfo]) -> String {
    let lines: Vec<String> = cars.iter().map(TrafficCarInfo::line).collect();
    lines.join("\n")
}

/// `list`: the cars of the pool go to the sink.
unsafe extern "C" fn list(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TrafficBackend>(ctx)? };
        let cars = slot.backend.cars(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &list_text(&cars)) }
    })
}

/// `spawn`: a car name, a finite point and a finite heading.
unsafe extern "C" fn spawn(ctx: *mut c_void, car: BbStr, position: BbVec3, heading: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TrafficBackend>(ctx)? };
        // SAFETY: the mod's name is valid for the call (ADR 0005 section 4).
        let car = unsafe { text_within(car, MAX_NAME_BYTES) }?;
        let finite = position.to_array().iter().chain([&heading]).all(|value| value.is_finite());
        if car.is_empty() || !finite {
            return Err(BB_ERR_ARG);
        }
        slot.backend.spawn(&slot.owner, car, [position.x, position.y], heading)
    })
}
