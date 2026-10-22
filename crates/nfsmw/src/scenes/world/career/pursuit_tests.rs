use blackbox_pursuit::Fines;
use nfsmw_career::stable::{CarRecord, CareerRecord};

use super::*;

fn log_of(infractions: &[Infraction]) -> InfractionLog {
    let fines = Fines::new([100; 8]);
    let mut log = InfractionLog::default();
    for &infraction in infractions {
        log.record(infraction, &fines);
    }
    log
}

#[test]
fn a_busted_chase_is_a_bust_and_an_evaded_one_is_not() {
    let mask = InfractionMask::of(&[InfractionKind::Speeding]);
    let busted = pursuit_result(EndReason::Busted, mask, 0, 0);
    let evaded = pursuit_result(EndReason::Evaded, mask, 0, 0);
    assert!(busted.busted);
    assert!(!evaded.busted);
    assert_eq!(evaded.infractions, mask);
    assert!(!evaded.challenge_series);
}

#[test]
fn the_bounty_is_the_normal_and_the_destruction_points() {
    let mask = InfractionMask::NONE;
    let result = pursuit_result(EndReason::Evaded, mask, 1200, 300);
    assert_eq!(result.rep_points_normal, 1200);
    assert_eq!(result.rep_points_destroyed, 300);
    assert_eq!(result.bounty(), 1500);
}

#[test]
fn each_world_infraction_maps_to_its_own_rule_bit() {
    for (i, infraction) in Infraction::ALL.into_iter().enumerate() {
        let mask = infraction_mask(&log_of(&[infraction]));
        assert_eq!(mask.0, 1 << i, "{infraction:?}");
        assert_eq!(mask.count(), 1);
    }
}

#[test]
fn a_repeated_infraction_counts_once() {
    let log = log_of(&[Infraction::Speeding, Infraction::Speeding, Infraction::HitAndRun]);
    let mask = infraction_mask(&log);
    assert_eq!(mask.0, 0x11);
    assert_eq!(mask.count(), 2);
}

#[test]
fn an_empty_log_is_no_infraction() {
    assert_eq!(infraction_mask(&InfractionLog::default()), InfractionMask::NONE);
}

#[test]
fn ticks_are_quarter_milliseconds() {
    assert_eq!(ticks_of(1.0), 4000);
    assert_eq!(ticks_of(1.5), 6000);
    assert_eq!(ticks_of(0.00025), 1);
    assert_eq!(ticks_of(0.0), 0);
}

#[test]
fn a_negative_or_nan_duration_is_no_ticks() {
    assert_eq!(ticks_of(-3.0), 0);
    assert_eq!(ticks_of(f32::NAN), 0);
}

#[test]
fn the_pursuit_stats_carry_what_the_world_knows() {
    let stats = pursuit_stats(0xBEEF, 2.0, 3, 2, 9000, 15_000);
    assert_eq!(stats.car_key, 0xBEEF);
    assert_eq!(stats.bounty, 9000);
    assert_eq!(stats.ticks, 8000);
    assert_eq!(stats.cops_destroyed, 3);
    assert_eq!(stats.infraction_kinds, 2);
    assert_eq!(stats.cost_to_state, 15_000);
    assert_eq!((stats.cops_involved, stats.cops_damaged), (0, 0));
    assert_eq!(stats.extras, [0; 8]);
}

#[test]
fn a_case_id_is_mw_then_eight_characters_and_a_terminator() {
    let id = case_id(12345);
    assert_eq!(&id[..3], b"MW-");
    assert_eq!(id[11], 0);
    for (i, &c) in id[3..11].iter().enumerate() {
        match i % 2 {
            0 => assert!(c.is_ascii_digit(), "position {i} is {c}"),
            _ => assert!(c.is_ascii_uppercase(), "position {i} is {c}"),
        }
    }
}

#[test]
fn a_case_id_is_the_same_for_the_same_seed_and_differs_across_seeds() {
    assert_eq!(case_id(7), case_id(7));
    assert_ne!(case_id(7), case_id(8));
}

#[test]
fn the_restored_heat_only_goes_back_when_one_was_kept() {
    let mut stable = Stable::empty();
    stable.cars[0] =
        CarRecord { handle: 5, flags: 0xF_0020 | CarRecord::FLAG_OWNED, career_index: 0, ..CarRecord::free() };
    stable.careers[0] = CareerRecord { heat: 4.0, ..CareerRecord::new(0) };
    restore_heat(&mut stable, 5, None);
    assert_eq!(stable.careers[0].heat, 4.0, "nothing kept, nothing written");
    restore_heat(&mut stable, 5, Some(1.5));
    assert_eq!(stable.careers[0].heat, 1.5);
    restore_heat(&mut stable, 99, Some(3.0));
    assert_eq!(stable.careers[0].heat, 1.5, "an unknown car keeps its record");
}

#[test]
fn a_banked_milestone_is_a_challenge_passed_in_its_bin() {
    use nfsmw_career::milestones::{Milestone, MilestoneDef, MilestoneSet, MilestoneState, STAT_COUNT, StatValue};
    let def = |key, bin| MilestoneDef { key, bin, stat: StatType::PursuitLength, threshold: 60.0, bounty: 500 };
    let milestone = |key, bin| Milestone { def: def(key, bin), state: MilestoneState::Complete, value: None };
    let set = MilestoneSet::from_parts(
        vec![milestone(7, 3), milestone(9, 12)],
        [StatValue::default(); STAT_COUNT],
        Vec::new(),
    );
    let banked = [Banked { key: 9, bounty: 500 }, Banked { key: 7, bounty: 500 }, Banked { key: 99, bounty: 1 }];
    assert_eq!(passed_challenges(&set, &banked), vec![(12, "9".to_owned()), (3, "7".to_owned())]);
}

const LEAD_CAR: u32 = 0x10;
const GUEST_CAR: u32 = 0x11;

/// Both players chased in one chase.
fn both_chased() -> Targets {
    let mut targets = Targets::new();
    targets.add(TargetId(LEAD.0));
    targets.add(TargetId(GUEST.0));
    targets
}

#[test]
fn single_player_banks_the_current_car() {
    assert_eq!(chase_banking(None, LEAD_CAR), (Some(LEAD_CAR), Vec::new()));
    let (bank, others) = bank_handles(LEAD_CAR, None, &Targets::new(), LEAD);
    assert_eq!((bank, others), (Some(LEAD_CAR), Vec::new()));
}

#[test]
fn guest_primary_banks_the_guest_car() {
    let (bank, others) = bank_handles(LEAD_CAR, Some(GUEST_CAR), &both_chased(), GUEST);
    assert_eq!((bank, others), (Some(GUEST_CAR), vec![LEAD_CAR]));
    let (bank, _) = bank_handles(LEAD_CAR, None, &both_chased(), GUEST);
    assert_eq!(bank, None, "a guest without a career car banks nothing");
}

#[test]
fn chased_partner_is_an_other() {
    let (bank, others) = bank_handles(LEAD_CAR, Some(GUEST_CAR), &both_chased(), LEAD);
    assert_eq!((bank, others), (Some(LEAD_CAR), vec![GUEST_CAR]));
    let mut busted = both_chased();
    busted.bust(TargetId(GUEST.0));
    let (_, others) = bank_handles(LEAD_CAR, Some(GUEST_CAR), &busted, LEAD);
    assert!(others.is_empty(), "a car busted earlier in the chase is not an other");
}

#[test]
fn coop_carries_the_higher_heat() {
    assert_eq!(carried(2.0, Some(3.5)), 3.5);
    assert_eq!(carried(4.0, Some(1.5)), 4.0);
    assert_eq!(carried(2.0, None), 2.0);
}
