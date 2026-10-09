use super::ids::{LABEL_OFF, LABEL_ON};
use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{Partial, Settings};

#[test]
fn the_video_screen_has_one_exhaust_flames_row_after_the_vehicle_effects() {
    let video = rows(Category::Video);
    let at = video.iter().position(|r| r.setting == Setting::ExhaustFlames).expect("a row");
    assert_eq!(video[at].title, Title::Text("Exhaust Flames"));
    assert_eq!(video[at - 1].setting, Setting::SpeedTrails);
    // The pause menu shows the same rows as the main menu.
    assert_eq!(video.iter().filter(|r| r.setting == Setting::ExhaustFlames).count(), 1);
    for category in [Category::Audio, Category::Gameplay] {
        assert!(rows(category).iter().all(|r| r.setting != Setting::ExhaustFlames));
    }
}

#[test]
fn the_row_toggles_both_ways_and_records_only_its_own_change() {
    let initial = Settings::from(Partial::default());
    assert_eq!(Setting::ExhaustFlames.control(&initial), Control::Toggle);
    assert_eq!(Setting::ExhaustFlames.data(&initial), Data::Label(LABEL_ON), "the default is on");
    for forward in [true, false] {
        let (mut settings, mut changed) = (initial, Partial::default());
        for enabled in [false, true] {
            assert!(Setting::ExhaustFlames.step(&mut settings, &mut changed, forward));
            assert_eq!(settings, Settings { exhaust_flames: enabled, ..initial });
            assert_eq!(changed, Partial { exhaust_flames: Some(enabled), ..Partial::default() });
            let label = if enabled { LABEL_ON } else { LABEL_OFF };
            assert_eq!(Setting::ExhaustFlames.data(&settings), Data::Label(label));
        }
    }
}
