use super::*;
use crate::economy::markers::{ENTRY_COUNT, MarkerKind};
use crate::settings::CareerSettings;

fn cash(amount: i32) -> Reward {
    Reward { kind: Some(KIND_CASH_BONUS), value: amount, ..Reward::default() }
}

fn marker(kind: u8) -> Reward {
    Reward { kind: Some(kind), ..Reward::default() }
}

fn pink(preset: &str) -> Reward {
    Reward { kind: Some(KIND_PINK_SLIP), preset: Some(preset.to_owned()), ..Reward::default() }
}

fn part(number: u32) -> Reward {
    Reward { part: Some(number), ..Reward::default() }
}

fn nobody_owns(_: &str) -> bool {
    false
}

fn inventory_of(career: &CareerSettings) -> MarkerInventory {
    MarkerInventory::from_table(&career.inventory[..]).unwrap()
}

/// Fills every inventory slot with a jail-free marker.
fn fill_inventory(career: &mut CareerSettings) {
    let mut inventory = MarkerInventory::empty();
    assert_eq!(inventory.add_entries(0x11, 0, ENTRY_COUNT as u32), ENTRY_COUNT as u32);
    career.inventory.copy_from_slice(&inventory.to_table());
}

#[test]
fn cash_bonus_adds_its_value_at_once() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    let effects = grant(&[cash(500)], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::CashAdded(500)]);
    assert_eq!(career.cash, 600);
    assert_eq!(inventory_of(&career).count_entries(0x13, 0), 0, "cash is not stored in the inventory");
}

#[test]
fn negative_cash_value_has_no_route() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    let effects = grant(&[cash(-5)], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::Unknown(cash(-5))]);
    assert_eq!(career.cash, 100);
}

#[test]
fn marker_kinds_are_stored_in_the_inventory() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let rewards = [marker(0x11), marker(0x14), marker(0x15)];
    let effects = grant(&rewards, false, &mut career, &mut popups, nobody_owns);
    assert_eq!(
        effects,
        vec![
            RewardEffect::MarkerAdded { kind: 0x11 },
            RewardEffect::MarkerAdded { kind: 0x14 },
            RewardEffect::MarkerAdded { kind: 0x15 },
        ]
    );
    let inventory = inventory_of(&career);
    assert_eq!(inventory.count(MarkerKind::JailFree), 1);
    assert_eq!(inventory.count(MarkerKind::ExtraStrike), 1);
    assert_eq!(inventory.count(MarkerKind::ImpoundRelease), 1);
    assert_eq!(career.cash, 0, "markers pay nothing");
}

#[test]
fn other_non_zero_kinds_are_stored_as_markers() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let effects = grant(&[marker(0x16)], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::MarkerAdded { kind: 0x16 }]);
    assert_eq!(inventory_of(&career).count_entries(0x16, 0), 1);
}

#[test]
fn pink_slip_is_not_stored_and_asks_for_the_car() {
    let mut career = CareerSettings::new_career();
    career.cash = 40;
    let mut popups = PopupQueue::new();
    let effects = grant(&[pink("BL05")], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::AddCar { preset: "BL05".to_owned() }]);
    assert_eq!(inventory_of(&career).count_entries(0x12, 0), 0);
    assert_eq!(career.cash, 40);
}

#[test]
fn pink_slip_for_a_car_already_owned_adds_nothing() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let owns = |preset: &str| preset == "BL05";
    let effects = grant(&[pink("BL05")], false, &mut career, &mut popups, owns);
    assert_eq!(effects, vec![RewardEffect::CarAlreadyOwned { preset: "BL05".to_owned() }]);
}

#[test]
fn the_same_preset_twice_in_one_list_is_granted_once() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let effects = grant(&[pink("BL07"), pink("BL07")], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(
        effects,
        vec![
            RewardEffect::AddCar { preset: "BL07".to_owned() },
            RewardEffect::CarAlreadyOwned { preset: "BL07".to_owned() },
        ]
    );
}

#[test]
fn repeat_grant_does_not_duplicate_a_pink_slip_car() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let mut owned: Vec<String> = Vec::new();
    let first = grant(&[pink("BL09")], false, &mut career, &mut popups, |p| owned.iter().any(|o| o == p));
    assert_eq!(first, vec![RewardEffect::AddCar { preset: "BL09".to_owned() }]);
    // The caller adds the car to the stable and records it; the next grant must see it.
    owned.push("BL09".to_owned());
    let second = grant(&[pink("BL09")], false, &mut career, &mut popups, |p| owned.iter().any(|o| o == p));
    assert_eq!(second, vec![RewardEffect::CarAlreadyOwned { preset: "BL09".to_owned() }]);
}

#[test]
fn a_part_is_unlocked_and_stored_under_its_number() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let effects = grant(&[part(4)], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::PartUnlocked { part: 4 }]);
    assert_eq!(inventory_of(&career).count_entries(4, 0), 1);
}

#[test]
fn a_part_takes_precedence_over_a_kind() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let both = Reward { part: Some(2), kind: Some(0x11), ..Reward::default() };
    let effects = grant(&[both], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::PartUnlocked { part: 2 }]);
    assert_eq!(inventory_of(&career).count_entries(0x11, 0), 0);
}

#[test]
fn parts_outside_one_to_sixteen_have_no_route() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let effects = grant(&[part(0), part(0x11)], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::Unknown(part(0)), RewardEffect::Unknown(part(0x11))]);
    assert_eq!(inventory_of(&career), MarkerInventory::empty());
}

#[test]
fn rewards_without_a_route_change_nothing() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let no_kind = Reward::default();
    let kind_zero = marker(0);
    let pink_no_preset = Reward { kind: Some(KIND_PINK_SLIP), ..Reward::default() };
    let rewards = [no_kind.clone(), kind_zero.clone(), pink_no_preset.clone()];
    let effects = grant(&rewards, false, &mut career, &mut popups, nobody_owns);
    assert_eq!(
        effects,
        vec![RewardEffect::Unknown(no_kind), RewardEffect::Unknown(kind_zero), RewardEffect::Unknown(pink_no_preset),]
    );
    assert_eq!(inventory_of(&career), MarkerInventory::empty());
    assert!(popups.entries().is_empty());
}

#[test]
fn deferred_rewards_are_queued_not_applied() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    let rewards = [cash(500), marker(0x11), pink("BL05"), part(2)];
    let effects = grant(&rewards, true, &mut career, &mut popups, nobody_owns);
    let queued: Vec<RewardEffect> = rewards.iter().cloned().map(RewardEffect::Queued).collect();
    assert_eq!(effects, queued);
    assert_eq!(popups.entries(), &rewards);
    assert_eq!(career.cash, 100);
    assert_eq!(inventory_of(&career), MarkerInventory::empty());
}

#[test]
fn deferred_rewards_without_a_route_are_not_queued() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let effects = grant(&[Reward::default()], true, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::Unknown(Reward::default())]);
    assert!(popups.entries().is_empty());
}

#[test]
fn queued_rewards_apply_like_immediate_ones() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    grant(&[cash(500), marker(0x11)], true, &mut career, &mut popups, nobody_owns);
    let first = popups.take(0).unwrap();
    let second = popups.take(0).unwrap();
    assert!(popups.entries().is_empty());
    let effects = grant(&[first, second], false, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::CashAdded(500), RewardEffect::MarkerAdded { kind: 0x11 }]);
    assert_eq!(career.cash, 600);
    assert_eq!(inventory_of(&career).count_entries(0x11, 0), 1);
}

#[test]
fn popup_queue_holds_six_entries_and_returns_the_seventh() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let six: Vec<Reward> = (0..POPUP_SLOTS).map(|_| marker(0x11)).collect();
    let effects = grant(&six, true, &mut career, &mut popups, nobody_owns);
    assert!(effects.iter().all(|e| matches!(e, RewardEffect::Queued(_))));
    assert!(popups.is_full());

    let seventh = pink("BL11");
    let effects = grant(std::slice::from_ref(&seventh), true, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::QueueFull(seventh)]);
    assert_eq!(popups.entries().len(), POPUP_SLOTS);
}

#[test]
fn a_full_inventory_refuses_markers_and_parts_but_pays_cash() {
    let mut career = CareerSettings::new_career();
    fill_inventory(&mut career);
    career.cash = 0;
    let mut popups = PopupQueue::new();
    let rewards = [marker(0x14), part(3), cash(10)];
    let effects = grant(&rewards, false, &mut career, &mut popups, nobody_owns);
    assert_eq!(
        effects,
        vec![
            RewardEffect::InventoryFull(marker(0x14)),
            RewardEffect::InventoryFull(part(3)),
            RewardEffect::CashAdded(10)
        ]
    );
    let inventory = inventory_of(&career);
    assert_eq!(inventory.count_entries(0x11, 0), ENTRY_COUNT as u32, "the stored markers are untouched");
    assert_eq!(inventory.count_entries(0x14, 0), 0);
    assert_eq!(career.cash, 10);
}

#[test]
fn a_repeat_gauntlet_win_grants_nothing() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    let effects =
        grant_gauntlet_rewards(&[cash(500), pink("BL15")], false, false, &mut career, &mut popups, nobody_owns);
    assert!(effects.is_empty());
    assert_eq!(career.cash, 100);
    assert_eq!(inventory_of(&career), MarkerInventory::empty());
    assert!(popups.entries().is_empty());
}

#[test]
fn the_first_gauntlet_win_grants_the_list() {
    let mut career = CareerSettings::new_career();
    career.cash = 100;
    let mut popups = PopupQueue::new();
    let rewards = [cash(500), marker(0x11), pink("BL15")];
    let effects = grant_gauntlet_rewards(&rewards, true, false, &mut career, &mut popups, nobody_owns);
    assert_eq!(
        effects,
        vec![
            RewardEffect::CashAdded(500),
            RewardEffect::MarkerAdded { kind: 0x11 },
            RewardEffect::AddCar { preset: "BL15".to_owned() },
        ]
    );
    assert_eq!(career.cash, 600);
}

#[test]
fn a_deferred_gauntlet_win_queues_the_list() {
    let mut career = CareerSettings::new_career();
    let mut popups = PopupQueue::new();
    let rewards = [marker(0x15)];
    let effects = grant_gauntlet_rewards(&rewards, true, true, &mut career, &mut popups, nobody_owns);
    assert_eq!(effects, vec![RewardEffect::Queued(marker(0x15))]);
    assert_eq!(popups.entries(), &rewards);
}
