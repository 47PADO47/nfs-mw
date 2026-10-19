use super::logic::Category;
use super::options::{Data, Setting, rows};
use crate::settings::{Partial, Settings, SpeedUnits};

#[test]
fn speed_units_row_is_in_gameplay_and_cycles_both_units() {
    let mut settings = Settings::from(Partial::default());
    let mut changes = Partial::default();
    let rows = rows(Category::Gameplay, &crate::settings::test_caps::native());
    assert!(rows.iter().any(|row| row.setting == Setting::SpeedUnits), "the speed units row is in gameplay");
    assert_eq!(settings.speed_units, SpeedUnits::Metric);
    assert_eq!(Setting::SpeedUnits.data(&settings), Data::Text("Metric (km/h)".into()));
    assert!(Setting::SpeedUnits.step(&mut settings, &mut changes, true));
    assert_eq!(settings.speed_units, SpeedUnits::Imperial);
    assert_eq!(Setting::SpeedUnits.data(&settings), Data::Text("Imperial (mph)".into()));
    assert_eq!(changes, Partial { speed_units: Some(SpeedUnits::Imperial), ..Partial::default() });
    assert!(Setting::SpeedUnits.step(&mut settings, &mut changes, true));
    assert_eq!(settings.speed_units, SpeedUnits::Metric, "the cycle wraps back to metric");
}
