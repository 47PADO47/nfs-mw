//! Old profiles must not reactivate the removed custom traffic rules.

use super::{Partial, Settings, env, file, write};

#[test]
fn traffic_parity_discards_the_removed_extension_from_saved_profiles() {
    let original = "traffic_avoidance = true\ntraffic = 20\ntraffic_lights = false\nother_mod_key = 7\n";
    let saved = write::merge(original, &Partial::default()).expect("valid old profile");
    let table: toml::Table = saved.parse().expect("valid saved profile");
    assert!(!table.contains_key("traffic_avoidance"), "obsolete extension survives a profile save");
    assert_eq!(table["traffic"].as_integer(), Some(20));
    assert_eq!(table["traffic_lights"].as_bool(), Some(false));
    assert_eq!(table["other_mod_key"].as_integer(), Some(7));
    let settings = Settings::from(file::parse(&saved, "saved profile"));
    assert_eq!(settings.traffic, 20);
    assert!(!settings.traffic_lights);
}

#[test]
fn traffic_parity_ignores_the_removed_key_in_old_files_and_environments() {
    let baseline = Settings::from(Partial::default());
    let legacy = Settings::from(file::parse("traffic_avoidance = true", "old profile"));
    let environment = Settings::from(env::read(|name| (name == "NFSMW_TRAFFIC_AVOIDANCE").then(|| "true".to_owned())));
    assert_eq!(super::TrafficSettings::from(&baseline), super::TrafficSettings::from(&legacy));
    assert_eq!(super::TrafficSettings::from(&baseline), super::TrafficSettings::from(&environment));
}
