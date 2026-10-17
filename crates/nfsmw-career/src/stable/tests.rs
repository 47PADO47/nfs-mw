use super::*;
use crate::economy::{InfractionKind, InfractionMask};

/// A stable with owned cars `(handle, career index)` and the given career records at those indices.
fn stable_with(cars: &[(u32, u8)], records: &[(usize, CareerRecord)]) -> Stable {
    let mut s = Stable::empty();
    for (slot, &(handle, career)) in cars.iter().enumerate() {
        s.cars[slot] = CarRecord { handle, flags: 0xF_0002, career_index: career, ..CarRecord::free() };
    }
    for &(index, record) in records {
        s.careers[index] = record;
    }
    s
}

fn record(index: u8, bounty: u32, evaded: u16, busted: u16) -> CareerRecord {
    CareerRecord { bounty, evaded, busted, ..CareerRecord::new(index) }
}

#[test]
fn the_block_sizes_add_up_to_the_documented_stable() {
    assert_eq!(CUSTOMIZATION_AT, 0xFA0);
    assert_eq!(CAREER_AT, 0x8728);
    assert_eq!(SOLD_AT, 0x8CA0);
    assert_eq!(SOLD_AT + SOLD_HISTORY_LEN, STABLE_LEN);
    assert_eq!(STABLE_LEN, 36_040);
}

#[test]
fn a_synthetic_stable_round_trips_every_byte() {
    let bytes: Vec<u8> = (0..STABLE_LEN).map(|i| (i * 31 % 253) as u8).collect();
    let stable = Stable::parse(&bytes).unwrap();
    assert_eq!(stable.to_bytes(), bytes);
}

#[test]
fn a_short_stable_is_refused() {
    let err = Stable::parse(&vec![0; STABLE_LEN - 1]).unwrap_err();
    assert_eq!((err.need, err.got), (STABLE_LEN, STABLE_LEN - 1));
}

#[test]
fn an_empty_stable_serializes_to_the_documented_markers() {
    let bytes = Stable::empty().to_bytes();
    assert_eq!(bytes.len(), STABLE_LEN);
    assert_eq!(&bytes[..4], &[0xFF; 4]);
    assert_eq!(bytes[0x11], 0xFF);
    assert_eq!(bytes[0x8728], 0xFF);
    assert_eq!(Stable::parse(&bytes).unwrap(), Stable::empty());
}

#[test]
fn offsets_of_the_tail_follow_the_editor_map() {
    // Editor offsets: bounty 0xE865 = 0x5BC5 + 0x8CA0, infractions 0xE86D = 0x5BC5 + 0x8CA8.
    assert_eq!(0x5BC5 + SOLD_AT, 0xE865);
    assert_eq!(0x5BC5 + SOLD_AT + 8, 0xE86D);
    assert_eq!(0x5BC5 + CAREER_AT, 0xE2ED);
}

#[test]
fn totals_are_the_sold_base_plus_the_live_records() {
    let mut s = stable_with(
        &[(10, 0), (11, 1)],
        &[(0, record(0, 1000, 3, 1)), (1, record(1, 250, 2, 0)), (2, record(2, 99_999, 9, 9))],
    );
    s.sold = SoldHistory { bounty: 43_116_691, evaded: 111, busted: 5, ..SoldHistory::default() };
    assert_eq!(s.total_bounty(), 43_116_691 + 1250, "record 2 belongs to no car and is not live");
    assert_eq!(s.total_evaded(), 111 + 5);
    assert_eq!(s.total_busted(), 5 + 1);
}

#[test]
fn a_car_counts_only_when_it_is_owned_classed_and_points_at_a_used_record() {
    let mut s =
        stable_with(&[(10, 0), (11, 1), (12, 2), (13, 0xFF)], &[(0, record(0, 10, 0, 0)), (1, record(1, 20, 0, 0))]);
    s.cars[1].flags = 0x2; // no class mask
    s.cars[3].flags = 0xF_0002;
    assert_eq!(s.live_career_indices(), vec![0], "car 11 has no class, car 12 points at an unused record");
    s.cars[1].flags = 0xF_0000; // class but not owned
    assert_eq!(s.total_bounty(), 10);
    s.cars[1].flags = 0xF_0002;
    assert_eq!(s.total_bounty(), 30);
}

#[test]
fn two_cars_sharing_a_record_count_it_once() {
    let s = stable_with(&[(10, 0), (11, 0)], &[(0, record(0, 500, 0, 0))]);
    assert_eq!(s.live_career_indices(), vec![0]);
    assert_eq!(s.total_bounty(), 500);
}

#[test]
fn infraction_counts_use_mask_and_set() {
    let mut a = record(0, 0, 0, 0);
    a.unserved = [1, 2, 0, 0, 0, 0, 0, 4];
    a.served = [0, 0, 5, 0, 0, 0, 0, 0];
    let mut b = record(1, 0, 0, 0);
    b.unserved = [3, 0, 0, 0, 0, 0, 0, 0];
    let mut s = stable_with(&[(10, 0), (11, 1)], &[(0, a), (1, b)]);
    s.sold.unserved[1] = 10;
    s.sold.served[2] = 1;
    let speeding_racing = InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::Racing]);
    assert_eq!(s.infraction_count(speeding_racing, InfractionSet::Unserved), (1 + 3) + (2 + 10));
    assert_eq!(s.infraction_count(InfractionMask::of(&[InfractionKind::RecklessDriving]), InfractionSet::Served), 6);
    assert_eq!(s.infraction_set_total(InfractionSet::Unserved), 1 + 2 + 4 + 3 + 10);
    assert_eq!(s.infraction_set_total(InfractionSet::Served), 6);
    assert_eq!(s.total_infractions(), 20 + 6);
    assert_eq!(s.fines_due(), 4 * 150 + 12 * 350 + 4 * 75);
}

#[test]
fn the_tickers_walk_only_live_records_and_count_releases() {
    let mut impounded = CareerRecord::new(0);
    impounded.enter_impound(ImpoundState::LimitReached);
    impounded.countdown = 1;
    let mut stray = impounded;
    stray.index = 1;
    let mut s = stable_with(&[(10, 0)], &[(0, impounded), (1, stray)]);
    assert_eq!(s.win_tick(), 1);
    assert_eq!(s.careers[0].state, 4);
    assert_eq!(s.careers[1].state, 2, "a record with no car is not touched");
    assert_eq!(s.win_tick(), 0);
}

#[test]
fn the_evade_tick_decays_strikes_across_the_stable() {
    let mut free = CareerRecord::new(0);
    free.strikes = 2;
    free.decay = 2;
    let mut s = stable_with(&[(10, 0)], &[(0, free)]);
    assert_eq!(s.evade_tick(), 0);
    assert_eq!((s.careers[0].strikes, s.careers[0].decay), (1, 0));
}

#[test]
fn free_cars_are_counted_for_the_game_over_check() {
    let mut a = CareerRecord::new(0);
    a.enter_impound(ImpoundState::LimitReached);
    let b = CareerRecord::new(1);
    let mut s = stable_with(&[(10, 0), (11, 1)], &[(0, a), (1, b)]);
    assert_eq!(s.free_car_count(), 1);
    s.careers[1].enter_impound(ImpoundState::CashConfiscated);
    assert_eq!(s.free_car_count(), 0);
}

#[test]
fn lookups_by_handle_follow_the_car_to_its_record() {
    let s = stable_with(&[(0x12345678, 3)], &[(3, record(3, 77, 0, 0))]);
    assert_eq!(s.career_index_for_handle(0x12345678), Some(3));
    assert_eq!(s.career_for_handle(0x12345678).map(|r| r.bounty), Some(77));
    assert!(s.career_for_handle(0x1).is_none());
    assert!(s.career_for_handle(FREE_HANDLE).is_none(), "the free handle never matches a free slot");
    assert!(s.car_by_handle(FREE_HANDLE).is_none());
}
