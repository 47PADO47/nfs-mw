use super::*;

fn tracks() -> Vec<TrackChoice> {
    vec![
        city(),
        TrackChoice { id: 11, title: "Circuit 1.1.1".into(), mode: Some(Mode::Circuit) },
        TrackChoice { id: 12, title: "Circuit 2.1.1".into(), mode: Some(Mode::Circuit) },
        TrackChoice { id: 21, title: "Sprint 1.2.1".into(), mode: Some(Mode::Sprint) },
    ]
}

/// Moves the cursor to `item` by pressing Down, as the keys would. By item, not by label: the rows the game's labels
/// name have no text in the tests, which run without the install's string table.
fn select(form: &mut HostForm, item: Item) {
    for _ in 0..12 {
        if form.item() == item {
            return;
        }
        form.down();
    }
    panic!("no row {item:?}");
}

/// The row of `item`.
fn row_of(form: &HostForm, item: Item) -> Row {
    let at = form.items().position(|shown| shown == item).unwrap_or_else(|| panic!("no row {item:?}"));
    form.rows()[at].clone()
}

#[test]
fn a_new_room_has_the_lan_defaults_and_the_city() {
    let form = HostForm::new("Pietro's game", tracks());
    let rows = form.rows();
    let value = |label: &str| rows.iter().find(|row| row.label == label).unwrap().value.clone();
    assert_eq!(value("Room name"), "Pietro's game");
    assert_eq!(value("Mode"), "Free Roam");
    assert_eq!(value("Track"), "Whole city");
    assert_eq!(value("Traffic density"), "50%");
    assert_eq!(value("Police"), "Up to heat 5");
    assert_eq!(value("Player limit"), "8");
    assert_eq!(row_of(&form, Item::Password).value, "none");
    assert!(!row_of(&form, Item::Laps).enabled, "free roam has no laps");
    assert_eq!(rows.last().unwrap().label, game(label::BACK));
}

#[test]
fn the_mode_brings_its_tracks_and_laps() {
    let mut form = HostForm::new("Room", tracks());
    select(&mut form, Item::Mode);
    form.step(true);
    assert_eq!(form.mode, Mode::Circuit);
    assert_eq!(form.track, 11, "the first circuit");
    select(&mut form, Item::Track);
    form.step(true);
    assert_eq!(form.track, 12);
    form.step(true);
    assert_eq!(form.track, 11, "wraps");
    select(&mut form, Item::Laps);
    form.step(true);
    assert_eq!(form.laps, 2);
    let settings = form.settings(&Settings::lan(ServerName::new("Room").unwrap()));
    assert_eq!((settings.mode, settings.track, settings.laps), (Mode::Circuit, 11, 2));
    assert_eq!(nfsmw_net::settings::validate(&settings), Ok(()));
}

#[test]
fn numbers_stay_in_range_and_police_off_stops_pursuits() {
    let mut form = HostForm::new("Room", tracks());
    select(&mut form, Item::Cops);
    for _ in 0..8 {
        form.step(false);
    }
    assert_eq!(form.cops, 0);
    select(&mut form, Item::Players);
    for _ in 0..20 {
        form.step(true);
    }
    assert_eq!(form.players, 16);
    select(&mut form, Item::Traffic);
    form.step(true);
    assert_eq!(form.traffic, 60);
    let settings = form.settings(&Settings::lan(ServerName::new("Room").unwrap()));
    assert!(!settings.pursuits && settings.cop_level_cap == 0);
    assert_eq!(nfsmw_net::settings::validate(&settings), Ok(()));
}

#[test]
fn text_rows_type_and_back_ends_typing_first() {
    let mut form = HostForm::new("Room", tracks());
    select(&mut form, Item::Password);
    assert_eq!(form.accept(), None);
    assert!(form.typing());
    for c in "pw".chars() {
        form.take(Typed::Char(c));
    }
    assert_eq!(row_of(&form, Item::Password).value, "**_");
    assert_eq!(form.back(), None, "Back ends typing");
    assert_eq!(form.password.value(), "pw");
    assert_eq!(form.back(), Some(FormOut::Back));
    select(&mut form, Item::Submit);
    assert_eq!(row_of(&form, Item::Submit).label, "Start Hosting");
    assert_eq!(form.accept(), Some(FormOut::Submit));
}

#[test]
fn an_empty_room_name_cannot_host() {
    let mut form = HostForm::new("", tracks());
    select(&mut form, Item::Submit);
    assert_eq!(form.accept(), None);
}

#[test]
fn the_settings_page_patches_only_what_changed() {
    let old = Settings::lan(ServerName::new("Room").unwrap());
    let mut form = HostForm::from_settings(&old, tracks());
    assert!(form.items().all(|item| item != Item::Password), "the password is set when hosting");
    assert_eq!(form.rows().iter().filter(|row| row.label == "Apply").count(), 1);
    assert_eq!(form.patch(&old), SettingsPatch::default());
    select(&mut form, Item::Players);
    form.step(false);
    select(&mut form, Item::Cops);
    form.step(false);
    let patch = form.patch(&old);
    assert_eq!(patch.max_players, Some(7));
    assert_eq!(patch.cop_level_cap, Some(4));
    assert_eq!((patch.mode, patch.track, patch.name.clone()), (None, None, None));
}
