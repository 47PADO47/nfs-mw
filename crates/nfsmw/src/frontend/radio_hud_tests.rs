use super::logic::Category;
use super::options::{Data, Setting, Title, rows};
use crate::settings::{Partial, RadioHudStyle, Settings};

#[test]
fn radio_hud_row_follows_hud_layout_and_cycles_both_styles() {
    let rows = rows(Category::Gameplay, &crate::settings::test_caps::native());
    let index = rows.iter().position(|row| row.setting == Setting::HudLayout).unwrap() + 1;
    assert_eq!(rows[index].setting, Setting::RadioHud);
    assert_eq!(rows[index].title, Title::Text("Radio HUD"));
    let (mut settings, mut changes) = (Settings::from(Partial::default()), Partial::default());
    assert_eq!(settings.radio_hud, RadioHudStyle::EaTrax);
    assert_eq!(Setting::RadioHud.data(&settings), Data::Text("EA Trax".into()));
    assert!(Setting::RadioHud.step(&mut settings, &mut changes, true));
    assert_eq!(settings.radio_hud, RadioHudStyle::Custom);
    assert_eq!(Setting::RadioHud.data(&settings), Data::Text("Custom".into()));
    assert_eq!(changes, Partial { radio_hud: Some(RadioHudStyle::Custom), ..Partial::default() });
    assert!(Setting::RadioHud.step(&mut settings, &mut changes, false));
    assert_eq!(settings.radio_hud, RadioHudStyle::EaTrax);
}
