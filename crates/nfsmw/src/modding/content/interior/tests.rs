use blackbox_modding::content::{Pose, Suspend, TimeOverride};

use super::machine::{Clock, Command, FADE_SECS, LOAD_LIMIT_SECS, Machine, Plan, Snapshot, traffic_while_inside};
use crate::settings::{Percent, TrafficSettings};
use crate::viewer::Environment;

const TILE: usize = 40;

fn traffic() -> TrafficSettings {
    TrafficSettings { cars: 12, cop_share: Percent(5), lights: true, avoidance: true }
}

fn snapshot() -> Snapshot {
    let clock = Clock { env: Environment { hour: 14.0, seconds_per_hour: 60.0 }, held: false };
    Snapshot { pos: [100.0, -50.0, 2.0], heading: 0.5, traffic: traffic(), clock }
}

fn plan() -> Plan {
    Plan {
        tile: TILE,
        spawn: Pose { pos: [3010.0, 4020.0, 0.0], heading: 90.0 },
        time: Some(TimeOverride { hour: 22.0, freeze: true, lights: None }),
        suspend: Suspend { traffic: true, cops: true, pursuit: true },
    }
}

/// Run the entry up to the moment the pack is resident and the car is in: the commands of the last step.
fn enter(machine: &mut Machine) -> Vec<Command> {
    machine.begin(plan(), snapshot());
    machine.tick(FADE_SECS, false);
    machine.tick(0.5, true)
}

#[test]
fn begin_pins_the_pack_and_starts_the_fade() {
    let mut machine = Machine::default();
    let commands = machine.begin(plan(), snapshot());
    assert_eq!(commands, [Command::Pin { tile: TILE, on: true }, Command::FadeOut]);
    assert!(!machine.is_outside(), "the fade is running: no entry card, and no second entry");
    assert_eq!(machine.loading_tile(), None);
}

#[test]
fn a_second_begin_while_going_in_does_nothing() {
    let mut machine = Machine::default();
    machine.begin(plan(), snapshot());
    assert!(machine.begin(plan(), snapshot()).is_empty());
}

#[test]
fn the_fade_then_the_wait_then_the_entry_run_in_order() {
    let mut machine = Machine::default();
    machine.begin(plan(), snapshot());
    assert!(machine.tick(0.2, false).is_empty(), "the fade is running");
    assert!(machine.tick(0.3, false).is_empty(), "the fade is done; the wait for the pack starts");
    assert_eq!(machine.loading_tile(), Some(TILE));
    assert!(machine.tick(0.1, false).is_empty(), "the pack is not resident yet");
    let entered = machine.tick(0.1, true);
    assert_eq!(
        entered,
        [
            Command::Teleport { pos: [3010.0, 4020.0, 0.0], heading: 90.0_f32.to_radians() },
            Command::Suspend(Suspend { traffic: true, cops: true, pursuit: true }),
            Command::SetClock(TimeOverride { hour: 22.0, freeze: true, lights: None }),
            Command::FadeIn,
        ]
    );
    assert!(machine.is_inside());
    assert_eq!(machine.loading_tile(), None);
}

#[test]
fn the_pack_being_resident_at_once_enters_without_a_wait() {
    let mut machine = Machine::default();
    machine.begin(plan(), snapshot());
    let entered = machine.tick(FADE_SECS, true);
    assert!(entered.iter().any(|c| matches!(c, Command::Teleport { .. })));
    assert!(machine.is_inside());
}

#[test]
fn a_pack_that_never_arrives_cancels_the_entry_and_leaves_the_car_alone() {
    let mut machine = Machine::default();
    machine.begin(plan(), snapshot());
    machine.tick(FADE_SECS, false);
    let waited_ticks = LOAD_LIMIT_SECS as usize;
    let mut last = Vec::new();
    for _ in 0..waited_ticks {
        last = machine.tick(1.0, false);
    }
    assert!(machine.is_outside());
    assert!(last.iter().any(|c| matches!(c, Command::Notice(_))));
    assert!(last.contains(&Command::Pin { tile: TILE, on: false }));
    assert!(!last.iter().any(|c| matches!(c, Command::Teleport { .. } | Command::Suspend(_) | Command::Restore(_))));
}

#[test]
fn traffic_is_suspended_and_the_clock_overridden_inside_and_both_come_back_on_exit() {
    let mut machine = Machine::default();
    enter(&mut machine);
    assert!(machine.is_inside());

    assert_eq!(machine.leave(), [Command::FadeOut]);
    assert!(machine.tick(FADE_SECS - 0.1, true).is_empty(), "the fade out is still running");
    let exit = machine.tick(0.2, true);
    assert!(machine.is_outside());
    assert_eq!(
        exit,
        [
            Command::Teleport { pos: [100.0, -50.0, 2.0], heading: 0.5 },
            Command::Restore(snapshot()),
            Command::Pin { tile: TILE, on: false },
            Command::FadeIn,
        ]
    );
    let Command::Restore(back) = &exit[1] else { panic!("the second step restores") };
    assert_eq!(back.traffic.cars, 12, "the traffic count comes back");
    assert_eq!(back.clock.env.hour, 14.0, "the hour comes back");
}

#[test]
fn leaving_is_only_possible_from_inside() {
    let mut machine = Machine::default();
    assert!(machine.leave().is_empty());
    machine.begin(plan(), snapshot());
    assert!(machine.leave().is_empty(), "a fade in is not an exit");
}

#[test]
fn traffic_while_inside_stops_the_named_parts_and_keeps_the_rest() {
    let all = Suspend { traffic: true, cops: true, pursuit: true };
    let quiet = traffic_while_inside(traffic(), all);
    assert_eq!(quiet.cars, 0);
    assert_eq!(quiet.cop_share, Percent(0));
    assert!(quiet.lights && quiet.avoidance, "the other settings are kept");

    let cops_only = Suspend { traffic: false, cops: true, pursuit: false };
    let kept = traffic_while_inside(traffic(), cops_only);
    assert_eq!(kept.cars, 12, "traffic runs when it is not named");
    assert_eq!(kept.cop_share, Percent(0));
}
