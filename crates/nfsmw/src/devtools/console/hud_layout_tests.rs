use super::{parse, settings_cmd};
use crate::settings::{HudLayout, Partial, Settings};

#[test]
fn hud_layout_console_aliases_cycle_validate_and_explain_their_values() {
    let mut settings = Settings::from(Partial::default());
    assert_eq!(settings_cmd::get(&settings, "hud_layout").unwrap(), "hud_layout = pc");
    for key in ["hud_layout", "hud-layout"] {
        for layout in [HudLayout::Classic, HudLayout::Xbox360, HudLayout::Pc] {
            let token = layout.to_string();
            assert_eq!(settings_cmd::set(&mut settings, key, &token).unwrap(), format!("{key} = {token}"));
            assert_eq!(settings.hud_layout, layout);
            assert_eq!(settings_cmd::get(&settings, key).unwrap(), format!("{key} = {token}"));
        }
        let before = settings;
        assert!(settings_cmd::set(&mut settings, key, "wide").is_err());
        assert_eq!(
            settings_cmd::set(&mut settings, key, "").unwrap_err(),
            format!("usage: set {key} <pc|classic|xbox360> (now pc)")
        );
        assert_eq!(settings, before);
    }
    assert_eq!(settings_cmd::get_all(&settings).lines().filter(|line| line.starts_with("hud_layout = ")).count(), 1);
}

#[test]
fn hud_layout_console_shorthands_and_completion_are_available() {
    for key in ["hud_layout", "hud-layout"] {
        assert_eq!(
            parse::parse(&format!("{key} xbox360")).unwrap(),
            Some(parse::Command::Set { key: key.into(), value: "xbox360".into() })
        );
        assert!(parse::parse(&format!("{key} pc extra")).is_err());
        assert_eq!(parse::parse(&format!("get {key}")).unwrap(), Some(parse::Command::Get(Some(key.into()))));
    }
    assert!(parse::complete("hud-", &[]).contains(&"hud-layout".to_owned()));
}
