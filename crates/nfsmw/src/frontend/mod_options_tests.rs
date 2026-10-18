use super::*;

fn info(kind: MenuKind, value: f32) -> RowInfo {
    RowInfo { handle: 1, label: "Row".into(), kind, value }
}

#[test]
fn a_toggle_shows_on_or_off_with_the_switch_control() {
    let on = describe(info(MenuKind::Toggle, 1.0));
    assert_eq!((on.control, on.data), (Control::Toggle, Data::Label(LABEL_ON)));
    assert_eq!(describe(info(MenuKind::Toggle, 0.0)).data, Data::Label(LABEL_OFF));
}

#[test]
fn a_slider_shows_where_its_value_lies_in_its_range() {
    let slider = MenuKind::Slider { min: 0.0, max: 10.0, step: 1.0 };
    assert_eq!(describe(info(slider, 2.5)).control, Control::Slider(25));
    assert_eq!(percent(5.0, 5.0, 5.0), 0, "an empty range shows the bottom");
    assert_eq!(percent(20.0, 0.0, 10.0), 100, "past the top is the top");
}

#[test]
fn a_list_shows_its_chosen_text_and_a_button_says_press() {
    let list = MenuKind::List { choices: vec!["low".into(), "high".into()] };
    assert_eq!(describe(info(list.clone(), 1.0)).data, Data::Text("high".into()));
    assert_eq!(describe(info(list, 9.0)).data, Data::Text(String::new()), "an index past the end shows nothing");
    assert_eq!(describe(info(MenuKind::Button, 0.0)).data, Data::Text("Press".into()));
}

#[test]
fn a_setting_that_is_not_a_mods_row_has_no_mod_view() {
    assert!(shown(None, Setting::Mod(1)).is_none(), "without mods there is nothing to show");
    assert!(shown(None, Setting::MasterVolume).is_none());
}

fn switch(id: &str, loaded: bool, enabled: bool) -> Switch {
    Switch { id: id.into(), loaded, enabled }
}

#[test]
fn a_switch_shows_its_mod_its_value_and_what_the_next_start_changes() {
    let on = describe_switch(&switch("a.mod", true, true));
    assert_eq!(on.label, "a.mod");
    assert_eq!((on.control, on.data), (Control::Toggle, Data::Label(LABEL_ON)));
    let unload = describe_switch(&switch("a.mod", true, false));
    assert_eq!(unload.label, "a.mod (restart to unload)");
    assert_eq!(unload.data, Data::Label(LABEL_OFF));
    let load = describe_switch(&switch("b.mod", false, true));
    assert_eq!(load.label, "b.mod (restart to load)");
    assert_eq!(describe_switch(&switch("c.mod", false, false)).label, "c.mod", "refused and still off needs no note");
}

#[test]
fn without_mods_there_are_no_switches_to_show_or_change() {
    assert!(rows(None).is_empty());
    assert!(shown(None, Setting::ModManaged(0)).is_none());
    change(None, Setting::ModManaged(0), true);
}

#[test]
fn the_ui_theme_row_is_offered_only_when_a_mod_registered_a_theme() {
    let row = theme_row(true).expect("a mod theme is registered, so the choice is offered");
    assert_eq!(row.setting, Setting::ModTheme);
    assert!(theme_row(false).is_none(), "no mod theme, no choice");
}

#[test]
fn the_ui_theme_row_shows_the_chosen_theme_or_the_built_in_style() {
    let built_in = describe_theme(None);
    assert_eq!(built_in.label, "UI theme");
    assert_eq!((built_in.control, built_in.data), (Control::Toggle, Data::Text("Built-in style".into())));
    assert_eq!(describe_theme(Some("Night drive".into())).data, Data::Text("Night drive".into()));
}

#[test]
fn without_mods_there_is_no_ui_theme_row_to_show_or_change() {
    assert!(shown(None, Setting::ModTheme).is_none());
    change(None, Setting::ModTheme, true);
}
