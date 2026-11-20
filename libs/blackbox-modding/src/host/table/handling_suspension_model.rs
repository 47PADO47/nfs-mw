//! The suspension hook of a mod as the vehicle sees it: an adapter from the mod's C callback to the vehicle's `SuspensionModel`.
//! The call, the budget and the fault policy are those of [`ModHook`] (see `handling_model.rs`).

use core::mem::size_of;
use std::cell::Cell;
use std::rc::Rc;

use blackbox_vehicle::suspension::{
    SuspensionModel, SuspensionQuery, SuspensionReject, SuspensionResult, SuspensionWheelQuery, SuspensionWheelResult,
};

use super::handling_model::{Outcome, SuspensionHook};
use crate::abi::{BbSuspensionQuery, BbSuspensionResult, BbSuspensionWheel, BbSuspensionWheelResult};

impl SuspensionHook {
    /// An adapter for one vehicle. `revision` is read at each call and echoed in the query as `vehicle_revision`.
    pub fn model(&self, revision: Rc<Cell<u32>>) -> Box<dyn SuspensionModel> {
        Box::new(ModSuspensionModel { hook: self.clone(), revision })
    }
}

/// The adapter the vehicle holds.
struct ModSuspensionModel {
    hook: SuspensionHook,
    revision: Rc<Cell<u32>>,
}

fn wire_wheel(wheel: &SuspensionWheelQuery) -> BbSuspensionWheel {
    BbSuspensionWheel {
        loaded: u32::from(wheel.loaded),
        compression_m: wheel.compression_m,
        compression_speed_mps: wheel.compression_speed_mps,
        steer_angle_rad: wheel.steer_angle_rad,
        body_accel_up_mps2: wheel.body_accel_up_mps2,
        tyre_lateral_force_n: wheel.tyre_lateral_force_n,
        static_load_n: wheel.static_load_n,
        reserved: [0; 3],
    }
}

fn wire_query(query: &SuspensionQuery, revision: u32) -> BbSuspensionQuery {
    BbSuspensionQuery {
        size: size_of::<BbSuspensionQuery>() as u32,
        axle_index: query.axle,
        role: query.role,
        vehicle_revision: revision,
        dt: query.dt,
        rest_travel_m: query.rest_travel_m,
        max_travel_m: query.max_travel_m,
        corner_mass_kg: query.corner_mass_kg,
        wheels: [wire_wheel(&query.wheels[0]), wire_wheel(&query.wheels[1])],
        reserved: [0; 8],
    }
}

fn vehicle_wheel(wire: &BbSuspensionWheelResult) -> SuspensionWheelResult {
    SuspensionWheelResult {
        body_force_n: wire.body_force_n,
        tyre_load_n: wire.tyre_load_n,
        camber_rad: wire.camber_rad,
    }
}

impl SuspensionModel for ModSuspensionModel {
    fn axle(&mut self, query: &SuspensionQuery) -> Option<SuspensionResult> {
        let wire = wire_query(query, self.revision.get());
        let mut answer =
            BbSuspensionResult { size: size_of::<BbSuspensionResult>() as u32, ..BbSuspensionResult::default() };
        match self.hook.invoke(&wire, &mut answer) {
            Outcome::Handled => {
                Some(SuspensionResult { wheels: [vehicle_wheel(&answer.wheels[0]), vehicle_wheel(&answer.wheels[1])] })
            }
            Outcome::NotHandled | Outcome::Fault => None,
        }
    }

    fn rejected(&mut self, why: SuspensionReject) {
        self.hook.refused(why);
    }

    fn retired(&self) -> bool {
        self.hook.is_retired()
    }

    fn duplicate(&self) -> Option<Box<dyn SuspensionModel>> {
        Some(self.hook.model(self.revision.clone()))
    }
}
