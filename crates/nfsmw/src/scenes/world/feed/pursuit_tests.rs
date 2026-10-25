//! The notices of a change between two chase snapshots, checked on plain values (no world, no mods).

use blackbox_pursuit::{EndReason, Fines, Infraction, InfractionLog};
use nfsmw_modding::abi::{EndReason as MwEndReason, Infraction as MwInfraction};

use super::{Notice, PursuitView, observe};

/// The install's fines in tests: speeding 100, the rest 10 more each.
fn fines() -> Fines {
    Fines::new([100, 110, 120, 130, 140, 150, 160, 170])
}

/// A chase at heat `heat` (its level is the integer part), `secs` in.
fn chase(heat: f32, secs: f32) -> PursuitView {
    PursuitView { pursuing: true, heat, level: heat as u32, secs, wave_cops: 4, cops: 4, ..PursuitView::idle(fines()) }
}

/// The log of `infractions`, recorded in order.
fn log(infractions: &[Infraction]) -> InfractionLog {
    let mut log = InfractionLog::default();
    for infraction in infractions {
        log.record(*infraction, &fines());
    }
    log
}

#[test]
fn a_chase_that_starts_says_its_level_and_the_wave() {
    let idle = PursuitView::idle(fines());
    let now = PursuitView { wave_cops: 6, ..chase(3.2, 0.0) };
    assert_eq!(
        observe(&idle, &now),
        [Notice::PursuitStarted { level: 3, cops: 6, heat: 3.2 }],
        "a chase starts with the cops of its first wave"
    );
}

#[test]
fn a_move_to_another_level_says_both_heat_values() {
    let before = chase(3.9, 20.0);
    let now = chase(4.1, 20.5);
    assert_eq!(observe(&before, &now), [Notice::HeatChanged { level: 4, old_heat: 3.9, new_heat: 4.1 }]);
}

#[test]
fn growth_within_a_level_is_not_a_heat_change() {
    assert!(observe(&chase(3.2, 20.0), &chase(3.4, 20.1)).is_empty());
}

#[test]
fn a_new_infraction_is_recorded_once_with_its_fine() {
    let before = PursuitView { infractions: log(&[]), ..chase(2.0, 5.0) };
    let now = PursuitView { infractions: log(&[Infraction::Speeding]), ..chase(2.0, 6.0) };
    assert_eq!(observe(&before, &now), [Notice::InfractionRecorded { infraction: MwInfraction::Speeding, fine: 100 }]);
    let later = PursuitView { infractions: log(&[Infraction::Speeding]), ..chase(2.0, 7.0) };
    assert!(observe(&now, &later).is_empty(), "an offence already in the log is not told again");
}

#[test]
fn the_last_chases_log_does_not_leak_into_the_next() {
    let idle = PursuitView { infractions: log(&[Infraction::Racing]), ..PursuitView::idle(fines()) };
    let started = PursuitView { infractions: log(&[]), ..chase(1.0, 0.0) };
    assert_eq!(observe(&idle, &started), [Notice::PursuitStarted { level: 1, cops: 4, heat: 1.0 }]);
}

#[test]
fn a_bust_is_told_in_its_frame_and_the_end_follows_it() {
    let before = PursuitView { infractions: log(&[Infraction::Speeding]), ..chase(5.0, 60.0) };
    let busted = PursuitView { busted: true, busted_meter: 1.0, ..before };
    assert_eq!(observe(&before, &busted), [Notice::Busted { level: 5, bounty: 100 }]);
    let ended = PursuitView {
        pursuing: false,
        heat: 0.0,
        level: 0,
        cops: 0,
        wave_cops: 0,
        end_reason: EndReason::Busted,
        ..busted
    };
    assert_eq!(
        observe(&busted, &ended),
        [Notice::PursuitEnded { reason: MwEndReason::Busted, bounty: 100, duration: 60.0 }]
    );
}

#[test]
fn an_escape_is_told_at_the_level_it_happened_then_the_end() {
    let before = chase(4.0, 90.0);
    let ended =
        PursuitView { pursuing: false, end_reason: EndReason::Evaded, secs: 91.0, ..PursuitView::idle(fines()) };
    assert_eq!(
        observe(&before, &ended),
        [
            Notice::Escaped { level: 4, duration: 91.0 },
            Notice::PursuitEnded { reason: MwEndReason::Evaded, bounty: 0, duration: 91.0 },
        ]
    );
}

#[test]
fn a_chase_stopped_without_a_result_ends_aborted() {
    let before = chase(2.0, 10.0);
    let ended = PursuitView { pursuing: false, secs: 10.5, ..PursuitView::idle(fines()) };
    assert_eq!(
        observe(&before, &ended),
        [Notice::PursuitEnded { reason: MwEndReason::Aborted, bounty: 0, duration: 10.5 }]
    );
}

#[test]
fn nothing_changes_between_two_idle_snapshots() {
    let idle = PursuitView { infractions: log(&[Infraction::Racing]), ..PursuitView::idle(fines()) };
    assert!(observe(&idle, &idle).is_empty(), "the old chase's log is not told again once it is over");
}
