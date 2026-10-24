use super::fe_shadows::{self, FeShadows, FeShadowsArgs, FeShadowsLayer};
use super::{Partial, Settings, test_caps, write};
use toml::Table;

fn read(text: &str) -> FeShadowsLayer {
    let table: Table = text.parse().unwrap();
    fe_shadows::read(&table, "test")
}

#[test]
fn the_front_end_shadows_follow_driving_by_default() {
    let s = FeShadows::default();
    assert_eq!(s, FeShadows { enabled: true, cars: true, bias: 1.0, divider: 1 });
    assert_eq!(FeShadows::from_layer(FeShadowsLayer::default()), s);
    assert!(Settings::from(Partial::default()).fe_shadows.enabled);
}

#[test]
fn the_config_keys_are_read_and_bad_values_are_left_unset() {
    let layer = read("fe_shadows = true\nfe_shadows_cars = false\nfe_shadows_bias = 2.5\nfe_shadows_divider = 2\n");
    assert_eq!(layer, FeShadowsLayer { enabled: Some(true), cars: Some(false), bias: Some(2.5), divider: Some(2) });
    assert_eq!(read("fe_shadows = \"yes\"\n").enabled, None);
    assert_eq!(read("fe_shadows_divider = 8\n").divider, None, "only 1, 2 and 4 are dividers");
    assert_eq!(read("fe_shadows_divider = 0\n").divider, None);
    assert_eq!(read("vsync = true\n"), FeShadowsLayer::default());
}

#[test]
fn the_bias_is_clamped_to_its_range() {
    assert_eq!(read("fe_shadows_bias = 0.1\n").bias, Some(0.25));
    assert_eq!(read("fe_shadows_bias = 9.0\n").bias, Some(4.0));
    assert_eq!(read("fe_shadows_bias = 3\n").bias, Some(3.0), "an integer is a number too");
    assert_eq!(fe_shadows::clamp_bias(f32::NAN), 1.0);
    assert_eq!(FeShadows::from_layer(FeShadowsLayer { bias: Some(9.0), ..Default::default() }).bias, 4.0);
}

#[test]
fn the_dividers_are_1_2_and_4_only() {
    assert!(fe_shadows::is_divider(1) && fe_shadows::is_divider(2) && fe_shadows::is_divider(4));
    assert!(!fe_shadows::is_divider(3) && !fe_shadows::is_divider(8));
    assert_eq!(fe_shadows::parse_divider("2"), Ok(2));
    assert!(fe_shadows::parse_divider("8").is_err());
    assert!(fe_shadows::parse_divider("half").is_err());
}

#[test]
fn the_environment_and_the_switches_set_the_switch() {
    let env = |name: &str| (name == fe_shadows::ENV).then(|| "on".to_owned());
    assert_eq!(fe_shadows::from_env(env).enabled, Some(true));
    assert_eq!(fe_shadows::from_env(|_| Some("maybe".to_owned())).enabled, None);
    let on = FeShadowsArgs { fe_shadows: true, ..Default::default() };
    assert_eq!(on.layer().enabled, Some(true));
    let off = FeShadowsArgs { no_fe_shadows: true, ..Default::default() };
    assert_eq!(off.layer().enabled, Some(false));
    assert_eq!(FeShadowsArgs::default().layer(), FeShadowsLayer::default());
}

#[test]
fn the_higher_layer_wins_and_gaps_fall_through() {
    let high = FeShadowsLayer { enabled: Some(true), bias: Some(2.0), ..Default::default() };
    let low = FeShadowsLayer { enabled: Some(false), cars: Some(false), divider: Some(4), ..Default::default() };
    let merged = high.or(low);
    assert_eq!(merged, FeShadowsLayer { enabled: Some(true), cars: Some(false), bias: Some(2.0), divider: Some(4) });
}

#[test]
fn the_config_writer_and_the_reader_agree() {
    let s = FeShadows { enabled: true, cars: false, bias: 2.0, divider: 2 };
    let changes = Partial { fe_shadows: s.into(), ..Default::default() };
    let text = write::merge("vsync = true\n", &changes).unwrap();
    let table: Table = text.parse().unwrap();
    assert_eq!(table["vsync"].as_bool(), Some(true), "the other keys stay");
    assert_eq!(FeShadows::from_layer(fe_shadows::read(&table, "test")), s);
}

#[test]
fn the_front_end_shadows_support_native_casters_and_bevy_soft_shadows() {
    let s = Settings::from(Partial {
        fe_shadows: FeShadowsLayer { enabled: Some(true), ..Default::default() },
        ..Default::default()
    });
    assert!(s.effective(&test_caps::native()).fe_shadows.enabled);
    assert!(s.effective(&test_caps::full()).fe_shadows.enabled);
    let unsupported = blackbox_gfx::Capabilities { separate_shadow_casters: false, ..test_caps::native() };
    assert!(!s.effective(&unsupported).fe_shadows.enabled);
    let gl = blackbox_gfx::Capabilities { api: blackbox_gfx::GraphicsApi::Gl, ..test_caps::native() };
    assert!(!s.effective(&gl).fe_shadows.enabled);
}

#[test]
fn the_front_end_tuning_follows_the_switch_and_the_world_quality() {
    use super::fe_shadows::{Snapshot, tuning};
    use blackbox_gfx::ShadowQuality;
    use blackbox_gfx::settings::ShadowTuning;
    let on = FeShadows { enabled: true, bias: 2.0, divider: 2, ..Default::default() };
    let tuned = |world| tuning(Snapshot { options: on, world_quality: world });
    assert_eq!(tuned(ShadowQuality::Off), Some(ShadowTuning { quality: ShadowQuality::Off, bias: 2.0, divider: 2 }));
    assert_eq!(tuned(ShadowQuality::High).map(|t| t.quality), Some(ShadowQuality::High));
    let off =
        Snapshot { options: FeShadows { enabled: false, ..Default::default() }, world_quality: ShadowQuality::High };
    assert_eq!(tuning(off), None, "off keeps the world's own tuning");
}
