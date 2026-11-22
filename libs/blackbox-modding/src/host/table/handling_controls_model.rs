//! The controls hook of a mod as the vehicle sees it: an adapter from the mod's C callback to the vehicle's `ControlsModel`. The
//! call, the budget and the fault policy are those of [`ModHook`] (see `handling_model.rs`).

use core::mem::size_of;
use std::cell::Cell;
use std::rc::Rc;

use blackbox_vehicle::input::{ControlsModel, ControlsQuery, ControlsReject, ControlsResult};

use super::handling_model::{ControlsHook, Outcome};
use crate::abi::{BB_CONTROLS_FLAG_STEER, BbControlsQuery, BbControlsResult};

impl ControlsHook {
    /// An adapter for one vehicle. `revision` is read at each call and echoed in the query as `vehicle_revision`.
    pub fn model(&self, revision: Rc<Cell<u32>>) -> Box<dyn ControlsModel> {
        Box::new(ModControlsModel { hook: self.clone(), revision })
    }
}

/// The adapter the vehicle holds.
struct ModControlsModel {
    hook: ControlsHook,
    revision: Rc<Cell<u32>>,
}

fn wire_query(query: &ControlsQuery, revision: u32) -> BbControlsQuery {
    let loaded = query.wheel_loaded.iter().enumerate().fold(0, |mask, (i, on)| mask | (u32::from(*on) << i));
    BbControlsQuery {
        size: size_of::<BbControlsQuery>() as u32,
        role: query.role,
        vehicle_revision: revision,
        dt: query.dt,
        throttle: query.throttle,
        brake: query.brake,
        handbrake: query.handbrake,
        steer: query.steer,
        forward_speed: query.forward_speed,
        yaw_rate: query.yaw_rate,
        lateral_accel: query.lateral_accel,
        longitudinal_accel: query.longitudinal_accel,
        wheel_loaded_mask: loaded,
        slip_ratio: query.slip_ratio,
        slip_angle: query.slip_angle,
        reserved: [0; 7],
    }
}

fn vehicle_result(wire: &BbControlsResult) -> ControlsResult {
    ControlsResult {
        throttle: wire.throttle,
        brake: wire.brake,
        steer_target: (wire.flags & BB_CONTROLS_FLAG_STEER != 0).then_some(wire.steer_target_rad),
    }
}

impl ControlsModel for ModControlsModel {
    fn filter(&mut self, query: &ControlsQuery) -> Option<ControlsResult> {
        let wire = wire_query(query, self.revision.get());
        let mut answer = BbControlsResult { size: size_of::<BbControlsResult>() as u32, ..BbControlsResult::default() };
        match self.hook.invoke(&wire, &mut answer) {
            Outcome::Handled => Some(vehicle_result(&answer)),
            Outcome::NotHandled | Outcome::Fault => None,
        }
    }

    fn rejected(&mut self, why: ControlsReject) {
        self.hook.refused(why);
    }

    fn retired(&self) -> bool {
        self.hook.is_retired()
    }

    fn duplicate(&self) -> Option<Box<dyn ControlsModel>> {
        Some(self.hook.model(self.revision.clone()))
    }
}
