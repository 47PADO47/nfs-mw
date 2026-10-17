use super::*;

#[test]
fn the_gate_set_holds_only_gates_10_to_15() {
    let mut s = GateSet::of(&[9, 10, 15, 16, 200]);
    assert_eq!(s.numbers(), [10, 15]);
    s.insert(3);
    s.insert(12);
    assert_eq!(s.numbers(), [10, 12, 15]);
    assert!(!s.contains(9) && !s.contains(16) && s.contains(12));
    assert!(GateSet::EMPTY.is_empty());
}

#[test]
fn all_sixteen_stages_close_the_documented_set() {
    for stage in 1..=16u8 {
        let expected: &[u8] = match stage {
            13..=16 => &[11, 12, 13, 14, 15],
            9..=12 => &[10, 12, 13],
            _ => &[],
        };
        assert_eq!(closed_gates(stage).numbers(), expected, "stage {stage}");
    }
}

#[test]
fn the_bins_barriers_lists_of_the_install_give_the_same_sets_as_the_table() {
    // `_N` is BARRIER_LOCKED_N; file order does not matter.
    let lists: [(&[u8], &[&str]); 5] = [
        (
            &[16, 15, 13],
            &["BARRIER_LOCKED_15", "BARRIER_LOCKED_14", "BARRIER_LOCKED_11", "BARRIER_LOCKED_13", "BARRIER_LOCKED_12"],
        ),
        (
            &[14],
            &["BARRIER_LOCKED_12", "BARRIER_LOCKED_13", "BARRIER_LOCKED_11", "BARRIER_LOCKED_15", "BARRIER_LOCKED_14"],
        ),
        (&[12, 10], &["BARRIER_LOCKED_12", "BARRIER_LOCKED_13", "BARRIER_LOCKED_10"]),
        (&[11, 9], &["BARRIER_LOCKED_10", "BARRIER_LOCKED_13", "BARRIER_LOCKED_12"]),
        (&[8, 7, 6, 5, 4, 3, 2, 1], &[]),
    ];
    for (stages, list) in lists {
        for &stage in stages {
            assert_eq!(gates_in_list(list.iter().copied()), closed_gates(stage), "stage {stage}");
        }
    }
}

#[test]
fn a_stage_that_is_no_bin_closes_nothing() {
    assert!(closed_gates(0).is_empty());
    for stage in 17..=21u8 {
        assert!(closed_gates(stage).is_empty());
    }
}

#[test]
fn the_gates_that_change_at_each_stage_step() {
    let step = |from: u8, to: u8| apply_stage(Some(from), to);
    let at_12 = step(13, 12);
    assert_eq!(at_12.opened.numbers(), [11, 14, 15], "stage 12 opens 11, 14, 15");
    assert_eq!(at_12.closed.numbers(), [10], "and gate 10 closes");
    let at_8 = step(9, 8);
    assert_eq!(at_8.opened.numbers(), [10, 12, 13], "stage 8 opens 10, 12, 13");
    assert!(at_8.closed.is_empty());
    for (from, to) in [(16, 15), (15, 14), (14, 13), (12, 11), (11, 10), (10, 9), (8, 7), (2, 1)] {
        let change = step(from, to);
        assert!(change.opened.is_empty() && change.closed.is_empty(), "{from} -> {to}");
    }
}

#[test]
fn the_whole_descent_changes_the_gates_exactly_twice_plus_gate_10_once() {
    let mut changes = Vec::new();
    for stage in (1..=16u8).rev().skip(1) {
        let change = apply_stage(Some(stage + 1), stage);
        if !change.opened.is_empty() || !change.closed.is_empty() {
            changes.push((stage, change.opened.numbers(), change.closed.numbers()));
        }
    }
    assert_eq!(changes, [(12, vec![11, 14, 15], vec![10]), (8, vec![10, 12, 13], vec![])]);
}

#[test]
fn a_new_world_just_closes_the_stage_set() {
    let change = apply_stage(None, 12);
    assert_eq!(change.closed.numbers(), [10, 12, 13]);
    assert!(change.opened.is_empty());
    assert_eq!(apply_stage(None, 8), GateChange { opened: GateSet::EMPTY, closed: GateSet::EMPTY });
}

#[test]
fn barrier_entries_are_filtered_by_prefix_ignoring_case() {
    let ok =
        ["BARRIER_SPLINE_3", "BARRIERS_X", "PLAYER_BARRIERS_Y", "SCENERY_GROUP_DOOR", "FREE_ROAM", "barrier_locked_11"];
    for text in ok {
        assert!(parse_barrier_entry(text).is_some(), "{text}");
    }
    for text in ["", "ROAD_BARRIER_1", "BARRIER", "BARRIERX_", "SCENERY_", "XFREE_ROAM"] {
        assert_eq!(parse_barrier_entry(text), None, "{text}");
    }
    assert_eq!(
        parse_barrier_entry("*SCENERY_GROUP_DOOR"),
        Some(BarrierEntry { name: "SCENERY_GROUP_DOOR", star: true })
    );
    assert_eq!(parse_barrier_entry("BARRIER_SPLINE_1"), Some(BarrierEntry { name: "BARRIER_SPLINE_1", star: false }));
    assert_eq!(parse_barrier_entry("*"), None);
}

#[test]
fn gate_names_are_exact_and_case_sensitive() {
    assert_eq!(gate_number("BARRIER_LOCKED_10"), Some(10));
    assert_eq!(gate_number("BARRIER_LOCKED_15"), Some(15));
    assert_eq!(gate_number("BARRIER_LOCKED_9"), None);
    assert_eq!(gate_number("BARRIER_LOCKED_16"), None);
    assert_eq!(gate_number("barrier_locked_11"), None, "the hash is case sensitive");
    assert_eq!(gate_number("BARRIER_SPLINE_11"), None);
    assert_eq!(gates_in_list(["*BARRIER_LOCKED_11", "BARRIER_SPLINE_4", "junk"]).numbers(), [11]);
    assert_eq!(gates_in_list(["barrier_locked_11"]).numbers(), [], "passes the filter, matches no group");
}

#[test]
fn the_group_state_packs_three_bits() {
    assert_eq!(group_state(false, false), 1);
    assert_eq!(group_state(true, false), 3);
    assert_eq!(group_state(false, true), 5);
    assert_eq!(group_state(true, true), 7);
}

#[test]
fn collision_includes_a_group_instance_only_when_enabled() {
    assert!(instance_blocks(0, 0), "an instance with no group always counts");
    assert!(!instance_blocks(200, 0), "an open gate is ignored");
    assert!(instance_blocks(200, group_state(false, false)), "a closed gate blocks");
}

#[test]
fn the_gate_table_has_the_install_group_numbers_and_piece_counts() {
    assert_eq!(GATES.len(), 6);
    let by_gate = |g: u8| GATES.iter().find(|(n, _, _)| *n == g).copied().unwrap();
    assert_eq!(by_gate(13), (13, 200, 5));
    assert_eq!(by_gate(12), (12, 356, 7));
    assert_eq!(by_gate(11), (11, 389, 5));
    assert_eq!(by_gate(14), (14, 495, 7));
    assert_eq!(by_gate(15), (15, 496, 5));
    assert_eq!(by_gate(10), (10, 624, 3));
}

#[test]
fn challenge_series_races_list_their_own_gates() {
    for c in 1..=5 {
        assert_eq!(challenge_gates(c), GateSet::NORTH_WEST);
    }
    for c in 6..=15 {
        assert_eq!(challenge_gates(c), GateSet::MIDDLE);
    }
    assert!(challenge_gates(0).is_empty() && challenge_gates(16).is_empty());
}

#[test]
fn creating_a_world_clears_and_applies_the_stage_without_an_event_node() {
    let mut g = RegionGates::new();
    g.world_created(12, false);
    assert_eq!(g.closed().numbers(), [10, 12, 13]);
    assert_eq!(g.stored_stage, Some(12));
    g.world_created(16, true);
    assert_eq!(g.closed().numbers(), [10, 12, 13], "an event node: nothing happens at creation");
    g.world_created(8, false);
    assert!(g.closed().is_empty());
}

#[test]
fn a_loaded_save_at_stage_12_gets_its_gates_at_world_creation() {
    let mut g = RegionGates::new();
    g.world_created(12, false);
    assert_eq!(g.region_gates().numbers(), [10, 12, 13]);
}

#[test]
fn a_stage_change_is_applied_at_the_next_roaming_update_only() {
    let mut g = RegionGates::new();
    g.world_created(13, false);
    g.stage_changed();
    assert!(g.bin_changed);
    assert_eq!(g.closed().numbers(), [11, 12, 13, 14, 15], "nothing moves at the change");
    assert_eq!(g.roaming_update(false, 12), None, "not in the roaming state");
    assert!(g.bin_changed);
    let change = g.roaming_update(true, 12).unwrap();
    assert!(!g.bin_changed);
    assert_eq!(change.opened.numbers(), [11, 14, 15]);
    assert_eq!(change.closed.numbers(), [10]);
    assert_eq!(g.closed().numbers(), [10, 12, 13]);
    assert_eq!(g.roaming_update(true, 12), None, "once");
}

#[test]
fn set_current_bin_with_no_stored_record_does_not_clear_first() {
    let mut g = RegionGates::new();
    g.race_started(GateSet::of(&[10]));
    g.stage_changed();
    g.roaming_update(true, 16);
    assert!(g.closed().contains(10), "nothing was stored, so nothing is cleared");
    let mut stored = RegionGates::new();
    stored.world_created(16, false);
    stored.race_started(GateSet::of(&[10]));
    stored.stage_changed();
    stored.roaming_update(true, 16);
    assert!(!stored.closed().contains(10), "a stored record means clear, then enable");
}

#[test]
fn a_race_adds_its_gates_and_done_restores_the_stage_set() {
    let mut g = RegionGates::new();
    g.world_created(8, false);
    assert!(g.closed().is_empty());
    g.race_started(challenge_gates(1));
    assert_eq!(g.closed(), GateSet::NORTH_WEST, "a challenge pursuit closes gates whatever the stage");
    g.race_done();
    assert!(g.closed().is_empty(), "done restores the stage set");
    let mut high = RegionGates::new();
    high.world_created(14, false);
    high.race_started(GateSet::of(&[10]));
    assert_eq!(high.closed().numbers(), [10, 11, 12, 13, 14, 15], "added on top, nothing cleared");
    high.race_done();
    assert_eq!(high.closed(), GateSet::NORTH_WEST);
}

#[test]
fn the_region_gates_stay_closed_during_a_race() {
    let mut g = RegionGates::new();
    g.world_created(12, false);
    g.race_started(GateSet::EMPTY);
    assert_eq!(g.closed(), GateSet::MIDDLE);
}

#[test]
fn a_status_reset_enables_the_stored_list_again_without_clearing() {
    let mut g = RegionGates::new();
    g.world_created(12, false);
    g.race_started(GateSet::of(&[14]));
    g.race_status_reset(false);
    assert_eq!(g.closed().numbers(), [10, 12, 13, 14], "no clear: the race gate stays");
    g.race_status_reset(true);
    let mut none = RegionGates::new();
    none.race_status_reset(false);
    assert!(none.closed().is_empty(), "no stored record: nothing to enable");
}
