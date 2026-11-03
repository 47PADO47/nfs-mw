use super::*;
use crate::settings::{Partial, Settings, file, write};

#[test]
fn batch18_map_modes_have_context_defaults_and_preserve_legacy_profiles() {
    let defaults = Settings::from(Partial::default());
    assert_eq!(
        Settings::from(crate::saves::partial_of(&defaults)),
        defaults,
        "resolved defaults survive settings snapshots"
    );
    assert_eq!(defaults.minimap_mode(MinimapContext::FreeRoam), MinimapMode::Fixed);
    assert_eq!(defaults.minimap_mode(MinimapContext::Race), MinimapMode::Rotating);
    for mode in [MinimapMode::Fixed, MinimapMode::Rotating, MinimapMode::Off] {
        let s = Settings::from(file::parse(&format!("minimap = '{mode}'"), "test"));
        assert_eq!(s.minimap_mode(MinimapContext::FreeRoam), mode);
        assert_eq!(s.minimap_mode(MinimapContext::Race), mode);
    }
}

#[test]
fn batch18_map_mode_layers_respect_context_and_legacy_priority() {
    let file = file::parse("minimap = 'off'\nfree_roam_map_mode = 'fixed'\nrace_map_mode = 'rotating'", "test");
    let assert_modes = |p, free, race| {
        let s = Settings::from(p);
        assert_eq!(s.minimap_mode(MinimapContext::FreeRoam), free);
        assert_eq!(s.minimap_mode(MinimapContext::Race), race);
    };
    assert_modes(file, MinimapMode::Fixed, MinimapMode::Rotating);
    assert_modes(Partial::default().or(file), MinimapMode::Fixed, MinimapMode::Rotating);
    assert_modes(
        Partial { minimap: Some(MinimapMode::Off), ..Partial::default() }.or(file),
        MinimapMode::Off,
        MinimapMode::Off,
    );
    assert_modes(
        Partial { race_map_mode: Some(MinimapMode::Off), ..Partial::default() }.or(file),
        MinimapMode::Fixed,
        MinimapMode::Off,
    );
    let env = crate::settings::env::read(|name| (name == "NFSMW_RACE_MAP_MODE").then(|| "off".into()));
    assert_modes(env.or(file), MinimapMode::Fixed, MinimapMode::Off);
    let invalid = file::parse("free_roam_map_mode = true\nrace_map_mode = 'zoomed'", "test");
    assert_modes(invalid.or(file), MinimapMode::Fixed, MinimapMode::Rotating);
}

#[test]
fn batch18_context_map_edits_persist_independently_and_keep_other_keys() {
    let existing = "minimap = 'fixed'\nfree_roam_map_mode = 'off'\nfuture_option = 42";
    let changes = Partial { race_map_mode: Some(MinimapMode::Rotating), ..Partial::default() };
    let saved = write::merge(existing, &changes).unwrap();
    let parsed = file::parse(&saved, "test");
    assert_eq!(parsed.race_map_mode, Some(MinimapMode::Rotating));
    assert_eq!(parsed.free_roam_map_mode, Some(MinimapMode::Off));
    assert_eq!(parsed.minimap, Some(MinimapMode::Fixed));
    assert_eq!(saved.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(42));
}

#[test]
fn batch18_global_map_edits_and_setting_snapshots_preserve_both_contexts() {
    let mut s = Settings::from(Partial {
        free_roam_map_mode: Some(MinimapMode::Off),
        race_map_mode: Some(MinimapMode::Rotating),
        ..Partial::default()
    });
    let snapshot = Settings::from(crate::saves::partial_of(&s));
    assert_eq!(snapshot.minimap_mode(MinimapContext::FreeRoam), MinimapMode::Off);
    assert_eq!(snapshot.minimap_mode(MinimapContext::Race), MinimapMode::Rotating);
    s.set_global_minimap(MinimapMode::Fixed);
    for context in [MinimapContext::FreeRoam, MinimapContext::Race] {
        assert_eq!(s.minimap_mode(context), MinimapMode::Fixed);
    }
    let mut change = Partial::default();
    change.set_global_minimap(MinimapMode::Fixed);
    let saved = write::merge("free_roam_map_mode = 'off'\nrace_map_mode = 'rotating'", &change).unwrap();
    let restored = Settings::from(file::parse(&saved, "test"));
    assert_eq!(restored.minimap_mode(MinimapContext::FreeRoam), MinimapMode::Fixed);
    assert_eq!(restored.minimap_mode(MinimapContext::Race), MinimapMode::Fixed);
}
