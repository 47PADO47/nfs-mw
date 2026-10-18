//! Layout snapshot of the `nfsmw.*` ABI (ADR 0005 section 9). The numbers are for 64-bit targets. Changing a field means editing
//! these numbers on purpose, in the same commit, and that is an ABI change under the rules in `mod.rs`.

use core::mem::{align_of, offset_of, size_of};

use super::*;

#[test]
fn ids_versions_and_limits_are_locked() {
    assert_eq!(IFACE_PURSUIT, "nfsmw.pursuit");
    assert_eq!(IFACE_CAREER, "nfsmw.career");
    assert_eq!(IFACE_RACES, "nfsmw.races");
    assert_eq!([IFACE_PURSUIT_VERSION, IFACE_CAREER_VERSION, IFACE_RACES_VERSION], [(0, 1), (0, 2), (0, 1)]);
    assert_eq!([MW_HEAT_LEVELS, MW_MAX_COPS_PER_ROW, MW_MAX_CHECKPOINTS], [10, 64, 256]);
}

#[test]
fn event_kinds_are_locked() {
    assert_eq!(
        [
            MW_EVENT_PURSUIT_STARTED,
            MW_EVENT_PURSUIT_ENDED,
            MW_EVENT_HEAT_CHANGED,
            MW_EVENT_BUSTED,
            MW_EVENT_ESCAPED,
            MW_EVENT_INFRACTION_RECORDED,
        ],
        [0x1000, 0x1001, 0x1002, 0x1003, 0x1004, 0x1005]
    );
}

#[test]
fn closed_sets_are_locked() {
    assert_eq!([CopTable::Roam.raw(), CopTable::Race.raw()], [0, 1]);
    assert_eq!([EndMode::Evade.raw(), EndMode::Bust.raw(), EndMode::Abort.raw()], [0, 1, 2]);
    assert_eq!([EndReason::Evaded.raw(), EndReason::Busted.raw(), EndReason::Aborted.raw()], [0, 1, 2]);
    assert_eq!(Infraction::OffRoad.raw(), 7);
    assert_eq!(ShopKind::Paint.raw(), 3);
    assert_eq!(CopTable::from_raw(2), None);
    assert_eq!(Infraction::from_raw(8), None);
}

#[test]
fn name_and_vector_records() {
    assert_eq!(size_of::<BbName>(), 132);
    assert_eq!(align_of::<BbName>(), 4);
    assert_eq!(offset_of!(BbName, len), 0);
    assert_eq!(offset_of!(BbName, bytes), 4);
    assert_eq!(size_of::<BbVec3>(), 12);
    assert_eq!(align_of::<BbVec3>(), 4);
}

#[test]
fn pursuit_table_and_records() {
    assert_eq!(size_of::<BbNfsmwPursuit>(), 88);
    assert_eq!(align_of::<BbNfsmwPursuit>(), 8);
    assert_eq!(offset_of!(BbNfsmwPursuit, head), 0);
    assert_eq!(offset_of!(BbNfsmwPursuit, ctx), 8);
    assert_eq!(offset_of!(BbNfsmwPursuit, heat_get), 16);
    assert_eq!(offset_of!(BbNfsmwPursuit, heat_set), 24);
    assert_eq!(offset_of!(BbNfsmwPursuit, cop_table_get), 32);
    assert_eq!(offset_of!(BbNfsmwPursuit, cop_table_set), 40);
    assert_eq!(offset_of!(BbNfsmwPursuit, start), 48);
    assert_eq!(offset_of!(BbNfsmwPursuit, end), 56);
    assert_eq!(offset_of!(BbNfsmwPursuit, is_pursuing), 64);
    assert_eq!(offset_of!(BbNfsmwPursuit, busted_get), 72);
    assert_eq!(offset_of!(BbNfsmwPursuit, cops_in_pursuit), 80);

    assert_eq!(size_of::<BbHeatRow>(), 24);
    assert_eq!(offset_of!(BbHeatRow, size), 0);
    assert_eq!(offset_of!(BbHeatRow, cop_count), 4);
    assert_eq!(offset_of!(BbHeatRow, patrol_cars), 8);
    assert_eq!(offset_of!(BbHeatRow, full_engagement_cops), 12);
    assert_eq!(offset_of!(BbHeatRow, first_four_interval), 16);
    assert_eq!(offset_of!(BbHeatRow, spawn_interval), 20);

    assert_eq!(size_of::<BbCopRecord>(), 140);
    assert_eq!(offset_of!(BbCopRecord, count), 0);
    assert_eq!(offset_of!(BbCopRecord, chance), 4);
    assert_eq!(offset_of!(BbCopRecord, name), 8);
}

#[test]
fn pursuit_event_payloads() {
    assert_eq!(size_of::<BbMwPursuitStarted>(), 16);
    assert_eq!(offset_of!(BbMwPursuitStarted, level), 4);
    assert_eq!(offset_of!(BbMwPursuitStarted, cops), 8);
    assert_eq!(offset_of!(BbMwPursuitStarted, heat), 12);

    assert_eq!(size_of::<BbMwPursuitEnded>(), 24);
    assert_eq!(align_of::<BbMwPursuitEnded>(), 8);
    assert_eq!(offset_of!(BbMwPursuitEnded, reason), 4);
    assert_eq!(offset_of!(BbMwPursuitEnded, bounty), 8);
    assert_eq!(offset_of!(BbMwPursuitEnded, duration), 16);

    assert_eq!(size_of::<BbMwHeatChanged>(), 16);
    assert_eq!(offset_of!(BbMwHeatChanged, level), 4);
    assert_eq!(offset_of!(BbMwHeatChanged, old_heat), 8);
    assert_eq!(offset_of!(BbMwHeatChanged, new_heat), 12);

    assert_eq!(size_of::<BbMwBusted>(), 16);
    assert_eq!(offset_of!(BbMwBusted, level), 4);
    assert_eq!(offset_of!(BbMwBusted, bounty), 8);

    assert_eq!(size_of::<BbMwEscaped>(), 12);
    assert_eq!(offset_of!(BbMwEscaped, duration), 8);

    assert_eq!(size_of::<BbMwInfraction>(), 12);
    assert_eq!(offset_of!(BbMwInfraction, infraction), 4);
    assert_eq!(offset_of!(BbMwInfraction, fine), 8);
}

#[test]
fn career_table() {
    assert_eq!(size_of::<BbNfsmwCareer>(), 328);
    assert_eq!(align_of::<BbNfsmwCareer>(), 8);
    assert_eq!(offset_of!(BbNfsmwCareer, ctx), 8);
    let fields = [
        offset_of!(BbNfsmwCareer, cash_get),
        offset_of!(BbNfsmwCareer, cash_add),
        offset_of!(BbNfsmwCareer, cash_set),
        offset_of!(BbNfsmwCareer, bounty_get),
        offset_of!(BbNfsmwCareer, bounty_add),
        offset_of!(BbNfsmwCareer, bounty_set),
        offset_of!(BbNfsmwCareer, rap_sheet_get),
        offset_of!(BbNfsmwCareer, rap_sheet_set_stat),
        offset_of!(BbNfsmwCareer, rap_sheet_add_stat),
        offset_of!(BbNfsmwCareer, rival_count),
        offset_of!(BbNfsmwCareer, rival_at),
        offset_of!(BbNfsmwCareer, rival_defeat),
        offset_of!(BbNfsmwCareer, rival_set_locked),
        offset_of!(BbNfsmwCareer, milestone_count),
        offset_of!(BbNfsmwCareer, milestone_at),
        offset_of!(BbNfsmwCareer, milestone_complete),
        offset_of!(BbNfsmwCareer, milestone_register),
        offset_of!(BbNfsmwCareer, milestone_progress),
        offset_of!(BbNfsmwCareer, stage_get),
        offset_of!(BbNfsmwCareer, garage_count),
        offset_of!(BbNfsmwCareer, garage_at),
        offset_of!(BbNfsmwCareer, give_car),
        offset_of!(BbNfsmwCareer, remove_car),
        offset_of!(BbNfsmwCareer, current_car_get),
        offset_of!(BbNfsmwCareer, current_car_set),
        offset_of!(BbNfsmwCareer, save),
        offset_of!(BbNfsmwCareer, slot_get),
        offset_of!(BbNfsmwCareer, safehouse_count),
        offset_of!(BbNfsmwCareer, safehouse_at),
        offset_of!(BbNfsmwCareer, safehouse_set_unlocked),
        offset_of!(BbNfsmwCareer, safehouse_register),
        offset_of!(BbNfsmwCareer, safehouse_set_cools_heat),
        offset_of!(BbNfsmwCareer, shop_count),
        offset_of!(BbNfsmwCareer, shop_at),
        offset_of!(BbNfsmwCareer, shop_stock_count),
        offset_of!(BbNfsmwCareer, shop_stock_at),
        offset_of!(BbNfsmwCareer, shop_add_item),
        offset_of!(BbNfsmwCareer, shop_remove_item),
        offset_of!(BbNfsmwCareer, shop_set_price),
    ];
    // Thirty-nine functions, one pointer each, from byte 16 on.
    for (index, offset) in fields.into_iter().enumerate() {
        assert_eq!(offset, 16 + 8 * index, "career field {index}");
    }
}

#[test]
fn career_records() {
    assert_eq!(size_of::<BbRapSheet>(), 52);
    assert_eq!(offset_of!(BbRapSheet, infractions), 20);
    assert_eq!(size_of::<BbRival>(), 152);
    assert_eq!(align_of::<BbRival>(), 8);
    assert_eq!(offset_of!(BbRival, car), 8);
    assert_eq!(offset_of!(BbRival, locked), 17);
    assert_eq!(offset_of!(BbRival, name), 20);
    assert_eq!(size_of::<BbMilestone>(), 152);
    assert_eq!(offset_of!(BbMilestone, progress), 8);
    assert_eq!(offset_of!(BbMilestone, name), 20);
    assert_eq!(size_of::<BbMilestoneDef>(), 276);
    assert_eq!(offset_of!(BbMilestoneDef, key), 12);
    assert_eq!(offset_of!(BbMilestoneDef, name), 144);
    assert_eq!(size_of::<BbOwnedCar>(), 152);
    assert_eq!(offset_of!(BbOwnedCar, car), 8);
    assert_eq!(offset_of!(BbOwnedCar, name), 20);
    assert_eq!(size_of::<BbSafehouse>(), 156);
    assert_eq!(offset_of!(BbSafehouse, pos), 12);
    assert_eq!(offset_of!(BbSafehouse, name), 24);
    assert_eq!(size_of::<BbShop>(), 156);
    assert_eq!(offset_of!(BbShop, kind), 8);
    assert_eq!(size_of::<BbShopItem>(), 148);
    assert_eq!(offset_of!(BbShopItem, unlock_milestone), 12);
    assert_eq!(size_of::<BbShopItemDef>(), 276);
    assert_eq!(offset_of!(BbShopItemDef, apply), 8);
}

#[test]
fn races_table_and_records() {
    assert_eq!(size_of::<BbNfsmwRaces>(), 104);
    assert_eq!(offset_of!(BbNfsmwRaces, race_count), 16);
    assert_eq!(offset_of!(BbNfsmwRaces, race_at), 24);
    assert_eq!(offset_of!(BbNfsmwRaces, race_checkpoint_count), 32);
    assert_eq!(offset_of!(BbNfsmwRaces, race_checkpoint_at), 40);
    assert_eq!(offset_of!(BbNfsmwRaces, race_start), 48);
    assert_eq!(offset_of!(BbNfsmwRaces, race_abort), 56);
    assert_eq!(offset_of!(BbNfsmwRaces, race_current), 64);
    assert_eq!(offset_of!(BbNfsmwRaces, race_register), 72);
    assert_eq!(offset_of!(BbNfsmwRaces, race_set_opponent_skill), 80);
    assert_eq!(offset_of!(BbNfsmwRaces, race_set_reward), 88);
    assert_eq!(offset_of!(BbNfsmwRaces, race_results), 96);

    assert_eq!(size_of::<BbRaceInfo>(), 168);
    assert_eq!(offset_of!(BbRaceInfo, done), 20);
    assert_eq!(offset_of!(BbRaceInfo, pos), 24);
    assert_eq!(offset_of!(BbRaceInfo, name), 36);
    assert_eq!(size_of::<BbRaceState>(), 28);
    assert_eq!(offset_of!(BbRaceState, elapsed), 24);
    assert_eq!(size_of::<BbRaceResult>(), 20);
    assert_eq!(offset_of!(BbRaceResult, winnings), 16);
    assert_eq!(size_of::<BbRaceDef>(), 296);
    assert_eq!(offset_of!(BbRaceDef, start_heading), 28);
    assert_eq!(offset_of!(BbRaceDef, key), 32);
    assert_eq!(offset_of!(BbRaceDef, name), 164);
}
