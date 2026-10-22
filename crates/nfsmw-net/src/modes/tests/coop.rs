use super::*;
use crate::message::PursuitEndReason;
use crate::results::{FLAG_BUSTED, FLAG_LEFT};

fn coop(players: &[u8], cap: u8, carried: &[(u8, u8)]) -> Run {
    let players = players.iter().copied().map(PlayerId).collect();
    let mut setup = ModeSetup::new(Mode::PursuitCoop, CustomRules { cop_level_cap: cap, ..rules(0) }, players, START);
    setup.carried_heat = carried.iter().map(|(player, heat)| (p(*player), *heat)).collect();
    Run::with(setup)
}

fn chase(run: &mut Run, player: u8) -> Vec<ModeEvent> {
    run.sim(SimEvent::PursuitStarted { player: p(player) })
}

#[test]
fn everyone_busted_ends_the_round_and_busted_players_spectate() {
    let mut run = coop(&[1, 2], 5, &[]);
    assert_eq!(chase(&mut run, 1), [ModeEvent::PursuitStarted { pursuit: 1, targets: vec![p(1)], heat: 1 }]);
    chase(&mut run, 2);
    let first = run.sim(SimEvent::Busted { player: p(1) });
    assert_eq!(
        first,
        [
            ModeEvent::TargetEnded { pursuit: 1, player: p(1), reason: PursuitEndReason::Busted },
            ModeEvent::Spectate { player: p(1) },
        ]
    );
    assert!(!run.mode.is_over(), "the chase goes on for player 2");
    let last = run.sim(SimEvent::Busted { player: p(2) });
    assert!(last.contains(&ModeEvent::PursuitEnded { pursuit: 1, reason: PursuitEndReason::Busted }));
    assert_eq!(over(&last), Some(OverReason::AllBusted));
    assert!(run.mode.placings().iter().all(|placing| placing.flags == FLAG_BUSTED && placing.place == 0));
}

#[test]
fn an_evade_ends_the_chase_for_every_target() {
    let mut run = coop(&[1, 2, 3], 5, &[]);
    chase(&mut run, 1);
    chase(&mut run, 2);
    let evaded = run.sim(SimEvent::Evaded { player: p(1) });
    let ended: Vec<u8> = evaded
        .iter()
        .filter_map(|event| match event {
            ModeEvent::TargetEnded { player, reason: PursuitEndReason::Evaded, .. } => Some(player.0),
            _ => None,
        })
        .collect();
    assert_eq!(ended, [1, 2], "the shared cool-down frees every chased car");
    assert_eq!(over(&evaded), Some(OverReason::Evaded));
    assert!(run.sim(SimEvent::PursuitEnded { player: p(1) }).is_empty(), "the round is over");
}

#[test]
fn heat_starts_at_the_higher_carried_heat_and_stays_under_the_cap() {
    let mut run = coop(&[1, 2], 4, &[(1, 2), (2, 5)]);
    assert_eq!(chase(&mut run, 1), [ModeEvent::PursuitStarted { pursuit: 1, targets: vec![p(1)], heat: 2 }]);
    assert_eq!(
        chase(&mut run, 2),
        [ModeEvent::TargetAdded { pursuit: 1, player: p(2), targets: vec![p(1), p(2)], heat: 4 }]
    );
    assert!(run.feed(ModeInput::Heat { level: 9 }).is_empty(), "already at the cap of 4");
    assert_eq!(
        run.feed(ModeInput::Heat { level: 3 }),
        [ModeEvent::HeatChanged { pursuit: 1, level: 3, targets: vec![p(1), p(2)] }]
    );
}

#[test]
fn a_busted_player_does_not_rejoin_the_same_pursuit() {
    let mut run = coop(&[1, 2], 5, &[]);
    chase(&mut run, 1);
    chase(&mut run, 2);
    run.sim(SimEvent::Busted { player: p(1) });
    assert!(chase(&mut run, 1).is_empty());
    assert!(run.sim(SimEvent::Busted { player: p(1) }).is_empty(), "a second bust is not counted");
}

#[test]
fn a_chased_player_who_disconnects_abandons_and_the_last_one_ends_the_round() {
    let mut run = coop(&[1, 2], 5, &[]);
    chase(&mut run, 1);
    chase(&mut run, 2);
    let first = run.leave(1);
    assert_eq!(first, [ModeEvent::TargetEnded { pursuit: 1, player: p(1), reason: PursuitEndReason::Abandoned }]);
    let last = run.leave(2);
    assert!(last.contains(&ModeEvent::PursuitEnded { pursuit: 1, reason: PursuitEndReason::Abandoned }));
    assert_eq!(over(&last), Some(OverReason::Abandoned));
    assert!(run.mode.placings().iter().all(|placing| placing.flags == FLAG_LEFT));
}

#[test]
fn a_bust_then_a_departure_still_ends_busted() {
    let mut run = coop(&[1, 2], 5, &[]);
    chase(&mut run, 1);
    chase(&mut run, 2);
    run.sim(SimEvent::Busted { player: p(1) });
    assert_eq!(over(&run.leave(2)), Some(OverReason::AllBusted));
}

#[test]
fn everyone_leaving_before_a_chase_ends_the_round_empty() {
    let mut run = coop(&[1, 2], 5, &[]);
    assert!(run.leave(1).is_empty());
    assert_eq!(run.leave(2), [ModeEvent::Over { reason: OverReason::Empty }]);
}

#[test]
fn free_roam_without_pursuits_has_no_police() {
    let mut setup = ModeSetup::new(Mode::FreeRoam, CustomRules { pursuits: false, ..rules(0) }, vec![p(1)], START);
    setup.carried_heat = vec![(p(1), 3)];
    let mut run = Run::with(setup);
    assert!(!run.mode.police());
    assert!(chase(&mut run, 1).is_empty());
    assert!(!run.mode.is_over());
}

#[test]
fn free_roam_runs_one_pursuit_after_another_and_never_ends() {
    let mut run = Run::new(Mode::FreeRoam, 0, &[1, 2]);
    assert!(run.mode.police());
    chase(&mut run, 1);
    let busted = run.sim(SimEvent::Busted { player: p(1) });
    assert!(!busted.contains(&ModeEvent::Spectate { player: p(1) }), "a busted roamer respawns and plays on");
    assert_eq!(chase(&mut run, 1), [ModeEvent::PursuitStarted { pursuit: 2, targets: vec![p(1)], heat: 1 }]);
    run.leave(1);
    run.leave(2);
    assert!(!run.mode.is_over());
    assert!(run.mode.placings().iter().all(|placing| placing.flags == FLAG_LEFT && placing.place == 0));
}
