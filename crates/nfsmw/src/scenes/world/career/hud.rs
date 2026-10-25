//! The career rows of the HUD: the bounty row and the heat meter (`docs/specs/career-economy.md`,
//! `docs/specs/ai-pursuit-heat.md`). [`WorldScene::career_hud`] reads the values once a frame; [`CareerHud::apply`]
//! puts them in the HUD state, from `scenes/world/hud.rs`.

use crate::hud::HudState;
use crate::scenes::world::WorldScene;

/// The top of the heat meter: the heat levels run 1 to 10.
const HEAT_TOP: f32 = 10.0;

/// The career values of the HUD for one frame. Off (the default) outside career mode.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(in crate::scenes::world) struct CareerHud {
    /// Career mode is on: the bounty row and the heat meter show.
    on: bool,
    /// The bounty row: the car's banked bounty plus the chase's pending points.
    bounty: u32,
    /// The car's heat, shown outside a chase.
    car_heat: f32,
    /// The career's cash, for the Custom HUD's free-roam extras.
    cash: u32,
    /// The chase's own pending points while a chase runs, for the Custom HUD's pursuit panel; `None` with no chase.
    chase: Option<u32>,
}

impl CareerHud {
    /// The values of a frame in career mode. `car` is the car's banked bounty and heat (`None` when the car has no
    /// career record); `pending` is the chase's pending points.
    pub(in crate::scenes::world) fn new(car: Option<(u32, f32)>, pending: u32) -> Self {
        let (banked, heat) = car.unwrap_or((0, 0.0));
        Self { on: true, bounty: bounty_row(banked, pending), car_heat: meter_heat(heat), cash: 0, chase: None }
    }

    /// The bounty row's number in career mode; `None` outside it.
    pub(in crate::scenes::world) fn bounty(&self) -> Option<u32> {
        self.on.then_some(self.bounty)
    }

    /// The same values with the career's `cash`.
    pub(in crate::scenes::world) fn with_cash(self, cash: u32) -> Self {
        Self { cash, ..self }
    }

    /// The same values with the running chase's pending points (`None` with no chase).
    pub(in crate::scenes::world) fn with_chase(self, chase: Option<u32>) -> Self {
        Self { chase, ..self }
    }

    /// The bounty the running chase has earned so far, in career mode; `None` outside it or with no chase.
    pub(in crate::scenes::world) fn chase_bounty(&self) -> Option<u32> {
        self.chase.filter(|_| self.on)
    }

    /// Puts the values in the HUD state: the bounty row when career mode is on, and the car's heat outside a chase
    /// (the police heat of the traffic stays during a chase). Leaves the state alone outside career mode.
    pub(in crate::scenes::world) fn apply(&self, state: &mut HudState) {
        if !self.on {
            return;
        }
        state.career_bounty = Some(self.bounty);
        state.cash = Some(self.cash);
        if state.pursuit.is_some() {
            return;
        }
        state.vehicle_heat = self.car_heat;
    }
}

/// The bounty row: the banked bounty and the pending points, summed (saturating).
fn bounty_row(banked: u32, pending: u32) -> u32 {
    banked.saturating_add(pending)
}

/// The heat meter's level: the car's heat within the meter, 0 for a heat that is not a number.
fn meter_heat(heat: f32) -> f32 {
    if heat.is_nan() {
        return 0.0;
    }
    heat.clamp(0.0, HEAT_TOP)
}

impl WorldScene {
    /// Reads the career values of this frame: the lead's car's banked bounty and heat from its career record, the car
    /// `career.settings.car_handle` names, as the rest of the career reads it. The pending points are the running rep
    /// points of the chase while it runs (`ai-pursuit-heat.md` section 5.1); after it ends they are banked into the
    /// record, so they count only while the chase is on.
    pub(super) fn career_hud(&mut self) {
        let lead = self.players.has_lead();
        let car = self.career.handle.with(|career| {
            let record = career.stable.career_for_handle(career.settings.car_handle).filter(|_| lead)?;
            Some((record.bounty, record.heat))
        });
        let cash = self.career.handle.with(|career| career.settings.cash).unwrap_or(0);
        let chase = self.traffic.as_ref().filter(|traffic| traffic.is_pursuing()).map(|traffic| traffic.rep_points());
        self.career.hud = CareerHud::new(car.flatten(), chase.unwrap_or(0)).with_cash(cash).with_chase(chase);
    }
}

#[cfg(test)]
#[path = "hud_tests.rs"]
mod tests;
