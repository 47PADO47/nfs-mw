use blackbox_pursuit::CooldownStatus;
use nfsmw_career::safehouse::PursuitPhase;
use nfsmw_career::safehouse::engage::EngageDecision;

use super::*;

#[test]
fn the_phase_follows_the_chase_and_its_cool_down() {
    assert_eq!(pursuit_phase(false, CooldownStatus::InitialChase), PursuitPhase::None);
    assert_eq!(pursuit_phase(true, CooldownStatus::InitialChase), PursuitPhase::Active);
    assert_eq!(pursuit_phase(true, CooldownStatus::CoolDown), PursuitPhase::CoolingDown);
}

#[test]
fn a_safe_house_with_no_chase_is_entered_at_once() {
    let decision = safe_house_decision(PursuitPhase::None);
    assert_eq!(decision, Some(EngageDecision::PostEnterMessage(Some(SafeZoneKind::SafeHouse))));
}

#[test]
fn a_safe_house_while_cooling_down_ends_the_chase_first() {
    assert_eq!(safe_house_decision(PursuitPhase::CoolingDown), Some(EngageDecision::EndCoolDownPursuit));
}

#[test]
fn a_safe_house_with_a_live_chase_is_refused() {
    assert_eq!(safe_house_decision(PursuitPhase::Active), None);
}

#[test]
fn an_auto_shop_entry_asks_for_the_garage_by_zone_name() {
    assert_eq!(auto_shop_request("downtown"), CareerRequest::AutoShop { zone: "downtown".to_owned() });
}

#[test]
fn the_engage_input_is_a_ready_prompt_at_a_safe_house() {
    let input = safe_house_input(PursuitPhase::None);
    assert!(input.prompt_present && input.prompt_ready);
    assert_eq!(input.zone, Some(SafeZoneKind::SafeHouse));
    assert!(!input.event_underway && !input.node_allows_engage && !input.controller_busy);
}
