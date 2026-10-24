use super::xenon_effects::{self, XenonEffects, XenonEffectsLayer};
use super::{Partial, Settings};
use toml::Table;

fn read(text: &str) -> XenonEffectsLayer {
    let table: Table = text.parse().unwrap();
    xenon_effects::read(&table)
}

#[test]
fn the_defaults_keep_the_effects_as_they_were() {
    let d = XenonEffects::default();
    assert_eq!((d.contrail_speed, d.contrail_min, d.contrail_max), (44.0, 0.1, 0.75));
    assert_eq!((d.spark_intensity, d.contrail_rate, d.spark_rate), (1.0, 60.0, 60.0));
    assert_eq!((d.max_sparks, d.max_trails), (2048, 256));
    assert!(!d.nos_contrails && !d.nis_contrails && !d.shadows);
    assert_eq!(Settings::from(Partial::default()).xenon_effects, d);
}

#[test]
fn the_trail_curve_rises_from_the_minimum_at_the_speed_to_the_maximum_at_twice_it() {
    let d = XenonEffects::default();
    assert_eq!(d.trail_strength(43.9, false), None);
    assert_eq!(d.trail_strength(44.0, false), Some(0.1));
    assert!((d.trail_strength(66.0, false).unwrap() - 0.425).abs() < 1e-6);
    assert_eq!(d.trail_strength(88.0, false), Some(0.75));
    assert_eq!(d.trail_strength(300.0, false), Some(0.75), "capped");
    let fast = XenonEffects::from_layer(XenonEffectsLayer { contrail_speed: Some(20.0), ..Default::default() });
    assert_eq!(fast.trail_strength(20.0, false), Some(0.1));
    assert_eq!(fast.trail_strength(40.0, false), Some(0.75));
}

#[test]
fn nitrous_shows_the_trail_at_full_strength_only_when_asked_for() {
    let off = XenonEffects::default();
    assert_eq!(off.trail_strength(10.0, true), None, "the nitrous alone does not bypass the speed");
    let on = XenonEffects::from_layer(XenonEffectsLayer { nos_contrails: Some(true), ..Default::default() });
    assert_eq!(on.trail_strength(10.0, true), Some(0.75), "no fade-in");
    assert_eq!(on.trail_strength(10.0, false), None, "without the nitrous the speed rule holds");
    assert_eq!(on.trail_strength(60.0, false), off.trail_strength(60.0, false));
}

#[test]
fn the_config_table_is_read_and_bad_values_are_left_unset() {
    let layer = read(
        "[xenon_effects]\ncontrail_speed = 30\ncontrail_min_intensity = 0.2\ncontrail_max_intensity = 0.9\n\
         spark_intensity = 2.0\nnos_contrails = true\nnis_contrails = true\ncontrail_rate = 30\nspark_rate = 120.5\n\
         max_sparks = 4000\nmax_trails = 512\nshadows = true\n",
    );
    assert_eq!(
        layer,
        XenonEffectsLayer {
            contrail_speed: Some(30.0),
            contrail_min: Some(0.2),
            contrail_max: Some(0.9),
            spark_intensity: Some(2.0),
            nos_contrails: Some(true),
            nis_contrails: Some(true),
            contrail_rate: Some(30.0),
            spark_rate: Some(120.5),
            max_sparks: Some(4000),
            max_trails: Some(512),
            shadows: Some(true),
        }
    );
    let bad =
        read("[xenon_effects]\ncontrail_speed = \"fast\"\nnos_contrails = 1\nmax_sparks = -3\nmax_trails = 2.5\n");
    assert_eq!(bad, XenonEffectsLayer::default());
    assert_eq!(read("vsync = true\n"), XenonEffectsLayer::default());
}

#[test]
fn numbers_outside_their_range_are_clamped_and_the_maximum_never_falls_below_the_minimum() {
    let s = XenonEffects::from_layer(XenonEffectsLayer {
        contrail_speed: Some(0.0),
        contrail_min: Some(0.6),
        contrail_max: Some(0.2),
        spark_intensity: Some(50.0),
        contrail_rate: Some(0.0),
        spark_rate: Some(9999.0),
        max_sparks: Some(0),
        max_trails: Some(1_000_000),
        ..Default::default()
    });
    assert_eq!((s.contrail_speed, s.contrail_min, s.contrail_max), (1.0, 0.6, 0.6));
    assert_eq!((s.spark_intensity, s.contrail_rate, s.spark_rate), (4.0, 1.0, 240.0));
    assert_eq!((s.max_sparks, s.max_trails), (16, 10_000));
}

#[test]
fn a_higher_layer_wins_and_the_table_reaches_the_world_tuning() {
    let high = XenonEffectsLayer { shadows: Some(true), ..Default::default() };
    let low = XenonEffectsLayer { shadows: Some(false), nis_contrails: Some(true), ..Default::default() };
    let merged = high.or(low);
    assert_eq!((merged.shadows, merged.nis_contrails), (Some(true), Some(true)));
    let settings = Settings::from(Partial { xenon_effects: merged, ..Default::default() });
    assert!(super::WorldTuning::from(&settings).xenon_effects.nis_contrails);
}
