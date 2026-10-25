use super::*;

/// A running chase with this cool-down status and contact.
fn chase(status: CooldownStatus, contact: bool) -> PhaseInput {
    PhaseInput { pursuing: true, status, contact, arrest: false, end: EndReason::None, since_end: 0.0 }
}

/// No chase, the last one ended for `end` this many seconds ago.
fn ended(end: EndReason, since_end: f32) -> PhaseInput {
    PhaseInput { pursuing: false, status: CooldownStatus::InitialChase, contact: false, arrest: false, end, since_end }
}

#[test]
fn the_table_maps_every_state() {
    use CooldownStatus::*;
    let rows = [
        (chase(InitialChase, true), PursuitPhase::Active),
        (chase(InitialChase, false), PursuitPhase::Search),
        (chase(CoolDown, false), PursuitPhase::Cooldown),
        (chase(CoolDown, true), PursuitPhase::Cooldown),
        (chase(Evaded, false), PursuitPhase::Escaped),
        (ended(EndReason::Evaded, 1.0), PursuitPhase::Escaped),
        (ended(EndReason::Busted, 1.0), PursuitPhase::Busted),
        (ended(EndReason::None, 1.0), PursuitPhase::None),
        (ended(EndReason::Evaded, END_HOLD), PursuitPhase::None),
        (ended(EndReason::Busted, END_HOLD + 5.0), PursuitPhase::None),
    ];
    for (input, expected) in rows {
        assert_eq!(phase(&input), expected, "{input:?}");
    }
}

#[test]
fn a_chase_runs_active_then_search_then_cooldown_then_escaped_then_none() {
    use CooldownStatus::*;
    let steps = [
        chase(InitialChase, true),
        chase(InitialChase, false),
        chase(CoolDown, false),
        chase(Evaded, false),
        ended(EndReason::Evaded, 0.0),
        ended(EndReason::Evaded, END_HOLD + 0.1),
    ];
    let phases: Vec<PursuitPhase> = steps.iter().map(phase).collect();
    use PursuitPhase as P;
    assert_eq!(phases, [P::Active, P::Search, P::Cooldown, P::Escaped, P::Escaped, P::None]);
}

#[test]
fn a_sighting_reacquires_the_player() {
    use CooldownStatus::*;
    // A cop sees the player during the search: active again at once.
    assert_eq!(phase(&chase(InitialChase, false)), PursuitPhase::Search);
    assert_eq!(phase(&chase(InitialChase, true)), PursuitPhase::Active);
    // During the cool-down the display's gate holds it, then the status falls back to the chase.
    assert_eq!(phase(&chase(CoolDown, true)), PursuitPhase::Cooldown);
    assert_eq!(phase(&chase(InitialChase, true)), PursuitPhase::Active);
}

#[test]
fn the_bust_sequence_is_busted_whatever_else_holds() {
    let arrest = PhaseInput { arrest: true, ..chase(CooldownStatus::InitialChase, true) };
    assert_eq!(phase(&arrest), PursuitPhase::Busted);
    let ticket = PhaseInput { arrest: true, ..ended(EndReason::Busted, END_HOLD + 30.0) };
    assert_eq!(phase(&ticket), PursuitPhase::Busted, "the ticket can outlast the end card");
}

#[test]
fn the_heat_is_not_an_input() {
    // Nothing but the pursuit's state decides: no chase and no end card is no phase, whatever the car's heat.
    assert_eq!(phase(&ended(EndReason::None, 0.0)), PursuitPhase::None);
}
