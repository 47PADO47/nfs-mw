//! Safe house entry in career mode (`docs/specs/career-safehouse.md`, `docs/specs/career-safehouse-2.md`). The rules are in
//! `nfsmw_career::safehouse`: this file gathers the world's facts (is a chase running, is its cool-down meter shown, the
//! driven car's heat), asks the rules, and applies the answer to the world and the career.

use blackbox_pursuit::{CooldownStatus, EndReason};
use nfsmw_career::safehouse::EntryPermissions;
use nfsmw_career::safehouse::PursuitPhase;
use nfsmw_career::safehouse::ZoneKind as SafeZoneKind;
use nfsmw_career::safehouse::engage::{EngageDecision, EngageInput, engage};
use nfsmw_career::safehouse::heat::unload_world;

use crate::career::CareerRequest;
use crate::scenes::world::WorldScene;
use crate::scenes::world::entry_zones::ZoneKind;

impl WorldScene {
    /// The career's answer to a zone the car drove into, with its card up and Interact pressed. True when the career took
    /// the entry, and the caller shows nothing more; false when the career does not take it (not career mode, a race, a
    /// custom shop, or a safe house the pursuit refuses), and the caller goes on as it does outside the career.
    pub(in crate::scenes::world) fn career_enter_zone(&mut self, kind: ZoneKind, name: &str) -> bool {
        if !self.career.handle.is_active() {
            return false;
        }
        match kind {
            ZoneKind::SafeHouse => self.career_enter_safe_house(name),
            ZoneKind::CarLot => self.career_enter_car_lot(name),
            // A custom shop (`ZoneType customshop`) has no career request yet, and a race is not the career's.
            ZoneKind::AutoShop | ZoneKind::Race => false,
        }
    }

    /// A safe house entry: the engage decision for the chase's phase, the chase ends as evaded when it was cooling down,
    /// the driven car's heat goes into its record, and the front end is asked for the safe house.
    fn career_enter_safe_house(&mut self, name: &str) -> bool {
        let phase = self.safe_house_phase();
        let decision = match safe_house_decision(phase) {
            Some(decision) => decision,
            None => {
                log::info!("career: safe house {name} refused: the pursuit does not allow it ({phase:?})");
                return false;
            }
        };
        let cool_down = match decision {
            EngageDecision::EndCoolDownPursuit => true,
            EngageDecision::PostEnterMessage(_) => false,
            other => {
                log::info!("career: safe house {name} refused: {other:?}");
                return false;
            }
        };
        // The heat is read while the chase still runs; ending it first would leave no live heat to keep.
        self.write_driven_heat();
        if cool_down {
            self.end_cool_down_pursuit();
            self.career.pursuit.note_safe_house_unload();
        }
        self.career.requests.push_back(CareerRequest::SafeHouse { zone: name.to_owned() });
        log::info!("career: entering safe house {name} (cool-down evade: {cool_down})");
        true
    }

    /// A car lot entry: the car lot screen is asked for. No pursuit rule applies to it, so the chase is left alone.
    fn career_enter_car_lot(&mut self, name: &str) -> bool {
        self.career.requests.push_back(CareerRequest::CarLot { zone: name.to_owned() });
        log::info!("career: entering car lot {name}");
        true
    }

    /// The chase's phase as the entry rules see it: none, running, or cooling down (the cool-down meter is shown).
    fn safe_house_phase(&self) -> PursuitPhase {
        match self.traffic.as_ref() {
            Some(traffic) => pursuit_phase(traffic.is_pursuing(), traffic.cooldown_status()),
            None => PursuitPhase::None,
        }
    }

    /// The cool-down engage ends the chase as evaded (`engage::cooldown_end`: status EVADED, evade level 1). The career's
    /// evade bookkeeping for that end (the factor on the records, the strike tick) belongs to `career_pursuit`.
    fn end_cool_down_pursuit(&mut self) {
        let Some(traffic) = self.traffic.as_mut() else { return };
        traffic.end_pursuit(EndReason::Evaded);
    }

    /// The driven car's heat goes into its career record when the world is torn down (`heat::unload_world`, capped at 5).
    /// Without a chase the world holds no heat value of its own, so the record is left as it is.
    fn write_driven_heat(&mut self) {
        let live = self.traffic.as_ref().filter(|traffic| traffic.is_pursuing()).map(|traffic| traffic.heat_value());
        let Some(heat) = live else { return };
        let written = self
            .career
            .handle
            .with(|career| unload_world(&career.settings, &mut career.stable, heat, true))
            .unwrap_or(false);
        if !written {
            log::warn!("career: the driven car has no career record; its heat is not kept");
        }
    }
}

/// The pursuit's phase for the entry rules: no chase, a chase that runs, or a chase whose cool-down meter is shown.
pub(super) fn pursuit_phase(pursuing: bool, status: CooldownStatus) -> PursuitPhase {
    match (pursuing, status) {
        (false, _) => PursuitPhase::None,
        (true, CooldownStatus::CoolDown) => PursuitPhase::CoolingDown,
        (true, _) => PursuitPhase::Active,
    }
}

/// The engage input of a safe house entry made with its card up. The car is in the zone and the prompt is up. The world
/// runs no race event and no controller is busy, so those are false.
pub(super) fn safe_house_input(phase: PursuitPhase) -> EngageInput {
    EngageInput {
        pursuit: phase,
        prompt_present: true,
        prompt_ready: true,
        zone: Some(SafeZoneKind::SafeHouse),
        event_underway: false,
        node_allows_engage: false,
        controller_busy: false,
    }
}

/// The engage decision for a safe house entry at `phase`. None when the entry permissions refuse the safe house (a chase
/// that is not cooling down), so no prompt would have offered it.
pub(super) fn safe_house_decision(phase: PursuitPhase) -> Option<EngageDecision> {
    let permitted = EntryPermissions::compute(false, phase).allows(SafeZoneKind::SafeHouse);
    if !permitted {
        return None;
    }
    Some(engage(&safe_house_input(phase)))
}

#[cfg(test)]
#[path = "safehouse_tests.rs"]
mod tests;
