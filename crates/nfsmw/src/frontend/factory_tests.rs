//! Tests for `factory::slot_for`: which menu slot each package is drawn for. No game data and no GPU are needed.

use super::*;

fn plain() -> Args {
    Args::default()
}

#[test]
fn main_menu_is_the_main_menu_slot() {
    assert_eq!(slot_for("MainMenu.fng", &plain()), Some(ElementId::MainMenu));
}

#[test]
fn main_menu_sub_is_categories_unless_it_is_the_safe_house() {
    assert_eq!(slot_for("MainMenu_Sub.fng", &plain()), Some(ElementId::Categories));
    let safehouse = Args { safehouse: true, ..Args::default() };
    assert_eq!(slot_for("MainMenu_Sub.fng", &safehouse), Some(ElementId::SafeHouseMenu));
}

#[test]
fn pause_main_is_the_pause_menu_or_the_categories_with_options() {
    assert_eq!(slot_for("Pause_Main.fng", &plain()), Some(ElementId::PauseMenu));
    let options = Args { options: true, ..Args::default() };
    assert_eq!(slot_for("Pause_Main.fng", &options), Some(ElementId::Categories));
}

#[test]
fn option_tables_share_the_option_table_slot() {
    assert_eq!(slot_for("Options.fng", &plain()), Some(ElementId::OptionTable));
    assert_eq!(slot_for("Pause_Options.fng", &plain()), Some(ElementId::OptionTable));
}

#[test]
fn track_select_is_its_own_slot() {
    assert_eq!(slot_for("Track_Select.fng", &plain()), Some(ElementId::TrackSelect));
}

#[test]
fn career_screens_take_their_career_slots() {
    let cases = [
        ("SafeHouseRivalChallenge.fng", ElementId::Blacklist),
        ("SafeHouseRivalBio.fng", ElementId::Blacklist),
        ("Car_Select.fng", ElementId::CarLot),
        ("RapSheetMain.fng", ElementId::RapSheet),
        ("RapSheetRankingsDetail.fng", ElementId::RapSheet),
        ("SafehouseRegionUnlock.fng", ElementId::RegionUnlock),
        ("MC_Main.fng", ElementId::MemoryCard),
        ("MC_List.fng", ElementId::MemoryCard),
        ("SMS_Mailboxes.fng", ElementId::Sms),
        ("SMS_Message.fng", ElementId::Sms),
    ];
    for (name, slot) in cases {
        assert_eq!(slot_for(name, &plain()), Some(slot), "{name}");
    }
}

#[test]
fn names_match_in_any_case() {
    assert_eq!(slot_for("mainmenu.FNG", &plain()), Some(ElementId::MainMenu));
    assert_eq!(slot_for("car_SELECT.fng", &plain()), Some(ElementId::CarLot));
}

#[test]
fn screens_outside_the_catalogue_take_no_slot() {
    let names = ["Track_Options.fng", "WorldMapMain.fng", "MW_LS_Splash.fng", "WS_MW_LS_Splash.fng"];
    for name in names {
        assert_eq!(slot_for(name, &plain()), None, "{name}");
    }
}

#[test]
fn the_dialog_the_key_list_the_playlist_and_the_credits_have_their_own_slots() {
    assert_eq!(slot_for("Dialog.fng", &plain()), Some(ElementId::Dialog));
    assert_eq!(slot_for("Options_PC_Controller.fng", &plain()), Some(ElementId::ControlsList));
    assert_eq!(slot_for("Pause_PC_Controller.fng", &plain()), Some(ElementId::ControlsList));
    assert_eq!(slot_for("EA_Trax_Jukebox.fng", &plain()), Some(ElementId::Jukebox));
    assert_eq!(slot_for("Credits.fng", &plain()), Some(ElementId::Credits));
}

#[test]
fn every_mapped_name_has_the_slot_the_menu_cell_feeds() {
    // A slot that `slot_for` names must be a menu slot: the frontend clears and sets only the menu cell.
    for name in ["MainMenu.fng", "Options.fng", "Car_Select.fng", "MC_Main.fng", "Track_Select.fng"] {
        let slot = slot_for(name, &plain()).expect("a catalogue screen has a slot");
        assert!(slot.is_menu(), "{name} maps to a HUD slot");
    }
}

#[test]
fn the_main_menu_and_its_sub_menus_ask_the_camera_for_their_poses() {
    assert_eq!(pose_for("MainMenu.fng", &plain()), Some("mainmenu"));
    let career = Args { sub: Sub::Career, ..Args::default() };
    assert_eq!(pose_for("MainMenu_Sub.fng", &career), Some("career_manager"));
    let quick = Args { sub: Sub::QuickRace, ..Args::default() };
    assert_eq!(pose_for("MainMenu_Sub.fng", &quick), Some("quickracemain_mainmenu_sub"));
    assert_eq!(pose_for("MainMenu_Sub.fng", &plain()), Some("options_mainmenu_sub"));
    // The safe house menu is the career's sub menu, and a pause menu's keeps the camera where it is.
    let safehouse = Args { safehouse: true, sub: Sub::Career, ..Args::default() };
    assert_eq!(pose_for("MainMenu_Sub.fng", &safehouse), Some("career_mainmenu_sub"));
    assert_eq!(pose_for("MainMenu_Sub.fng", &Args { pause: true, ..Args::default() }), None);
}

#[test]
fn the_option_screens_have_poses_and_the_dialogs_keep_the_camera() {
    assert_eq!(pose_for("Options.fng", &plain()), Some("options_options"));
    assert_eq!(pose_for("credits.fng", &plain()), Some("options_credits"));
    assert_eq!(pose_for("Dialog.fng", &plain()), None);
    assert_eq!(pose_for("Pause_Main.fng", &plain()), None);
}

#[test]
fn the_blacklist_pages_opened_from_the_pause_menu_keep_the_world_behind_them() {
    let pause = Args { pause: true, safehouse: true, ..Args::default() };
    assert_eq!(look_for(screen::SAFE_HOUSE_RIVAL_CHALLENGE, &pause), None);
    assert_eq!(pose_for(screen::SAFE_HOUSE_RIVAL_CHALLENGE, &pause), None);
    let room = Args { safehouse: true, ..Args::default() };
    assert!(look_for(screen::SAFE_HOUSE_RIVAL_CHALLENGE, &room).is_some());
    assert!(pose_for(screen::SAFE_HOUSE_RIVAL_CHALLENGE, &room).is_some());
}
