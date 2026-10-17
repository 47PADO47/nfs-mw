use blackbox_gfx::reflection::{MirrorTint, ReflectionSize};

use super::reflection_res::{MENU_SIZES, size_label, step_size, step_tint, tint_label};
use super::{Partial, Settings, env, file, write};

#[test]
fn the_defaults_keep_the_port_look() {
    let s = Settings::from(Partial::default());
    assert_eq!(s.reflection_cube_size, ReflectionSize::Scale(1.0));
    assert_eq!(s.reflection_mirror_size, ReflectionSize::Scale(1.0));
    assert!(!s.reflection_pow2);
    assert_eq!(s.mirror_tint, MirrorTint::NONE);
    assert_eq!(s.graphics().reflection_resolution, Default::default());
    assert_eq!(s.graphics().mirror_tint, MirrorTint::NONE);
}

#[test]
fn the_menu_steps_wrap_and_a_size_outside_the_list_steps_from_the_start() {
    let mut size = MENU_SIZES[0];
    for _ in 0..MENU_SIZES.len() {
        size = step_size(size, true);
    }
    assert_eq!(size, MENU_SIZES[0]);
    assert_eq!(step_size(MENU_SIZES[0], false), MENU_SIZES[MENU_SIZES.len() - 1]);
    assert_eq!(step_size(ReflectionSize::Fixed(256), true), MENU_SIZES[1]);
    assert_eq!(step_tint(MirrorTint::NONE), MirrorTint::STOCK);
    assert_eq!(step_tint(MirrorTint::STOCK), MirrorTint::NONE);
    assert_eq!(step_tint(MirrorTint([1, 2, 3])), MirrorTint::NONE);
}

#[test]
fn labels_show_auto_percent_pixels_and_the_tint_name() {
    assert_eq!(size_label(ReflectionSize::Auto), "Auto");
    assert_eq!(size_label(ReflectionSize::Scale(1.5)), "150%");
    assert_eq!(size_label(ReflectionSize::Fixed(256)), "256 px");
    let names = [MirrorTint::NONE, MirrorTint::STOCK, MirrorTint([1, 2, 3])].map(tint_label);
    assert_eq!(names, ["Off", "Stock", "Custom"]);
}

#[test]
fn the_config_file_round_trips_and_a_higher_layer_wins() {
    let changes = Partial {
        reflection_cube_size: Some(ReflectionSize::Scale(1.5)),
        reflection_mirror_size: Some(ReflectionSize::Fixed(200)),
        reflection_pow2: Some(true),
        mirror_tint: Some(MirrorTint([10, 20, 30])),
        ..Partial::default()
    };
    let text = write::merge("future_option = 42", &changes).unwrap();
    assert!(text.contains("future_option = 42"));
    let saved = file::parse(&text, "test");
    assert_eq!(saved, changes);
    let cli = Partial { reflection_pow2: Some(false), ..Partial::default() };
    let s = Settings::from(cli.or(saved));
    assert_eq!((s.reflection_pow2, s.reflection_cube_size), (false, ReflectionSize::Scale(1.5)));
}

#[test]
fn the_file_takes_whole_pixels_and_channel_arrays_and_drops_bad_values() {
    let saved = file::parse(
        "reflection_cube_size = 256\nreflection_mirror_size = 'auto'\nmirror_tint = [1, 2, 3]\nreflection_pow2 = true",
        "test",
    );
    let expected = Partial {
        reflection_cube_size: Some(ReflectionSize::Fixed(256)),
        reflection_mirror_size: Some(ReflectionSize::Auto),
        mirror_tint: Some(MirrorTint([1, 2, 3])),
        reflection_pow2: Some(true),
        ..Partial::default()
    };
    assert_eq!(saved, expected);
    let bad = file::parse("reflection_cube_size = '10%'\nmirror_tint = [1, 2, 300]", "test");
    assert_eq!(bad, Partial::default());
}

#[test]
fn the_environment_overrides_the_file_and_a_bad_value_falls_through() {
    let env = env::read(|n| match n {
        env::REFLECTION_CUBE_SIZE => Some("50%".into()),
        env::REFLECTION_MIRROR_SIZE => Some("bogus".into()),
        env::MIRROR_TINT => Some("on".into()),
        _ => None,
    });
    let file = file::parse("reflection_cube_size = 256\nreflection_mirror_size = 'auto'", "test");
    let s = Settings::from(env.or(file));
    assert_eq!(s.reflection_cube_size, ReflectionSize::Scale(0.5));
    assert_eq!(s.reflection_mirror_size, ReflectionSize::Auto, "a bad environment value falls through");
    assert_eq!(s.mirror_tint, MirrorTint::STOCK);
}
