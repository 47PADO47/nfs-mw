//! The engage button and entering while a pursuit cools down (`docs/specs/career-safehouse.md` sections 3.3 to 3.5).
//!
//! The decision is a pure function of an input struct. What the caller does with it (post the message, change the
//! pursuit, open the results screen) is named by the result types, and the evade bookkeeping that follows is in
//! [`finish_cooldown_pursuit`].

use super::heat::{DEFAULT_EVADE_HEAT_FACTOR, apply_heat_factor};
use super::zone::{PursuitPhase, ZoneKind};
use crate::economy::{PursuitEnd, PursuitResult, end_player_pursuit};
use crate::settings::CareerSettings;
use crate::stable::Stable;

/// Everything the engage handler reads when the engage action is pressed (online and LAN contexts and the debug
/// switch skip the handler entirely; the caller does not call it then).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngageInput {
    /// The state of the player pursuit.
    pub pursuit: PursuitPhase,
    /// The prompt component is present (a zone or an event trigger was entered and not left).
    pub prompt_present: bool,
    /// The prompt object property `0x0280164F` is true (probably "the prompt is up"; unconfirmed).
    pub prompt_ready: bool,
    /// The zone kind stored by `EnterGateZone`, `None` when the stored string is none of the three kinds.
    pub zone: Option<ZoneKind>,
    /// An event is underway (`race_status.event_underway`).
    pub event_underway: bool,
    /// The current event node carries the flag `0x2B1F54F6`, which lets the engage action through while an event
    /// is underway (its meaning is not known).
    pub node_allows_engage: bool,
    /// The HUD controller busy byte (`+700`) is non-zero.
    pub controller_busy: bool,
}

/// What the engage action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngageDecision {
    /// A safe house zone with the pursuit cooling down: call `EndPursuitEnteringSafehouse` ([`cooldown_end`]);
    /// the safe house is entered later, when the results screen closes ([`results_screen_closed`]).
    EndCoolDownPursuit,
    /// An event is underway and its node does not allow it: a refusal HUD event of kind 0, argument 1.
    RefuseEventUnderway,
    /// The controller is busy: a refusal HUD event of kind 1, argument 1.
    RefuseBusy,
    /// Post `MEnterSafeHouse` with the zone type string now. `None` means the component engaged an event trigger
    /// (the node hash path) and no zone message is sent.
    PostEnterMessage(Option<ZoneKind>),
    /// Nothing happens (no prompt, or the prompt flag is false).
    Nothing,
}

/// The engage action, in the order of the spec (3.3):
///
/// 1. pursuit cooling down, prompt flag true, stored zone `safehouse`: end the pursuit entering the safe house;
/// 2. an event underway and the node not flagged `0x2B1F54F6`: refusal, kind 0;
/// 3. otherwise, with the controller not busy: a present, ready prompt posts the zone message (a zone whose
///    string is none of the three kinds posts nothing);
/// 4. the controller busy: refusal, kind 1.
pub fn engage(input: &EngageInput) -> EngageDecision {
    if input.pursuit == PursuitPhase::CoolingDown && input.prompt_ready && input.zone == Some(ZoneKind::SafeHouse) {
        return EngageDecision::EndCoolDownPursuit;
    }
    if input.event_underway && !input.node_allows_engage {
        return EngageDecision::RefuseEventUnderway;
    }
    if input.controller_busy {
        return EngageDecision::RefuseBusy;
    }
    if !(input.prompt_present && input.prompt_ready) {
        return EngageDecision::Nothing;
    }
    EngageDecision::PostEnterMessage(input.zone)
}

/// The pursuit status the cool-down engage writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoolDownEnd {
    /// Status 4: evaded.
    pub status_evaded: bool,
    /// The evade level, 1.0.
    pub evade_level: f32,
    /// The byte "enter the safe house when done".
    pub enter_safe_house_when_done: bool,
}

/// `EndPursuitEnteringSafehouse`: writes exactly three fields of the pursuit and nothing else.
pub const fn cooldown_end() -> CoolDownEnd {
    CoolDownEnd { status_evaded: true, evade_level: 1.0, enter_safe_house_when_done: true }
}

/// The normal evade handling that the next pursuit update runs, applied to the career: the evade heat factor on
/// every car of the stable, then the strike bookkeeping and the recorder ([`end_player_pursuit`]). An evade into
/// the safe house costs exactly what a normal evade costs and nothing more (3.4). `evade_factor` is the
/// `pursuitlevels/default` value (see [`DEFAULT_EVADE_HEAT_FACTOR`]). The results screen, the type 0x14 message
/// and the other notices are the caller's.
pub fn finish_cooldown_pursuit(
    settings: &CareerSettings,
    stable: &mut Stable,
    result: &PursuitResult,
    evade_factor: f32,
) -> PursuitEnd {
    apply_heat_factor(stable, evade_factor);
    end_player_pursuit(settings, stable, result)
}

/// [`finish_cooldown_pursuit`] with the documented default factor.
pub fn finish_cooldown_pursuit_default(
    settings: &CareerSettings,
    stable: &mut Stable,
    result: &PursuitResult,
) -> PursuitEnd {
    finish_cooldown_pursuit(settings, stable, result, DEFAULT_EVADE_HEAT_FACTOR)
}

/// The results screen closes (its destructor): when the stored "enter the safe house after close" byte is set it is
/// cleared and `MEnterSafeHouse` is posted with the payload `safehouse`. Returns the new byte and the message.
pub fn results_screen_closed(enter_after_close: bool) -> (bool, Option<ZoneKind>) {
    match enter_after_close {
        true => (false, Some(ZoneKind::SafeHouse)),
        false => (false, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::{InfractionMask, PursuitResult};
    use crate::stable::{CarRecord, CareerRecord};

    fn input() -> EngageInput {
        EngageInput {
            pursuit: PursuitPhase::None,
            prompt_present: true,
            prompt_ready: true,
            zone: Some(ZoneKind::SafeHouse),
            event_underway: false,
            node_allows_engage: false,
            controller_busy: false,
        }
    }

    #[test]
    fn a_ready_prompt_posts_the_zone_message() {
        assert_eq!(engage(&input()), EngageDecision::PostEnterMessage(Some(ZoneKind::SafeHouse)));
        let lot = EngageInput { zone: Some(ZoneKind::CarLot), ..input() };
        assert_eq!(engage(&lot), EngageDecision::PostEnterMessage(Some(ZoneKind::CarLot)));
    }

    #[test]
    fn a_zone_string_of_no_kind_posts_no_zone() {
        let other = EngageInput { zone: None, ..input() };
        assert_eq!(engage(&other), EngageDecision::PostEnterMessage(None));
    }

    #[test]
    fn cooling_down_at_a_safe_house_ends_the_pursuit_first() {
        let cooling = EngageInput { pursuit: PursuitPhase::CoolingDown, ..input() };
        assert_eq!(engage(&cooling), EngageDecision::EndCoolDownPursuit);
    }

    #[test]
    fn cooling_down_at_other_zones_or_without_the_prompt_flag_does_not_take_that_path() {
        let lot = EngageInput { pursuit: PursuitPhase::CoolingDown, zone: Some(ZoneKind::CarLot), ..input() };
        assert_eq!(engage(&lot), EngageDecision::PostEnterMessage(Some(ZoneKind::CarLot)));
        let not_ready = EngageInput { pursuit: PursuitPhase::CoolingDown, prompt_ready: false, ..input() };
        assert_eq!(engage(&not_ready), EngageDecision::Nothing);
    }

    #[test]
    fn an_event_underway_refuses_unless_the_node_allows_it() {
        let event = EngageInput { event_underway: true, ..input() };
        assert_eq!(engage(&event), EngageDecision::RefuseEventUnderway);
        let allowed = EngageInput { event_underway: true, node_allows_engage: true, ..input() };
        assert_eq!(engage(&allowed), EngageDecision::PostEnterMessage(Some(ZoneKind::SafeHouse)));
    }

    #[test]
    fn a_busy_controller_refuses_with_kind_one() {
        let busy = EngageInput { controller_busy: true, ..input() };
        assert_eq!(engage(&busy), EngageDecision::RefuseBusy);
    }

    #[test]
    fn no_prompt_means_nothing_happens() {
        assert_eq!(engage(&EngageInput { prompt_present: false, ..input() }), EngageDecision::Nothing);
        assert_eq!(engage(&EngageInput { prompt_ready: false, ..input() }), EngageDecision::Nothing);
    }

    #[test]
    fn the_cooldown_end_writes_three_fields() {
        let end = cooldown_end();
        assert!(end.status_evaded && end.enter_safe_house_when_done);
        assert_eq!(end.evade_level, 1.0);
    }

    #[test]
    fn closing_the_results_screen_enters_the_safe_house_once() {
        assert_eq!(results_screen_closed(true), (false, Some(ZoneKind::SafeHouse)));
        assert_eq!(results_screen_closed(false), (false, None));
    }

    #[test]
    fn the_cooldown_evade_costs_exactly_a_normal_evade() {
        let settings = CareerSettings::new_career();
        let mut stable = Stable::empty();
        for (slot, handle) in [(0usize, settings.car_handle), (1, 77)] {
            stable.cars[slot] = CarRecord { handle, flags: 0xF_0002, career_index: slot as u8, ..CarRecord::free() };
            stable.careers[slot] = CareerRecord { heat: 4.0, strikes: 2, decay: 2, ..CareerRecord::new(slot as u8) };
        }
        let result = PursuitResult {
            rep_points_normal: 1000,
            rep_points_destroyed: 0,
            infractions: InfractionMask::NONE,
            ..PursuitResult::default()
        };
        let end = finish_cooldown_pursuit(&settings, &mut stable, &result, 0.95);
        assert!(end.ticked);
        for record in &stable.careers[..2] {
            assert_eq!(record.heat, 4.0 * 0.95, "every car of the stable is multiplied");
            assert_eq!((record.strikes, record.decay), (1, 0), "the strike tick ran");
        }
        assert_eq!((stable.careers[0].evaded, stable.careers[0].bounty), (1, 1000));
        assert_eq!(stable.careers[1].evaded, 0, "only the current car is recorded");
    }
}
