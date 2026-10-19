use super::logic::Category;
use super::options::{Data, Setting, rows};
use crate::settings::{Partial, Settings, UiStyle, UiTheme};

#[test]
fn ui_style_row_follows_hud_layout_and_cycles_all_three_styles() {
    let mut settings = Settings::from(Partial::default());
    let mut changes = Partial::default();
    let rows = rows(Category::Gameplay, &crate::settings::test_caps::native());
    let index = rows.iter().position(|row| row.setting == Setting::UiStyle).expect("the UI style row is in gameplay");
    assert_eq!(rows[index].setting, Setting::UiStyle);
    assert_eq!(settings.ui_style, UiStyle::Original);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Original".into()));
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Custom);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Custom".into()));
    assert_eq!(changes, Partial { ui_style: Some(UiStyle::Custom), ..Partial::default() });
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Remastered);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Remastered".into()));
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Original, "the cycle wraps back to Original");
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, false));
    assert_eq!(settings.ui_style, UiStyle::Remastered, "stepping back from Original wraps to Remastered");
}

#[test]
fn hud_theme_row_cycles_remastered_and_bevy() {
    let mut settings = Settings::from(Partial::default());
    let mut changes = Partial::default();
    assert_eq!(settings.ui_theme, UiTheme::Bevy);
    assert_eq!(Setting::UiTheme.data(&settings), Data::Text("Bevy".into()));
    assert!(Setting::UiTheme.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_theme, UiTheme::Remastered);
    assert_eq!(Setting::UiTheme.data(&settings), Data::Text("Remastered".into()));
    assert_eq!(changes, Partial { ui_theme: Some(UiTheme::Remastered), ..Partial::default() });
    assert!(Setting::UiTheme.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_theme, UiTheme::Bevy, "the cycle wraps back to Bevy");
}
