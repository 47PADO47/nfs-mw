//! The icon rows of the career-parity menus: which icons each menu lists and what the pause menu names. No game data is needed.

#[test]
fn the_quick_race_menu_lists_quick_play_custom_race_and_split_screen() {
    use super::icon_items::{Id, icons};
    use super::icon_menu::Kind;
    let list = icons(Kind::QuickRace);
    let ids: Vec<_> = list.iter().map(|i| (i.id, i.enabled)).collect();
    assert_eq!(ids, [(Id::QuickPlay, true), (Id::CustomRace, true), (Id::SplitScreen, true)]);
}

#[test]
fn the_alias_manager_lists_create_load_save_delete() {
    use super::icon_items::{Id, icons};
    use super::icon_menu::Kind;
    let ids: Vec<_> = icons(Kind::AliasManager).iter().map(|i| i.id).collect();
    assert_eq!(ids, [Id::AliasCreate, Id::AliasLoad, Id::AliasSave, Id::AliasDelete]);
}

#[test]
fn the_pause_menu_in_a_race_offers_resume_race_and_restart_and_names_its_quit() {
    use super::icon_items::{Id, pause_icons};
    use super::logic::{PauseCtx, QuitTo};
    let free = pause_icons(PauseCtx::default());
    assert_eq!(free.iter().map(|i| i.id).collect::<Vec<_>>(), [Id::Resume, Id::Options, Id::Quit(QuitTo::Main)]);
    let race = pause_icons(PauseCtx { race: true, quit: QuitTo::QuickRace, coop: false, coop_locked: false });
    assert_eq!(
        race.iter().map(|i| i.id).collect::<Vec<_>>(),
        [Id::ResumeRace, Id::RestartRace, Id::Options, Id::Quit(QuitTo::QuickRace)]
    );
    assert_eq!(race[0].name, 0xDED3_57E7, "Resume Race");
    assert_eq!(race[3].name, 0x4349_998B, "Quit to Quick Race Menu");
    let career = pause_icons(PauseCtx { race: true, quit: QuitTo::Career, coop: false, coop_locked: false });
    assert_eq!(career[3].name, 0x690E_9B7C, "Quit to Career Main Menu");
}
