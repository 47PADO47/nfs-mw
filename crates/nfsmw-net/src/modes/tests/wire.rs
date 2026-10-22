use super::*;
use crate::message::{
    HeatChange, LapComplete, Message, PursuitEnd, PursuitEndReason, PursuitStart, Role, RoleChanged, decode, encode,
};
use crate::results::{Roster, Seat};

fn roster() -> Roster {
    let mut roster = Roster::new();
    assert!(roster.insert(Seat { player: p(1), slot: 0, entity: 10 }));
    assert!(roster.insert(Seat { player: p(2), slot: 5, entity: 20 }));
    roster
}

fn round_trip(message: &Message) {
    let mut bytes = Vec::new();
    encode(message, &mut bytes).expect("encodes");
    assert_eq!(&decode(&bytes).expect("decodes"), message);
}

#[test]
fn a_lap_becomes_lap_complete_with_the_cars_entity() {
    let event = ModeEvent::LapCompleted { player: p(2), lap: 3, time_ms: 41_000, best: true };
    let messages = messages(&event, 900, &roster());
    let fields = match &messages[..] {
        [Message::LapComplete(LapComplete { head, entity_id, lap, time_ms })] => {
            (head.tick, head.seq, *entity_id, *lap, *time_ms)
        }
        _ => panic!("{messages:?}"),
    };
    assert_eq!(fields, (900, 0, 20, 3, 41_000));
    round_trip(&messages[0]);
}

#[test]
fn spectate_becomes_a_role_change_of_the_slot() {
    let messages = messages(&ModeEvent::Spectate { player: p(2) }, 5, &roster());
    assert!(matches!(&messages[..], [Message::RoleChanged(RoleChanged { slot: 5, role: Role::Spectator, .. })]));
    round_trip(&messages[0]);
}

#[test]
fn pursuit_events_name_the_chased_cars() {
    let roster = roster();
    let added = ModeEvent::TargetAdded { pursuit: 3, player: p(2), targets: vec![p(1), p(2)], heat: 4 };
    let start = messages(&added, 7, &roster);
    let fields = match &start[..] {
        [Message::PursuitStart(PursuitStart { pursuit_id, targets, heat, .. })] => {
            (*pursuit_id, targets.to_vec(), *heat)
        }
        _ => panic!("{start:?}"),
    };
    assert_eq!(fields, (3, vec![10u16, 20], 4));
    round_trip(&start[0]);
    let heat = messages(&ModeEvent::HeatChanged { pursuit: 3, level: 5, targets: vec![p(1), p(2)] }, 8, &roster);
    let levels: Vec<(u16, u8)> = heat
        .iter()
        .map(|message| match message {
            Message::HeatChange(HeatChange { entity_id, level, .. }) => (*entity_id, *level),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(levels, [(10, 5), (20, 5)], "one heat change per chased car");
    let ended = ModeEvent::TargetEnded { pursuit: 3, player: p(1), reason: PursuitEndReason::Busted };
    let end = messages(&ended, 9, &roster);
    assert!(matches!(&end[..], [Message::PursuitEnd(PursuitEnd { entity_id: 10, pursuit_id: 3, .. })]));
}

#[test]
fn events_without_a_message_and_unseated_players_give_nothing() {
    let roster = roster();
    let none = [
        ModeEvent::KnockedOut { player: p(1), lap: 1 },
        ModeEvent::TagPassed { from: None, to: p(1) },
        ModeEvent::Penalty { player: p(1), ms: 5_000 },
        ModeEvent::Over { reason: OverReason::Finished },
        ModeEvent::PursuitEnded { pursuit: 1, reason: PursuitEndReason::Evaded },
        ModeEvent::Finished { player: p(9), place: 1, time_ms: 1 },
    ];
    for event in none {
        assert!(messages(&event, 1, &roster).is_empty(), "{event:?}");
    }
}
