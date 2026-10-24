use super::*;
use crate::scenes::world::StateHasher;

fn input() -> Input {
    Input { boost: 1.0, bottle_seconds: 120.0, capacity: 1.0, was_on: false }
}

fn decision() -> Decision {
    Decision {
        speed: 20.0,
        requested_speed: 40.0,
        potential_speed: 70.0,
        potential_acceleration: 6.0,
        actual_acceleration: 0.0,
        occluded: false,
        stopping: false,
        skill: 1.0,
    }
}

fn rested() -> State {
    State { active: true, requested: false, timer: -4.01 }
}

#[test]
fn starting_requires_the_strict_rest_capacity_and_deficit_thresholds() {
    assert!(!State { timer: -4.0, ..rested() }.choose(input(), 1.0, &decision()));
    assert!(rested().choose(input(), 1.0, &decision()));
    assert!(!rested().choose(Input { capacity: 0.3, ..input() }, 1.0, &decision()));
    assert!(rested().choose(Input { capacity: 0.31, ..input() }, 1.0, &decision()));
    assert!(!rested().choose(input(), 1.0, &Decision { potential_speed: 35.0, ..decision() }));
    assert!(!rested().choose(input(), 1.0, &Decision { actual_acceleration: 3.0, ..decision() }));
    assert!(!rested().choose(input(), 1.0, &Decision { speed: 9.99, ..decision() }));
    assert!(!rested().choose(input(), 1.0, &Decision { requested_speed: 20.0, ..decision() }));
}

#[test]
fn missing_capability_empty_bottles_and_start_occlusion_refuse_nitrous() {
    assert!(!rested().choose(input(), 0.0, &decision()), "player matching retains the zero capability");
    assert!(!rested().choose(Input { boost: 0.0, ..input() }, 1.0, &decision()));
    assert!(!rested().choose(Input { bottle_seconds: 0.0, ..input() }, 1.0, &decision()));
    assert!(!rested().choose(Input { capacity: 0.0, was_on: true, ..input() }, 1.0, &decision()));
    assert!(!rested().choose(input(), 1.0, &Decision { occluded: true, ..decision() }));
}

#[test]
fn an_active_burn_uses_the_lower_capacity_threshold_and_minimum_burn_interval() {
    let held = Input { was_on: true, capacity: 0.01, ..input() };
    let almost_caught = Decision { potential_speed: 24.0, occluded: true, ..decision() };
    let burning = State { timer: 4.0, ..rested() };
    assert!(burning.choose(held, 1.0, &almost_caught), "strictly more than four seconds precedes deficit release");
    assert!(!State { timer: 4.01, ..burning }.choose(held, 1.0, &almost_caught));
    assert!(
        !State { timer: 4.01, ..rested() }.choose(held, 1.0, &Decision { actual_acceleration: 5.9, ..decision() },)
    );
    assert!(!rested().choose(held, 1.0, &Decision { stopping: true, ..decision() }));
    assert!(!rested().choose(Input { capacity: 0.0, ..held }, 1.0, &decision()));
}

#[test]
fn only_previous_held_control_with_remaining_capacity_boosts_acceleration() {
    assert_eq!(input().acceleration_boost(1.0), None);
    let held = Input { was_on: true, ..input() };
    assert_eq!(held.acceleration_boost(0.5), Some(0.5));
    assert_eq!(held.acceleration_boost(2.0), Some(1.0));
    assert_eq!(held.acceleration_boost(0.0), Some(0.0));
    assert_eq!(Input { capacity: 0.0, ..held }.acceleration_boost(1.0), None);
}

fn hash(state: &State) -> u64 {
    let mut hash = StateHasher::default();
    state.hash_into(&mut hash);
    hash.finish()
}

#[test]
fn action_entry_resets_the_timer_and_all_decision_state_participates_in_hashes() {
    let mut state = rested();
    state.update(input(), 1.0, decision(), 0.1);
    assert!(state.requested());
    assert_eq!(state.timer, 0.0, "the inactive interval does not count toward the active burn");
    assert_ne!(hash(&state), hash(&State { requested: false, ..state.clone() }));
    assert_ne!(hash(&state), hash(&State { active: false, ..state.clone() }));
    assert_ne!(hash(&state), hash(&State { timer: 0.1, ..state.clone() }));
    state.leave();
    assert!(!state.active() && !state.requested());
    state.begin();
    assert!(state.active() && !state.requested());
    assert_eq!(state.timer, 0.0);
    assert_eq!(hash(&state), hash(&State { active: true, ..Default::default() }));
}
