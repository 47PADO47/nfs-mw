use super::logic::Category;
use super::options::{Data, Setting, Title, rows};
use crate::settings::{Partial, Settings, UiStyle};

#[test]
fn ui_style_row_follows_hud_layout_and_cycles_all_three_styles() {
    let rows = rows(Category::Gameplay, &crate::settings::test_caps::native());
    let index = rows.iter().position(|row| row.setting == Setting::HudLayout).unwrap() + 1;
    assert_eq!(rows[index].setting, Setting::UiStyle);
    assert_eq!(rows[index].title, Title::Text("UI style"));
    let (mut settings, mut changes) = (Settings::from(Partial::default()), Partial::default());
    assert_eq!(settings.ui_style, UiStyle::Original);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Original".into()));
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Custom);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Custom".into()));
    assert_eq!(changes, Partial { ui_style: Some(UiStyle::Custom), ..Partial::default() });
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Remastered);
    assert_eq!(Setting::UiStyle.data(&settings), Data::Text("Remastered (incomplete)".into()));
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, true));
    assert_eq!(settings.ui_style, UiStyle::Original, "the cycle wraps back to Original");
    assert!(Setting::UiStyle.step(&mut settings, &mut changes, false));
    assert_eq!(settings.ui_style, UiStyle::Remastered, "stepping back from Original wraps to Remastered");
}
