use blackbox_text::label_hash;
use nfsmw_career::settings::{CareerSettings, Flags};

use super::*;
use crate::frontend::icon_items::Id;

/// The items of the menu for a career, as the screen lists them (the memory-card layer is always there).
fn items(settings: &CareerSettings) -> Vec<MenuItem> {
    visible_items(settings, MEMORY_CARD_LAYER)
}

#[test]
fn a_new_career_lists_resume_blacklist_car_select_and_save() {
    let s = CareerSettings::new_career();
    assert_eq!(items(&s), [MenuItem::ResumeFreeRoam, MenuItem::Blacklist(16), MenuItem::CarSelect, MenuItem::Save]);
}

#[test]
fn the_rap_sheet_is_listed_after_car_select_once_its_flag_is_set() {
    let mut s = CareerSettings::new_career();
    s.stage = 13;
    s.flags.insert(Flags::HAS_RAP_SHEET);
    let list = items(&s);
    assert_eq!(
        list,
        [MenuItem::ResumeFreeRoam, MenuItem::Blacklist(13), MenuItem::CarSelect, MenuItem::RapSheet, MenuItem::Save]
    );
    assert_eq!(list.len(), 5);
}

#[test]
fn the_blacklist_item_shows_the_current_stage_and_the_icons_follow_the_items() {
    let mut s = CareerSettings::new_career();
    s.stage = 1;
    let list = items(&s);
    let icons = safehouse_icons(&list);
    assert_eq!(icons.len(), list.len());
    assert_eq!(icons[1].id, Id::SafeHouseBlacklist(1));
    assert_eq!(icons[0].id, Id::SafeHouseResume);
    assert_eq!(icons[0].name, label_hash("PAUSE_RESUME_FREE_ROAM"));
    assert!(icons.iter().all(|icon| icon.enabled));
}

#[test]
fn each_item_opens_the_documented_target_in_the_safe_house() {
    let args = Args { safehouse: true, ..Args::default() };
    assert_eq!(
        command_for(select(MenuItem::ResumeFreeRoam), args),
        Command::Confirm(Confirm::new(Text::Game(label_hash("CONFIRM_RESUME_FREE_ROAM")), Command::Resume))
    );
    assert_eq!(
        command_for(select(MenuItem::Blacklist(9)), args),
        Command::Switch(screen::SAFE_HOUSE_RIVAL_CHALLENGE.into(), args)
    );
    assert_eq!(command_for(select(MenuItem::CarSelect), args), Command::Switch(screen::CAR_SELECT.into(), args));
    assert_eq!(command_for(select(MenuItem::RapSheet), args), Command::Switch(screen::RAP_SHEET_LOGIN.into(), args));
    assert_eq!(command_for(select(MenuItem::Save), args), Command::Switch(screen::MC_MAIN.into(), args));
}

#[test]
fn an_icon_maps_back_to_its_item_and_other_icons_map_to_none() {
    assert_eq!(item_of(Id::SafeHouseBlacklist(13)), Some(MenuItem::Blacklist(13)));
    assert_eq!(item_of(Id::SafeHouseSave), Some(MenuItem::Save));
    assert_eq!(item_of(Id::Resume), None);
}

#[test]
fn the_totals_show_the_bounty_and_the_cash_with_commas() {
    assert_eq!(money_text(1_777), "$1,777");
    assert_eq!(money_text(0), "$0");
    assert_eq!(money_text(u64::MAX), format!("${}", money(u32::MAX)));
}
