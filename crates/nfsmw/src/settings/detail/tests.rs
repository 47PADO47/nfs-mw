use super::*;
use crate::settings::{Partial, Settings, file, write};

#[test]
fn profiles_round_trip_detail_and_preserve_future_keys() {
    for world in WorldDetail::ALL {
        for car in CarGeometry::ALL {
            let detail = DetailSettings {
                world,
                car,
                geometry_range: DetailRange::Double,
                visibility_range: DetailRange::Extended,
                highest_player: true,
                highest_world: true,
            };
            let settings = Settings::from(Partial { detail: Some(detail), ..Default::default() });
            let text = write::merge("game_dir='keep'\n[detail]\nfuture=42", &settings.graphics_layer()).unwrap();
            assert_eq!(Settings::from(file::parse(&text, "test")).detail, detail);
            assert!(text.contains("future = 42"));
            assert!(text.contains("game_dir = \"keep\""));
        }
    }
}

#[test]
fn invalid_profile_is_rejected_and_older_profiles_keep_defaults() {
    assert!(file::parse("[detail]\ngeometry_range='infinite'", "test").detail.is_none());
    assert!(file::parse("[detail]\nworld=12", "test").detail.is_none());
    assert_eq!(Settings::from(file::parse("post_aa='smaa'", "test")).detail, DetailSettings::default());
    let old = Partial {
        detail: Some(DetailSettings { world: WorldDetail::Low, ..Default::default() }),
        ..Default::default()
    };
    assert_eq!(Partial::default().or(old).detail, old.detail);
}

#[test]
fn authored_variants_are_exclusive_at_each_world_level() {
    for world in WorldDetail::ALL {
        assert!(world.admits(0));
        assert_eq!(world.admits(0x80000), world == WorldDetail::Maximum);
        assert_eq!(world.admits(0x40000), world != WorldDetail::Maximum);
        assert!(world.admits(0x40100));
    }
}

#[test]
fn menu_console_and_quality_choices_share_persistable_detail_settings() {
    use crate::frontend::graphics_options::GraphicsSetting;
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert!(GraphicsSetting::WorldDetail.step(&mut settings, &mut changed, true));
    assert_eq!(settings.detail.world, WorldDetail::Low);
    assert!(GraphicsSetting::CarGeometry.step(&mut settings, &mut changed, true));
    assert_eq!(settings.detail.car, CarGeometry::Low);
    assert_eq!(changed.detail, Some(settings.detail));
    settings.detail.geometry_range = DetailRange::Double;
    assert_eq!(settings.detail.geometry_range, DetailRange::Double);
    settings.detail.highest_player = true;
    assert!(settings.detail.highest_player);
    settings.select_graphics_quality(crate::settings::GraphicsQuality::High);
    assert_eq!(settings.detail.world, WorldDetail::Maximum);
    assert_eq!(settings.detail.geometry_range, DetailRange::Double);
    assert!(settings.detail.highest_player);
}
