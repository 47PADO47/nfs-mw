use super::*;
use crate::milestones::speedtrap::{KMH_TO_MPS, SpeedTrap, def_for, unlock_bin};

/// A block with a pattern in every byte, so a stray write shows up.
fn patterned() -> Vec<u8> {
    (0..BLOCK_LEN).map(|i| (i * 7 % 251) as u8).collect()
}

/// A block whose header and digest are valid.
fn loadable() -> MilestoneBlock {
    let mut raw = patterned();
    raw[0x10..0x14].copy_from_slice(b"Game");
    raw[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    let mut block = MilestoneBlock::from_bytes(&raw).expect("full block");
    block.refresh_digest();
    block
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn block_round_trip_is_byte_exact() {
    let raw = patterned();
    let block = MilestoneBlock::from_bytes(&raw).expect("full block");
    assert_eq!(block.to_bytes(), raw);
    assert_eq!(block.bytes(), &raw[..]);

    let mut out = vec![0xAAu8; BLOCK_LEN + 8];
    block.write_into(&mut out).expect("room");
    assert_eq!(&out[..BLOCK_LEN], &raw[..]);
    assert!(out[BLOCK_LEN..].iter().all(|&b| b == 0xAA), "bytes past the block are not touched");
}

#[test]
fn short_block_is_refused() {
    let err = MilestoneBlock::from_bytes(&[0u8; 100]).expect_err("too short");
    assert_eq!(err, LayoutError { what: "milestone block", need: BLOCK_LEN, got: 100 });

    let block = loadable();
    let mut small = vec![0u8; 10];
    assert!(block.write_into(&mut small).is_err());
}

#[test]
fn header_words_sit_at_0x10_and_0x14() {
    let block = loadable();
    assert_eq!(block.game_magic(), GAME_MAGIC);
    assert_eq!(&block.bytes()[0x10..0x14], b"Game");
    assert_eq!(block.version(), 8);
    assert!(block.is_loadable());
}

#[test]
fn an_old_version_is_not_loadable() {
    let mut block = loadable();
    let mut raw = block.to_bytes();
    raw[0x14..0x18].copy_from_slice(&7u32.to_le_bytes());
    block = MilestoneBlock::from_bytes(&raw).expect("full block");
    block.refresh_digest();
    assert!(block.digest_ok());
    assert!(!block.is_loadable());
}

#[test]
fn digest_covers_0x10_to_the_end() {
    let mut block = loadable();
    assert!(block.digest_ok());

    let mut raw = block.to_bytes();
    raw[0x20] ^= 1;
    let changed = MilestoneBlock::from_bytes(&raw).expect("full block");
    assert!(!changed.digest_ok());
    assert!(!changed.is_loadable());

    raw[0x20] ^= 1;
    raw[BLOCK_LEN - 1] ^= 1;
    let last = MilestoneBlock::from_bytes(&raw).expect("full block");
    assert!(!last.digest_ok());

    block.refresh_digest();
    assert!(block.digest_ok());
    let stored = &block.bytes()[..DIGEST_LEN];
    assert_eq!(stored, md5(&block.bytes()[DIGEST_FROM..]));
}

#[test]
fn not_set_is_none() {
    assert_eq!(opt_from_save(-1.0), None);
    assert_eq!(opt_to_save(None).to_bits(), (-1.0f32).to_bits());
    assert_eq!(opt_from_save(2.5), Some(2.5));
    assert_eq!(opt_to_save(Some(2.5)), 2.5);
    assert_eq!(opt_from_save(0.0), Some(0.0));
}

#[test]
fn stat_flags_follow_the_statistic() {
    assert_eq!(stat_flags(StatType::CopsDestroyed), 1);
    assert_eq!(stat_flags(StatType::PursuitEvasionTime), 2);
    assert_eq!(stat_flags(StatType::PursuitLength), 3);
}

#[test]
fn stat_node_offsets_and_round_trip() {
    let mut raw = [0u8; STAT_NODE_LEN];
    raw[0x00..0x04].copy_from_slice(&0x1122_3344u32.to_le_bytes());
    raw[0x04..0x08].copy_from_slice(&(-1.0f32).to_le_bytes());
    raw[0x08..0x0C].copy_from_slice(&250.0f32.to_le_bytes());
    raw[0x0C..0x10].copy_from_slice(&3u32.to_le_bytes());

    let node = StatNode::from_bytes(&raw).expect("16 bytes");
    assert_eq!(node.key, 0x1122_3344);
    assert_eq!(node.current, None);
    assert_eq!(node.best, Some(250.0));
    assert_eq!(node.flags, 3);
    assert_eq!(node.value(), StatValue { current: None, best: Some(250.0) });

    let mut out = [0u8; STAT_NODE_LEN];
    node.write_into(&mut out).expect("16 bytes");
    assert_eq!(out, raw);
}

#[test]
fn stat_node_from_value_sets_the_flags_and_keeps_none() {
    let value = StatValue { current: None, best: Some(30.0) };
    let node = StatNode::from_value(5, StatType::PursuitEvasionTime, value);
    assert_eq!(node.flags, 2);
    assert_eq!(node.current, None);

    let mut out = [0u8; STAT_NODE_LEN];
    node.write_into(&mut out).expect("16 bytes");
    assert_eq!(&out[0x04..0x08], &(-1.0f32).to_le_bytes());
    assert_eq!(&out[0x08..0x0C], &30.0f32.to_le_bytes());
}

#[test]
fn milestone_entry_offsets_and_round_trip() {
    let mut raw = [0u8; MILESTONE_LEN];
    raw[0x00..0x04].copy_from_slice(&0xAABB_CCDDu32.to_le_bytes());
    raw[0x04..0x08].copy_from_slice(&0x0102_0304u32.to_le_bytes());
    raw[0x08] = 2;
    raw[0x09] = 1;
    raw[0x0A..0x0C].copy_from_slice(&15u16.to_le_bytes());
    raw[0x0C..0x10].copy_from_slice(&2.5f32.to_le_bytes());
    raw[0x10..0x14].copy_from_slice(&0.0f32.to_le_bytes());

    let entry = MilestoneEntry::from_bytes(&raw).expect("20 bytes");
    assert_eq!(entry.type_key, 0xAABB_CCDD);
    assert_eq!(entry.key, 0x0102_0304);
    assert_eq!(entry.state(), Some(MilestoneState::Active));
    assert_eq!(entry.flags, 1);
    assert_eq!(entry.bin, 15);
    assert_eq!(entry.threshold, 2.5);
    assert_eq!(entry.banked_value(), None);

    let mut out = [0u8; MILESTONE_LEN];
    entry.write_into(&mut out).expect("20 bytes");
    assert_eq!(out, raw);
}

#[test]
fn milestone_entry_keeps_an_unknown_state_byte() {
    let mut raw = [0u8; MILESTONE_LEN];
    raw[0x08] = 9;
    let entry = MilestoneEntry::from_bytes(&raw).expect("20 bytes");
    assert_eq!(entry.state(), None);

    let def = MilestoneDef { key: 0, bin: 0, stat: StatType::CopsDestroyed, threshold: 1.0, bounty: 1 };
    assert_eq!(entry.to_milestone(def), None);

    let mut out = [0u8; MILESTONE_LEN];
    entry.write_into(&mut out).expect("20 bytes");
    assert_eq!(out[0x08], 9);
}

#[test]
fn milestone_entry_round_trips_with_the_in_memory_milestone() {
    let def = MilestoneDef { key: 0x0102, bin: 15, stat: StatType::CopsDestroyed, threshold: 3.0, bounty: 500 };
    let mut entry = MilestoneEntry::from_bytes(&[0u8; MILESTONE_LEN]).expect("20 bytes");
    entry.flags = 2;

    let at_stake = Milestone { def, state: MilestoneState::AtStake, value: None };
    entry.update_from_milestone(&at_stake, 0x77);
    assert_eq!(entry.type_key, 0x77);
    assert_eq!(entry.key, 0x0102);
    assert_eq!(entry.state_byte, 3);
    assert_eq!(entry.bin, 15);
    assert_eq!(entry.threshold, 3.0);
    assert_eq!(entry.value, 0.0);
    assert_eq!(entry.flags, 3, "bit 1 (forced) is kept, bit 0 follows the statistic");
    assert_eq!(entry.to_milestone(def), Some(at_stake));

    let done = Milestone { def, state: MilestoneState::Complete, value: Some(1234.0) };
    entry.update_from_milestone(&done, 0x77);
    assert_eq!(entry.to_milestone(def), Some(done));
}

#[test]
fn milestone_entry_refuses_a_definition_with_another_key_or_bin() {
    let def = MilestoneDef { key: 9, bin: 2, stat: StatType::CopsDestroyed, threshold: 3.0, bounty: 500 };
    let mut entry = MilestoneEntry::from_bytes(&[0u8; MILESTONE_LEN]).expect("20 bytes");
    entry.state_byte = 2;
    entry.key = 9;
    entry.bin = 2;
    assert!(entry.to_milestone(def).is_some());

    let other_key = MilestoneDef { key: 8, ..def };
    assert_eq!(entry.to_milestone(other_key), None);
    let other_bin = MilestoneDef { bin: 3, ..def };
    assert_eq!(entry.to_milestone(other_bin), None);
}

#[test]
fn speed_trap_entry_offsets_and_round_trip() {
    let mut raw = [0u8; SPEED_TRAP_LEN];
    raw[0x00..0x02].copy_from_slice(&5u16.to_le_bytes());
    raw[0x02..0x04].copy_from_slice(&1u16.to_le_bytes());
    raw[0x04..0x08].copy_from_slice(&11u32.to_le_bytes());
    raw[0x08..0x0C].copy_from_slice(&0xDEADu32.to_le_bytes());
    raw[0x0C..0x10].copy_from_slice(&88.9f32.to_le_bytes());
    raw[0x10..0x14].copy_from_slice(&91.7f32.to_le_bytes());

    let entry = SpeedTrapEntry::from_bytes(&raw).expect("20 bytes");
    assert_eq!(entry.flags, 5);
    assert_eq!(entry.bin, 1);
    assert_eq!(entry.key, 11);
    assert_eq!(entry.unknown_0x08, 0xDEAD);
    assert_eq!(entry.threshold_mps, 88.9);
    assert_eq!(entry.recorded_mps, 91.7);

    let mut out = [0u8; SPEED_TRAP_LEN];
    entry.write_into(&mut out).expect("20 bytes");
    assert_eq!(out, raw);
}

#[test]
fn speed_trap_entry_follows_a_completed_trap() {
    let mut trap = SpeedTrap::new(def_for(1, 1, 11).expect("bin 1 trap 1"));
    unlock_bin(std::slice::from_mut(&mut trap), 1);
    let _ = trap.crossing(330.0);

    let mut entry = SpeedTrapEntry::from_bytes(&[0u8; SPEED_TRAP_LEN]).expect("20 bytes");
    entry.unknown_0x08 = 7;
    entry.update_from_trap(&trap);
    assert_eq!(entry.flags, 5, "unlocked and done");
    assert_eq!(entry.bin, 1);
    assert_eq!(entry.key, 11);
    assert_eq!(entry.unknown_0x08, 7, "the unknown number is kept");
    assert!(near(entry.threshold_mps, 320.0 * KMH_TO_MPS));
    assert!(near(entry.recorded_mps, 330.0 * KMH_TO_MPS));
}

#[test]
fn speed_trap_entry_finds_its_definition() {
    let mut entry = SpeedTrapEntry::from_bytes(&[0u8; SPEED_TRAP_LEN]).expect("20 bytes");
    entry.bin = 1;
    entry.key = 11;

    let def = entry.def(1).expect("bin 1 trap 1");
    assert_eq!(def.key, 11);
    assert_eq!(def.threshold_kmh, 320);
    assert_eq!(def.bounty, 136_000);
    assert_eq!(entry.def(4), None);

    entry.bin = 16;
    assert_eq!(entry.def(1), None);
}

#[test]
fn bin_counter_offsets_and_round_trip() {
    let mut raw = [0u8; BIN_COUNTER_LEN];
    raw[0x00..0x02].copy_from_slice(&3u16.to_le_bytes());
    raw[0x02..0x04].copy_from_slice(&4u16.to_le_bytes());
    raw[0x04..0x06].copy_from_slice(&9u16.to_le_bytes());
    raw[0x06..0x08].copy_from_slice(&2u16.to_le_bytes());

    let counter = BinCounter::from_bytes(&raw).expect("8 bytes");
    assert_eq!(counter, BinCounter { bin: 3, length: 4, completed: 9, events: 2 });

    let mut out = [0u8; BIN_COUNTER_LEN];
    counter.write_into(&mut out).expect("8 bytes");
    assert_eq!(out, raw);
}

#[test]
fn bin_counter_counts_done_traps_and_keeps_events() {
    let mut traps = vec![SpeedTrap::new(def_for(1, 1, 11).expect("bin 1 trap 1"))];
    unlock_bin(&mut traps, 1);
    let _ = traps[0].crossing(330.0);
    let set = MilestoneSet::new(&[]);

    let mut counter = BinCounter { bin: 1, length: 4, completed: 0, events: 7 };
    counter.set_progress(&set, &traps);
    assert_eq!(counter.completed, 1);
    assert_eq!(counter.events, 7);

    let mut other = BinCounter { bin: 2, length: 4, completed: 5, events: 0 };
    other.set_progress(&set, &traps);
    assert_eq!(other.completed, 0);
}

/// The object sizes of the `mwo` save: 9 empty records (stride 16) and 8 records of 9 bytes (stride 32).
fn mwo_sizes() -> Vec<u32> {
    let mut sizes = vec![0; 9];
    sizes.extend([9; 8]);
    sizes
}

/// The blob with the header counts of the three saves (N, S = 8, E = 30, M = 55, T = 39, B = 512, C = 164, K = 0),
/// object records of the given sizes and the patterned filler elsewhere.
fn raw_blob(object_sizes: &[u32]) -> Vec<u8> {
    let mut raw = patterned();
    let header: [(usize, u32); 9] = [
        (0x18, u32::try_from(object_sizes.len()).expect("few objects")),
        (0x1C, 8),
        (0x20, 30),
        (0x24, 55),
        (0x28, 39),
        (0x2C, 0),
        (0x30, 512),
        (0x34, 164),
        (0x38, 0),
    ];
    for (at, value) in header {
        raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut at = RECORDS_AT;
    for (i, &size) in object_sizes.iter().enumerate() {
        raw[at..at + 4].copy_from_slice(&(0x1000 + i as u32).to_le_bytes());
        raw[at + 4..at + 8].copy_from_slice(&size.to_le_bytes());
        at += (size as usize + 0x17) & !0xF;
    }
    raw
}

/// [`raw_blob`] with the bin counter count (20) written where the layout puts it.
fn synthetic(object_sizes: &[u32]) -> MilestoneBlock {
    let mut raw = raw_blob(object_sizes);
    let bins = MilestoneBlock::from_bytes(&raw).expect("full block").layout().expect("fits").bin_counters;
    raw[bins..bins + 4].copy_from_slice(&20u32.to_le_bytes());
    MilestoneBlock::from_bytes(&raw).expect("full block")
}

/// The block with the u32 at `at` replaced.
fn with_word(block: &MilestoneBlock, at: usize, value: u32) -> MilestoneBlock {
    let mut raw = block.to_bytes();
    raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
    MilestoneBlock::from_bytes(&raw).expect("full block")
}

#[test]
fn layout_matches_the_name_and_stock_offsets() {
    let layout = synthetic(&[129, 20]).layout().expect("fits");
    assert_eq!(layout.slots, 0x130);
    assert_eq!(layout.stats, 0x230);
    assert_eq!(layout.milestones, 0x410);
    assert_eq!(layout.speed_traps, 0x860);
    assert_eq!(layout.flag_bits, 0xB70);
    assert_eq!(layout.bin_counters, 0xBB0);
    assert_eq!(layout.bounty_keys, 0xC60);
    assert_eq!(layout.references, 0xC60);
    assert_eq!((layout.stat_count, layout.milestone_count, layout.speed_trap_count), (30, 55, 39));
    assert_eq!((layout.flag_bits_len, layout.bin_counters_len), (64, 164));
}

#[test]
fn layout_matches_the_mwo_offsets() {
    let layout = synthetic(&mwo_sizes()).layout().expect("fits");
    assert_eq!(layout.slots, 0x210);
    assert_eq!(layout.stats, 0x310);
    assert_eq!(layout.milestones, 0x4F0);
    assert_eq!(layout.speed_traps, 0x940);
    assert_eq!(layout.flag_bits, 0xC50);
    assert_eq!(layout.bin_counters, 0xC90);
    assert_eq!(layout.bounty_keys, 0xD40);
    assert_eq!(layout.references, 0xD40);
}

#[test]
fn layout_refuses_parts_that_run_past_the_block() {
    let block = synthetic(&[129, 20]);
    let err = with_word(&block, 0x24, 100_000).layout().expect_err("55 becomes 100,000");
    assert_eq!(err.what, "milestone block layout");
    assert!(err.need > BLOCK_LEN);

    let huge_object = MilestoneBlock::from_bytes(&raw_blob(&[0x4000])).expect("full block");
    assert!(huge_object.layout().is_err());
}

#[test]
fn arrays_write_back_byte_exact() {
    for sizes in [vec![129, 20], mwo_sizes()] {
        let original = synthetic(&sizes);
        let mut block = original.clone();
        block.set_stats(&original.stats().expect("stats")).expect("same count");
        block.set_milestones(&original.milestones().expect("milestones")).expect("same count");
        block.set_speed_traps(&original.speed_traps().expect("traps")).expect("same count");
        block.set_bin_counters(&original.bin_counters().expect("bins")).expect("same count");
        assert_eq!(block.bytes(), original.bytes());
    }
}

#[test]
fn arrays_read_back_what_was_written() {
    let mut block = synthetic(&[129, 20]);

    let nodes: Vec<StatNode> = (0..30u32)
        .map(|i| StatNode { key: 100 + i, current: (i % 2 == 1).then_some(i as f32), best: Some(250.0), flags: 3 })
        .collect();
    block.set_stats(&nodes).expect("30 nodes");
    assert_eq!(block.stats().expect("stats"), nodes);

    let entries: Vec<MilestoneEntry> = (0..55u32)
        .map(|i| MilestoneEntry {
            type_key: 0x1000 + i,
            key: 0x2000 + i,
            state_byte: 1,
            flags: 0,
            bin: u16::try_from(i % 15 + 1).expect("bin"),
            threshold: 2.5,
            value: 0.0,
        })
        .collect();
    block.set_milestones(&entries).expect("55 entries");
    assert_eq!(block.milestones().expect("milestones"), entries);

    let traps: Vec<SpeedTrapEntry> = (0..39u32)
        .map(|i| SpeedTrapEntry {
            flags: 5,
            bin: u16::try_from(i % 15 + 1).expect("bin"),
            key: 0x3000 + i,
            unknown_0x08: 0x1234,
            threshold_mps: 88.9,
            recorded_mps: 91.7,
        })
        .collect();
    block.set_speed_traps(&traps).expect("39 traps");
    assert_eq!(block.speed_traps().expect("traps"), traps);

    let counters: Vec<BinCounter> =
        (0..20u16).map(|i| BinCounter { bin: i + 1, length: 4, completed: i, events: 0 }).collect();
    block.set_bin_counters(&counters).expect("20 bins");
    assert_eq!(block.bin_counters().expect("bins"), counters);
}

#[test]
fn a_setter_refuses_another_count_and_writes_nothing() {
    let mut block = synthetic(&[129, 20]);
    let before = block.to_bytes();
    let stats = block.stats().expect("stats");
    let err = block.set_stats(&stats[..29]).expect_err("29 for 30");
    assert_eq!(err, ArrayError::Count { what: "statistic nodes", have: 30, given: 29 });
    assert_eq!(block.to_bytes(), before);
}
