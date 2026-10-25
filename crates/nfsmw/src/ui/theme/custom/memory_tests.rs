//! Tests for the layouts' memory: one advance per frame clock, the marks of a race and a chase, and the reset on the
//! way back to free roam.

use std::sync::Arc;

use super::{LayoutMemory, Mark, heat_level, mark};
use crate::hud::{HudContext, HudState, NavMarker, NavMarkerKind, PursuitPhase, RaceHud, RaceStage};
use crate::ui::theme::custom::feed::CustomFeed;
use crate::ui::theme::custom::mode::HudMode;

fn feed(hud: HudState, now: f32) -> CustomFeed {
    CustomFeed { hud: Arc::new(hud), elapsed_s: now, ..CustomFeed::default() }
}

fn racing(place: u32) -> HudState {
    HudState {
        race: Some(RaceHud { place, racers: 4, lap: Some((1, 2)), clock: 0.0, progress: 0.0, count: None }),
        context: HudContext { race_stage: Some(RaceStage::Running), ..HudContext::default() },
        ..HudState::default()
    }
}

#[test]
fn a_mark_keeps_its_time_until_the_value_changes() {
    let first = mark(None, Some(3), 1.0);
    assert_eq!(first, Some(Mark { value: 3, since: 1.0, previous: None }));
    assert_eq!(mark(first, Some(3), 2.0), first, "the same value keeps the first time");
    assert_eq!(mark(first, Some(2), 4.0), Some(Mark { value: 2, since: 4.0, previous: Some(3) }));
    assert_eq!(mark(first, None, 5.0), None, "no value forgets it");
}

#[test]
fn the_memory_advances_once_per_frame_clock() {
    let memory = LayoutMemory::default();
    let first = memory.frame(&feed(racing(3), 1.0));
    assert_eq!(first.mode(), HudMode::Racing);
    // A second read on the same clock with another state is the same frame.
    let again = memory.frame(&feed(racing(1), 1.0));
    assert_eq!(again, first);
    let next = memory.frame(&feed(racing(1), 2.0));
    let place = next.marks.place.expect("a race has a place");
    assert_eq!((place.value, place.since, place.previous), (1, 2.0, Some(3)), "the place changed at 2 s");
}

#[test]
fn returning_to_free_roam_resets_the_race_and_the_chase() {
    let memory = LayoutMemory::default();
    memory.frame(&feed(racing(2), 1.0));
    let chase = HudState { pursuit_phase: PursuitPhase::Active, vehicle_heat: 2.0, ..HudState::default() };
    memory.frame(&feed(chase, 2.0));
    let free = memory.frame(&feed(HudState::default(), 3.0));
    assert_eq!(free.mode(), HudMode::FreeRoam);
    assert_eq!(free.marks, Default::default(), "nothing of the race or the chase is kept");
    // A restarted race starts its strip afresh: its place is a first one, with no change to pulse.
    let restarted = memory.frame(&feed(racing(4), 4.0));
    assert_eq!(restarted.marks.place.map(|mark| mark.previous), Some(None));
}

#[test]
fn the_heat_level_is_the_whole_multiplier_while_a_chase_runs() {
    let chase = |phase, heat| HudState { pursuit_phase: phase, vehicle_heat: heat, ..HudState::default() };
    assert_eq!(heat_level(&chase(PursuitPhase::Active, 3.8)), Some(3));
    assert_eq!(heat_level(&chase(PursuitPhase::Cooldown, 1.0)), Some(1));
    assert_eq!(heat_level(&chase(PursuitPhase::None, 3.8)), None, "no chase, no heat field");
    assert_eq!(heat_level(&chase(PursuitPhase::Escaped, 3.8)), None, "the panel has left");
    assert_eq!(heat_level(&chase(PursuitPhase::Active, 0.5)), None);
}

#[test]
fn discovery_chips_come_up_in_free_roam_only() {
    let shop = NavMarker { kind: NavMarkerKind::AutoShop, position: [30.0, 0.0], name: None, discovered: true };
    let roaming = HudState {
        markers: vec![shop],
        context: HudContext { player: Some([0.0, 0.0]), ..HudContext::default() },
        ..HudState::default()
    };
    let memory = LayoutMemory::default();
    assert_eq!(memory.frame(&feed(roaming.clone(), 1.0)).chips.len(), 1);
    let chased = HudState { pursuit_phase: PursuitPhase::Active, ..roaming };
    let frame = memory.frame(&feed(chased, 2.0));
    assert!(frame.chips.iter().all(|chip| chip.left.is_some()), "a chase sends the chips away");
}
