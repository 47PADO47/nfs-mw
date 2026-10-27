use blackbox_modding::content::parse_sparks_profile;

use super::super::contact::Kind;
use super::*;

const STEP: f32 = 1.0 / 60.0;

/// An impact of 10 sparks (full at 20 m/s, 0.2 at rest) flying at 8 m/s, and a scrape of 60 a second (from 1 to 3 m/s).
const PROFILE: &str = r#"
format = 1
[hit]
closing_speed_full = 20.0
intensity_floor = 0.2
[[hit.layer]]
kind = "streak"
burst = 10.0
life = 1.0
speed = 8.0
[scrape]
speed_start = 1.0
speed_full = 3.0
[[scrape.layer]]
kind = "streak"
rate = 60.0
life = 1.0
speed = 8.0
"#;

/// A variant with the profile loaded without a renderer: every layer draws with the one picture, whose handle is made up.
fn loaded(text: &str) -> SparkVariantFx {
    let profile = Arc::new(parse_sparks_profile(text).unwrap());
    let mut fx = SparkVariantFx::default();
    fx.set_wanted(Some(profile.clone()));
    let layers = profile.hit.items.len() + profile.scrape.items.len();
    let pictures = vec![Some((TextureHandle::from_raw(3), BlendMode::Additive)); layers];
    fx.loaded = Some(Loaded { profile, pictures, owned: Vec::new() });
    fx
}

/// A contact on a surface facing up (physics y): it closes at `closing` m/s and slides at `sliding` m/s.
fn contact(closing: f32, sliding: f32) -> RawContact {
    let relative = -Vec3::Y * closing + Vec3::X * sliding;
    RawContact::new(Kind::World, 7, Vec3::new(2.0, 3.0, 4.0), Vec3::Y, relative, 5.0, Vec3::ZERO)
}

const NEAR: Vec3 = Vec3::new(4.0, -2.0, 3.0);

#[test]
fn nothing_plays_until_the_profile_is_loaded_and_the_install_accepts_the_contact() {
    let mut fx = SparkVariantFx::default();
    fx.set_wanted(Some(Arc::new(parse_sparks_profile(PROFILE).unwrap())));
    assert!(fx.needs_load());
    assert_eq!(
        fx.play(1, &contact(10.0, 0.0), Some(30.0), None, NEAR),
        (false, false),
        "the install's sparks until loaded"
    );
    let mut ready = loaded(PROFILE);
    assert!(!ready.needs_load());
    assert_eq!(ready.play(1, &contact(10.0, 0.0), None, None, NEAR), (false, false), "the install did not accept it");
    assert_eq!(ready.len(), 0);
}

#[test]
fn an_impact_throws_count_times_strength_sparks_at_strength_times_the_speed() {
    let mut fx = loaded(PROFILE);
    // 10 m/s of 20: 0.2 + 0.8 * 0.5 = 0.6.
    assert_eq!(fx.play(1, &contact(10.0, 0.0), Some(30.0), None, NEAR), (true, false));
    assert_eq!(fx.len(), 6);
    let up = space::to_render([0.0, 1.0, 0.0]);
    for spark in &fx.sparks {
        assert!(spark.velocity.abs_diff_eq(up * 4.8, 1e-4), "{:?}", spark.velocity);
        assert!(spark.point.abs_diff_eq(space::to_render([2.0, 3.0, 4.0]) + up * 0.02, 1e-5));
    }
    let mut fast = loaded(PROFILE);
    fast.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR);
    assert_eq!(fast.len(), 10, "full strength from 20 m/s");
}

#[test]
fn a_contact_beyond_the_links_cull_distance_is_taken_but_throws_nothing() {
    let mut fx = loaded(PROFILE);
    let far = Vec3::new(400.0, 0.0, 0.0);
    assert_eq!(
        fx.play(1, &contact(10.0, 0.0), Some(30.0), None, far),
        (true, false),
        "the install's group stays off too"
    );
    assert_eq!(fx.len(), 0);
}

#[test]
fn a_scrape_throws_rate_times_strength_a_second_and_keeps_the_speed() {
    let mut fx = loaded(PROFILE);
    // 2 m/s of 1 to 3: half of 60 a second.
    for _ in 0..60 {
        assert_eq!(fx.play(1, &contact(0.0, 2.0), None, Some(30.0), NEAR), (false, true));
        fx.finish(STEP);
    }
    assert!((29..=31).contains(&fx.len()), "{}", fx.len());
    assert!(
        fx.sparks.iter().all(|spark| (spark.velocity.length() - 8.0).abs() < 1e-3),
        "a scrape does not scale the speed"
    );
}

#[test]
fn a_car_scrapes_once_a_step_from_its_strongest_contact_and_every_car_has_its_own_clock() {
    let mut fx = loaded(PROFILE);
    // Strength 0.5 and 1.0 at one car: one set of sparks at 1.0 (60 a second, 2 in a step of 1/30), not the sum.
    fx.play(1, &contact(0.0, 2.0), None, Some(30.0), NEAR);
    fx.play(1, &contact(0.0, 3.0), None, Some(30.0), NEAR);
    fx.play(1, &contact(0.0, 2.5), None, Some(30.0), NEAR);
    fx.finish(1.0 / 30.0);
    assert!((1..=2).contains(&fx.len()), "{}", fx.len());
    let one_car = fx.len();
    fx.play(1, &contact(0.0, 3.0), None, Some(30.0), NEAR);
    fx.play(2, &contact(0.0, 3.0), None, Some(30.0), NEAR);
    fx.finish(1.0 / 30.0);
    assert!(fx.len() >= one_car + 3, "two cars throw twice as much: {} after {one_car}", fx.len());
}

#[test]
fn a_scrape_below_the_profiles_start_speed_throws_nothing_and_a_pause_resets_the_car() {
    let mut fx = loaded(PROFILE);
    assert_eq!(
        fx.play(1, &contact(0.0, 1.0), None, Some(30.0), NEAR),
        (false, true),
        "taken: the install's group stays off"
    );
    fx.finish(STEP);
    assert_eq!(fx.len(), 0, "1 m/s is the profile's start: strength 0");
    fx.play(1, &contact(0.0, 3.0), None, Some(30.0), NEAR);
    fx.finish(STEP);
    assert_eq!(fx.scrapers.len(), 1);
    fx.finish(STEP);
    assert!(fx.scrapers.is_empty(), "a car that did not scrape in a step is forgotten");
}

#[test]
fn a_channel_with_no_layer_is_not_taken() {
    let hit_only = "format = 1\n[hit]\nclosing_speed_full = 20.0\nintensity_floor = 0.2\n[[hit.layer]]\nkind = \"streak\"\nburst = 4.0";
    let mut fx = loaded(hit_only);
    assert_eq!(
        fx.play(1, &contact(10.0, 5.0), Some(30.0), Some(30.0), NEAR),
        (true, false),
        "the install's scrape stays"
    );
}

#[test]
fn the_spark_budget_holds() {
    let big = "format = 1\n[hit]\nclosing_speed_full = 20.0\nintensity_floor = 1.0\n[[hit.layer]]\nkind = \"streak\"\nburst = 1000.0";
    let mut fx = loaded(big);
    for _ in 0..3 {
        fx.play(1, &contact(30.0, 0.0), Some(30.0), None, NEAR);
    }
    assert_eq!(fx.len(), LIMIT);
}

#[test]
fn sparks_age_out_and_a_variant_that_went_leaves_none() {
    let mut fx = loaded(PROFILE);
    fx.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR);
    fx.age(0.5);
    assert_eq!(fx.len(), 10);
    fx.age(0.6);
    assert_eq!(fx.len(), 0, "a life of a second");
    fx.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR);
    fx.set_wanted(None);
    assert!(fx.needs_load(), "the variant was unchosen");
    assert_eq!(fx.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR), (false, false));
    fx.unload();
    assert_eq!(fx.len(), 0);
    assert!(fx.loaded.is_none());
}

#[test]
fn the_same_profile_is_not_loaded_again_and_a_changed_one_is() {
    let mut fx = loaded(PROFILE);
    let same = fx.wanted.clone();
    fx.set_wanted(same);
    assert!(!fx.needs_load());
    let equal = Arc::new(parse_sparks_profile(PROFILE).unwrap());
    fx.set_wanted(Some(equal));
    assert!(!fx.needs_load(), "an equal profile read again is the same");
    let other = Arc::new(parse_sparks_profile(&PROFILE.replace("burst = 10.0", "burst = 11.0")).unwrap());
    fx.set_wanted(Some(other));
    assert!(fx.needs_load());
    fx.set_wanted(Some(Arc::new(SparksProfile::default())));
    assert!(fx.wanted.is_none(), "a profile that plays nothing is no variant");
}

#[test]
fn the_sparks_are_drawn_in_one_batch_for_each_picture_and_a_layer_without_a_picture_draws_nothing() {
    let mut fx = loaded(PROFILE);
    fx.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR);
    let mut out = Vec::new();
    fx.textured(Vec3::X, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].texture, out[0].blend), (TextureHandle::from_raw(3), BlendMode::Additive));
    assert_eq!(out[0].vertices.len(), 60, "10 sparks, 6 vertices each");
    // A layer whose picture could not be read draws nothing.
    let mut none = loaded(PROFILE);
    none.loaded.as_mut().unwrap().pictures = vec![None, None];
    none.play(1, &contact(80.0, 0.0), Some(30.0), None, NEAR);
    let mut out = Vec::new();
    none.textured(Vec3::X, &mut out);
    assert!(out.is_empty());
}
