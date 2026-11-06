//! Legacy music-volume compatibility and independent context controls.

pub(super) fn write(changes: &super::Partial, put: &mut dyn FnMut(&str, toml::Value)) {
    for (key, value) in [
        ("music_volume", changes.music_volume),
        ("menu_music_volume", changes.menu_music_volume),
        ("game_music_volume", changes.game_music_volume),
    ] {
        if let Some(value) = value {
            put(key, toml::Value::Integer(i64::from(value.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::settings::{Partial, Percent, Settings, file, write};

    #[test]
    fn batch18_music_legacy_defaults_and_context_layers_preserve_priority() {
        let pair = |p: Partial| {
            let s = Settings::from(p);
            (s.menu_music_volume, s.game_music_volume)
        };
        assert_eq!(pair(Partial::default()), (Percent(80), Percent(80)));
        let legacy = Partial { music_volume: Some(Percent(25)), ..Default::default() };
        assert_eq!(pair(legacy), (Percent(25), Percent(25)));
        let split = Partial {
            music_volume: Some(Percent(45)),
            menu_music_volume: Some(Percent(20)),
            game_music_volume: Some(Percent(80)),
            ..Default::default()
        };
        assert_eq!(pair(split), (Percent(20), Percent(80)), "explicit keys win within one layer");
        assert_eq!(pair(legacy.or(split)), (Percent(25), Percent(25)), "higher common override sets both");
        let menu = Partial { menu_music_volume: Some(Percent(0)), ..Default::default() };
        assert_eq!(pair(menu.or(split)), (Percent(0), Percent(80)), "mute one context without resetting another");
    }

    #[test]
    fn batch18_music_config_and_alias_round_trip_keep_independent_values() {
        let source = "music_volume = 35\nmenu_music_volume = 10\ngame_music_volume = 90\ncustom = 'keep'\n";
        let mut settings = Settings::from(file::parse(source, "test"));
        assert_eq!((settings.menu_music_volume, settings.game_music_volume), (Percent(10), Percent(90)));
        let changes = Partial { menu_music_volume: Some(Percent(0)), ..Default::default() };
        let text = write::merge(source, &changes).unwrap();
        let parsed = Settings::from(file::parse(&text, "test"));
        assert_eq!((parsed.menu_music_volume, parsed.game_music_volume), (Percent(0), Percent(90)));
        assert!(text.contains("keep"));
        settings.menu_music_volume = Percent(20);
        settings.game_music_volume = Percent(75);
        let alias = crate::saves::partial_of(&settings);
        let restored = Settings::from(file::parse(&write::merge("", &alias).unwrap(), "alias"));
        assert_eq!((restored.menu_music_volume, restored.game_music_volume), (Percent(20), Percent(75)));
        let old = file::parse("music_volume = 0", "old alias");
        let old = Settings::from(old.or(alias));
        assert_eq!((old.menu_music_volume, old.game_music_volume), (Percent(0), Percent(0)));
    }
    #[test]
    fn batch18_music_environment_priority_and_invalid_values_fall_through_per_context() {
        let lower = file::parse("menu_music_volume = 20\ngame_music_volume = 80", "file");
        let high = crate::settings::env::read(|name| match name {
            "NFSMW_MUSIC_VOLUME" => Some("35".into()),
            "NFSMW_MENU_MUSIC_VOLUME" => Some("5%".into()),
            "NFSMW_GAME_MUSIC_VOLUME" => Some("invalid".into()),
            _ => None,
        });
        let settings = Settings::from(high.or(lower));
        assert_eq!((settings.menu_music_volume, settings.game_music_volume), (Percent(5), Percent(35)));
    }
}
