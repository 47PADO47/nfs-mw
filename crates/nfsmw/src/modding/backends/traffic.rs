//! `bb.traffic`: the traffic cars that mods added, listed and asked for by name (`docs/modding/api-traffic.md`). The pool is the
//! published mod cars that are traffic (`nfsmw_data::car::mods`); an ask goes into the queue the world's traffic manager serves
//! (`crate::traffic_requests`, `scenes/world/ai/mod_traffic.rs`), which places the car when its model is loaded.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::{BackendResult, TrafficBackend, TrafficCarInfo};
use nfsmw_data::car::mods::{self as car_mods, ModCar};

use super::bridge::Bridge;

/// The `bb.traffic` port of the game: one per mod table, sharing the bridge.
pub struct TrafficPort(pub(super) Rc<Bridge>);

/// A pool entry as the mods see it.
pub(super) fn info_of(car: &ModCar) -> Option<TrafficCarInfo> {
    let unit = car.traffic.as_ref()?;
    Some(TrafficCarInfo {
        id: car.folder.clone(),
        class: unit.class.name().to_owned(),
        weight: unit.weight,
        max_instances: unit.max_instances,
        speed_factor: unit.speed_factor,
        patterns: unit.patterns.clone(),
    })
}

impl TrafficBackend for TrafficPort {
    fn cars(&self, _owner: &str) -> BackendResult<Vec<TrafficCarInfo>> {
        Ok(car_mods::traffic_cars().into_iter().filter_map(info_of).collect())
    }

    fn spawn(&self, _owner: &str, car: &str, at: [f32; 2], heading: f32) -> BackendResult<()> {
        if self.0.world.borrow().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        let folder = car_mods::traffic_cars()
            .into_iter()
            .find(|candidate| candidate.folder.eq_ignore_ascii_case(car))
            .map(|found| found.folder.clone())
            .ok_or(BB_ERR_NOT_FOUND)?;
        match crate::traffic_requests::push(&folder, at, heading) {
            true => Ok(()),
            false => Err(BB_ERR_HOST),
        }
    }
}
