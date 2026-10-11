use super::*;
use crate::cli::{Cli, Command};
use blackbox_gfx::{Antialiasing, PostSettings, Tonemap};
use clap::Parser;

fn view_layer(flags: &[&str]) -> Partial {
    let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
    let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
    view.settings_layer()
}

#[test]
fn every_effect_is_off_by_default_and_asks_the_renderer_for_nothing() {
    let defaults = Settings::from(Partial::default());
    assert_eq!(
        (defaults.post_tonemap, defaults.post_bloom, defaults.post_aa),
        (PostTonemap::Off, PostBloom::Off, PostAa::Off)
    );
    assert_eq!(post_effects(&defaults), PostSettings::default());
    assert!(post_effects(&defaults).effects().is_empty());
}

#[test]
fn names_round_trip_and_anything_else_is_rejected() {
    for text in ["off", "aces"] {
        assert_eq!(text.parse::<PostTonemap>().unwrap().to_string(), text);
    }
    for text in ["off", "low", "medium", "high"] {
        assert_eq!(text.parse::<PostBloom>().unwrap().to_string(), text);
    }
    for text in ["off", "fxaa", "smaa", "taa"] {
        assert_eq!(text.parse::<PostAa>().unwrap().to_string(), text);
    }
    for text in ["", "ACES", " aces", "on", "reinhard", "1"] {
        assert!(text.parse::<PostTonemap>().is_err(), "{text:?}");
    }
    for text in ["ultra", "HIGH", "on", "0.5"] {
        assert!(text.parse::<PostBloom>().is_err(), "{text:?}");
    }
    for text in ["msaa", "FXAA", "TAA", "on", " taa"] {
        assert!(text.parse::<PostAa>().is_err(), "{text:?}");
    }
}

#[test]
fn bloom_steps_grow_and_off_is_zero() {
    let strengths = [PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High].map(PostBloom::intensity);
    assert_eq!(strengths[0], 0.0);
    assert!(strengths.windows(2).all(|w| w[0] < w[1]), "{strengths:?}");
    assert!(strengths[3] <= blackbox_gfx::MAX_BLOOM_INTENSITY);
}

#[test]
fn the_settings_map_to_the_renderer_effects() {
    let all = Settings::from(Partial {
        post_tonemap: Some(PostTonemap::Aces),
        post_bloom: Some(PostBloom::Medium),
        post_aa: Some(PostAa::Fxaa),
        ..Partial::default()
    });
    let effects = post_effects(&all);
    assert_eq!((effects.tonemap, effects.antialiasing), (Tonemap::Aces, Antialiasing::Fxaa));
    assert_eq!(effects.bloom_intensity, PostBloom::Medium.intensity());
    assert_eq!(effects.effects().len(), 3);
    let only_aa = Settings::from(Partial { post_aa: Some(PostAa::Fxaa), ..Partial::default() });
    assert_eq!(post_effects(&only_aa).effects(), [blackbox_gfx::PostEffect::Fxaa]);
}

#[test]
fn layers_resolve_in_order_independently_and_bad_values_fall_through() {
    let file = file::parse("post_tonemap = 'aces'\npost_bloom = 'low'\npost_aa = 'fxaa'", "test");
    let environment = env::read(|key| match key {
        env::POST_BLOOM => Some("high".into()),
        env::POST_AA => Some("msaa".into()),
        _ => None,
    });
    let resolved = Settings::from(environment.or(file));
    assert_eq!(resolved.post_bloom, PostBloom::High, "environment over file");
    assert_eq!(resolved.post_aa, PostAa::Fxaa, "a bad environment value falls through to the file");
    assert_eq!(resolved.post_tonemap, PostTonemap::Aces);
    let cli = view_layer(&["--post-tonemap", "off", "--post-bloom", "medium"]);
    let resolved = Settings::from(cli.or(environment).or(file));
    assert_eq!(
        (resolved.post_tonemap, resolved.post_bloom, resolved.post_aa),
        (PostTonemap::Off, PostBloom::Medium, PostAa::Fxaa)
    );
    for text in ["post_tonemap = 'reinhard'", "post_tonemap = true", "post_bloom = 3", "post_aa = 'msaa'"] {
        assert_eq!(file::parse(text, "test"), Partial::default(), "{text}");
    }
}

#[test]
fn the_command_line_is_optional_and_strict() {
    assert_eq!(view_layer(&[]).post_aa, None);
    assert_eq!(view_layer(&["--post-aa", "fxaa"]).post_aa, Some(PostAa::Fxaa));
    assert_eq!(view_layer(&["--post-aa", "off"]).post_aa, Some(PostAa::Off));
    assert_eq!(view_layer(&["--post-aa", "taa"]).post_aa, Some(PostAa::Taa));
    assert_eq!(view_layer(&["--post-aa", "smaa"]).post_aa, Some(PostAa::Smaa));
    for flags in [["--post-aa", "msaa"], ["--post-bloom", "ultra"], ["--post-tonemap", "ACES"]] {
        assert!(Cli::try_parse_from(["nfsmw", "view-world", flags[0], flags[1]]).is_err(), "{flags:?}");
    }
}

#[test]
fn choices_round_trip_through_the_config_file_without_dropping_other_keys() {
    let changes = Partial {
        post_tonemap: Some(PostTonemap::Aces),
        post_bloom: Some(PostBloom::High),
        post_aa: Some(PostAa::Fxaa),
        ..Partial::default()
    };
    let text = write::merge("future_option = 5\nsmoke_quality = 'high'", &changes).unwrap();
    assert_eq!(file::parse(&text, "test").post_bloom, Some(PostBloom::High));
    assert_eq!(file::parse(&text, "test").smoke_quality, Some(SmokeQuality::High));
    let table = text.parse::<toml::Table>().unwrap();
    assert_eq!(table["future_option"].as_integer(), Some(5));
    assert_eq!(table["post_tonemap"].as_str(), Some("aces"));
    assert_eq!(table["post_aa"].as_str(), Some("fxaa"));
    assert_eq!(file::parse(&write::merge("", &changes).unwrap(), "test"), changes);
}
