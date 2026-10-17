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
