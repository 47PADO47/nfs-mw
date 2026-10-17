use super::*;
use crate::settings::{Partial, Settings, file, write};

#[test]
fn presets_and_old_profiles_round_trip_without_resetting_other_preferences() {
    let mut settings = Settings::from(Partial::default());
    assert_eq!(settings.traffic_preset(), TrafficPreset::Reference);
    for preset in TrafficPreset::ALL {
        settings.select_traffic_preset(preset);
        assert_eq!(settings.traffic_preset(), preset);
        let text = write::merge("master_volume=17\n[traffic_tuning]\nfuture=42", &settings.graphics_layer()).unwrap();
        let restored = Settings::from(file::parse(&text, "test"));
        assert_eq!(restored.traffic_preset(), preset);
        assert_eq!(restored.master_volume.0, 17);
        assert_eq!(text.parse::<toml::Table>().unwrap()["traffic_tuning"]["future"].as_integer(), Some(42));
    }
    let legacy = Settings::from(file::parse("traffic=7", "legacy"));
    assert_eq!(legacy.traffic, 7);
    assert_eq!(legacy.traffic_policy, TrafficPolicy::default());
}

#[test]
fn unsafe_ranges_cannot_overlap_retirement_or_escape_bounds() {
    let policy = TrafficPolicy {
        spawn_min: u32::MAX,
        spawn_max: 0,
        retire_distance: 1,
        offscreen_seconds: 0,
        density_percent: 255,
        custom: true,
    }
    .sanitized();
    assert_eq!((policy.spawn_min, policy.spawn_max), (1200, 1250));
    assert_eq!((policy.retire_distance, policy.offscreen_seconds, policy.density_percent), (1450, 4, 100));
    assert_eq!(TrafficPolicy::default().density(0.75), 0.75);
    assert_eq!(TrafficPolicy { density_percent: 50, ..Default::default() }.density(0.75), 0.375);
    assert!(file::parse("[traffic_tuning]\nspawn_min='no'", "bad").traffic_policy.is_none());
}

#[test]
fn range_presets_preserve_density_and_budget_for_overlay_testing() {
    let mut settings = Settings::from(Partial::default());
    settings.traffic = 7;
    settings.traffic_policy.density_percent = 30;
    for expected in [TrafficPreset::Extended, TrafficPreset::Custom, TrafficPreset::Reference] {
        settings.select_traffic_preset(expected);
        assert_eq!(settings.traffic_preset(), expected);
        assert_eq!(settings.traffic, 7);
        assert_eq!(settings.traffic_policy.density_percent, 30);
    }
}
