//! Tests of the new career block (`new.rs`): it lays out, holds its records, and starts locked and not set.

use super::super::*;
use crate::milestones::speedtrap::SpeedTrapDef;
use crate::milestones::{MilestoneDef, MilestoneState, StatType};

/// The records of a new career with 55 milestones, 39 traps and 15 bin counters, as the inputs given.
fn new_career_inputs() -> (Vec<MilestoneEntry>, Vec<SpeedTrapEntry>, Vec<BinCounter>) {
    let milestones = (0..55u32)
        .map(|i| {
            let def = MilestoneDef {
                key: 0x1000 + i,
                bin: (i % 15 + 1) as u8,
                stat: StatType::PursuitLength,
                threshold: 10.0 + i as f32,
                bounty: 0,
            };
            MilestoneEntry::new_locked(def, 0)
        })
        .collect();
    let traps = (0..39u32)
        .map(|i| {
            let def = SpeedTrapDef {
                key: 0x2000 + i,
                bin: (i % 15 + 1) as u8,
                index: (i % 3 + 1) as u8,
                threshold_kmh: 150,
                bounty: 0,
            };
            SpeedTrapEntry::new_locked(def)
        })
        .collect();
    let bins = (1..=15u16).map(BinCounter::new_career).collect();
    (milestones, traps, bins)
}

#[test]
fn a_new_career_block_lays_out_and_is_loadable() {
    let (milestones, traps, bins) = new_career_inputs();
    let block = MilestoneBlock::new_career(&milestones, &traps, &bins).expect("a new career block");
    let layout = block.layout().expect("lays out");
    assert_eq!(layout.milestone_count, 55);
    assert_eq!(layout.speed_trap_count, 39);
    assert_eq!(layout.stat_count, 30);
    assert!(block.is_loadable());
}

#[test]
fn a_new_career_block_round_trips_its_records() {
    let (milestones, traps, bins) = new_career_inputs();
    let block = MilestoneBlock::new_career(&milestones, &traps, &bins).expect("a new career block");
    assert_eq!(block.milestones().expect("milestones"), milestones);
    assert_eq!(block.speed_traps().expect("traps"), traps);
    assert_eq!(block.bin_counters().expect("bins"), bins);
    let reread = MilestoneBlock::from_bytes(&block.to_bytes()).expect("full block");
    assert_eq!(reread, block);
}

#[test]
fn a_new_career_block_starts_locked_and_not_set() {
    let (milestones, traps, bins) = new_career_inputs();
    let block = MilestoneBlock::new_career(&milestones, &traps, &bins).expect("a new career block");
    assert!(block.milestones().expect("milestones").iter().all(|e| e.state() == Some(MilestoneState::Locked)));
    assert!(block.speed_traps().expect("traps").iter().all(|t| t.flags == 0 && t.recorded_mps == 0.0));
    assert!(block.stats().expect("stats").iter().all(|n| n.current.is_none() && n.best.is_none()));
}

#[test]
fn a_new_career_block_refuses_more_bins_than_it_holds() {
    let (milestones, traps, _) = new_career_inputs();
    let bins: Vec<BinCounter> = (1..=21u16).map(BinCounter::new_career).collect();
    assert!(MilestoneBlock::new_career(&milestones, &traps, &bins).is_err());
}
