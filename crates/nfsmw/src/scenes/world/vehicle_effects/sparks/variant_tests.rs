use blackbox_modding::content::parse_xenon_profile;

use super::*;
use crate::scenes::world::drive::ContactKind;

const STEP: f32 = 1.0 / 60.0;

/// The profile: an impact spark of 100 at full strength (full at 20 m/s, 0.2 at rest) that flies at 5 m/s along the normal, and a
/// scrape spark of 120 a second (from 1 to 3 m/s), also at 4 m/s.
fn profile(extra_hit: &str) -> Arc<XenonProfile> {
    let text = format!(
        "format = 1
[hit]
closing_speed_full = 20.0
intensity_floor = 0.2
[[hit.spark]]
color = [1.0, 0.5, 0.25, 1.0]
life = 0.5
count = 100.0
velocity = [0.0, 0.0, 5.0]
{extra_hit}
[scrape]
speed_start = 1.0
speed_full = 3.0
[[scrape.spark]]
color = [0.5, 0.5, 1.0, 1.0]
life = 0.5
count = 120.0
velocity = [0.0, 0.0, 4.0]
"
    );
    Arc::new(parse_xenon_profile(&text).unwrap())
}

/// The install's link: 30 sparks, flying 1 m/s along the normal.
fn link() -> SparkLink {
    let style = EmitterStyle {
        color: [1.0; 4],
        life: 0.4,
        count: 30.0,
        velocity: [0.0, 0.0, 1.0],
        length: 255.0,
        height: 62.0,
        ..EmitterStyle::default()
    };
    SparkLink { min: 1.0, max: 10.0, inherit_velocity: 1.0, styles: [Some(style), None] }
}

fn contact(velocity: Vec3, impulse: f32) -> VisualContact {
    VisualContact {
        point: Vec3::new(2.0, 3.0, 4.0),
        normal: Vec3::NEG_Z,
        velocity,
        impulse_delta_v: impulse,
        surface: Some(7),
        kind: ContactKind::Face,
        info: None,
        prop: None,
    }
}

fn with(profile: Option<Arc<XenonProfile>>) -> Sparks {
    let mut sparks = Sparks::default();
    sparks.set_variant(profile);
    sparks
}

fn emit(sparks: &mut Sparks, c: VisualContact) {
    sparks.emit_selected(&[c], |_, _| Some(link()), &XenonEffects::default(), STEP);
}

#[test]
fn a_spark_of_the_profile_is_the_restored_emitter_with_the_same_numbers() {
    let profile =
        profile("elasticity = 217.0\ngravity = -4.9\nlength = 200.0\nheight = 59.0\nextent = [0.5, 0.5, 0.0]");
    let style = style_of(&profile.hit.items[0]);
    assert_eq!((style.color, style.life, style.count), ([1.0, 0.5, 0.25, 1.0], 0.5, 100.0));
    assert_eq!((style.velocity, style.extent), ([0.0, 0.0, 5.0], [0.5, 0.5, 0.0]));
    assert_eq!((style.gravity, style.length, style.height), (-4.9, 200.0, 59.0));
    let played = hit_played(&profile, 10.0).unwrap();
    assert_eq!(played.elasticity, [Some(217.0 / 255.0), None]);
    assert!(played.styles[1].is_none());
    assert_eq!(played.link().inherit_velocity, 1.0, "the owner's velocity is shared by the profile's own inherit");
}

#[test]
fn without_a_variant_the_installs_intensity_and_sparks_are_used() {
    let (picked, strength, played) = hit(None, link(), 10.0, 5.0);
    assert_eq!(picked, link());
    assert_eq!(strength, intensity(link(), 5.0, false));
    assert!(played.is_none());
    let (_, scrape_strength, played) = scrape(None, link(), 0.5);
    assert_eq!(scrape_strength, intensity(link(), 0.5, true));
    assert!(played.is_none());
}

#[test]
fn an_impact_throws_count_times_strength_particles_as_fast_as_it_is_strong() {
    let mut sparks = with(Some(profile("")));
    // 10 m/s of 20: 0.2 + 0.8 * 0.5 = 0.6 of the 100 sparks, at 0.6 of 5 m/s along the normal.
    emit(&mut sparks, contact(Vec3::Z * 10.0, 10.0));
    assert_eq!(sparks.emitted, 60);
    assert_eq!(sparks.len(), 60);
    let normal = space::to_render([0.0, 0.0, -1.0]);
    for p in &sparks.particles {
        assert!(p.velocity.abs_diff_eq(normal * 3.0, 1e-4), "{:?}", p.velocity);
    }
    assert_eq!(sparks.flashes.len(), 1);
    assert_eq!(sparks.flashes[0].color, [1.0, 0.5, 0.25, 1.0], "the flash takes the profile's colour");
    assert!((sparks.flashes[0].strength - 0.6).abs() < 1e-6);
}

#[test]
fn an_impact_at_full_closing_speed_throws_them_all_and_a_slow_one_the_floor() {
    let mut fast = with(Some(profile("")));
    emit(&mut fast, contact(Vec3::Z * 40.0, 10.0));
    assert_eq!(fast.emitted, 100);
    let mut slow = with(Some(profile("")));
    emit(&mut slow, contact(Vec3::Z * 1.5, 10.0));
    // 0.2 + 0.8 * 0.075 = 0.26.
    assert_eq!(slow.emitted, 26);
}

#[test]
fn the_installs_triggers_stay() {
    let mut sparks = with(Some(profile("")));
    emit(&mut sparks, contact(Vec3::ZERO, 10.0));
    assert_eq!(sparks.emitted, 0, "a stationary correction is not an impact");
    emit(&mut sparks, contact(Vec3::Z * 10.0, 0.0));
    assert_eq!(sparks.emitted, 0, "an impact needs a reaction");
    sparks.emit_selected(&[contact(Vec3::Z * 10.0, 10.0)], |_, _| None, &XenonEffects::default(), STEP);
    assert_eq!(sparks.emitted, 0, "a surface with no spark in the install has none");
}

#[test]
fn a_scrape_throws_its_rate_times_strength_a_second_at_the_full_speed() {
    let mut sparks = with(Some(profile("")));
    // 2 m/s of 1 to 3: half of 120 a second.
    for _ in 0..60 {
        emit(&mut sparks, contact(Vec3::X * 2.0, 0.0));
    }
    assert!((59..=61).contains(&sparks.emitted), "{}", sparks.emitted);
    assert!(
        sparks.particles.iter().all(|p| (p.velocity.length() - 4.0).abs() < 1e-3),
        "a scrape does not scale the speed"
    );
    assert!(sparks.scrape_flash.is_some());
    assert!(sparks.flashes.is_empty());
}

#[test]
fn a_scrape_below_the_start_speed_throws_nothing() {
    let late = "format = 1\n[scrape]\nspeed_start = 2.0\nspeed_full = 4.0\n[[scrape.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 60.0";
    let mut sparks = with(Some(Arc::new(parse_xenon_profile(late).unwrap())));
    for _ in 0..30 {
        // The install would scrape at 1.5 m/s (its trigger and its minimum of 1); the profile starts at 2.
        emit(&mut sparks, contact(Vec3::X * 1.5, 0.0));
    }
    assert_eq!(sparks.emitted, 0);
    assert!(sparks.scrape_flash.is_none());
    emit(&mut sparks, contact(Vec3::X * 3.0, 0.0));
    assert_eq!(sparks.emitted, 0, "half strength of 60 a second is under one particle in a step");
    for _ in 0..60 {
        emit(&mut sparks, contact(Vec3::X * 3.0, 0.0));
    }
    assert!((29..=31).contains(&sparks.emitted), "{}", sparks.emitted);
}

#[test]
fn a_channel_the_profile_has_no_spark_for_keeps_the_installs() {
    let scrape_only = "format = 1\n[scrape]\nspeed_start = 1.0\nspeed_full = 3.0\n[[scrape.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 60.0";
    let mut sparks = with(Some(Arc::new(parse_xenon_profile(scrape_only).unwrap())));
    emit(&mut sparks, contact(Vec3::Z * 10.0, 10.0));
    assert_eq!(sparks.emitted, 30, "the install's impact: its count, not scaled");
    assert!(sparks.particles.iter().all(|p| (p.velocity.length() - 1.0).abs() < 1e-3));
}

#[test]
fn the_elasticity_is_the_profiles_where_it_has_one_and_the_styles_own_where_not() {
    let mut own = with(Some(profile("")));
    emit(&mut own, contact(Vec3::Z * 10.0, 10.0));
    assert!(own.particles.iter().all(|p| (p.elasticity - 160.0 / 255.0).abs() < 1e-6));
    let mut given = with(Some(profile("elasticity = 217.0")));
    emit(&mut given, contact(Vec3::Z * 10.0, 10.0));
    assert!(given.particles.iter().all(|p| (p.elasticity - 217.0 / 255.0).abs() < 1e-6));
}

#[test]
fn the_particle_cap_holds_and_the_boost_of_the_tuning_adds_particles() {
    let mut sparks = with(Some(profile("")));
    let tuning = XenonEffects { max_sparks: 16, ..XenonEffects::default() };
    sparks.emit_selected(&[contact(Vec3::Z * 40.0, 10.0)], |_, _| Some(link()), &tuning, STEP);
    assert_eq!(sparks.len(), 16, "the oldest go");
    assert_eq!(sparks.emitted, 100);
    let mut boosted = with(Some(profile("")));
    let tuning = XenonEffects { spark_intensity: 2.0, ..XenonEffects::default() };
    boosted.emit_selected(&[contact(Vec3::Z * 40.0, 10.0)], |_, _| Some(link()), &tuning, STEP);
    assert_eq!(boosted.emitted, 200);
}

#[test]
fn a_new_profile_starts_clean_and_the_same_one_changes_nothing() {
    let mut sparks = with(Some(profile("")));
    sparks.variant_carry = [0.5, 0.5];
    let same = sparks.variant.clone();
    sparks.set_variant(same);
    assert_eq!(sparks.variant_carry, [0.5, 0.5]);
    sparks.set_variant(Some(profile("")));
    assert_eq!(sparks.variant_carry, [0.0; 2]);
    sparks.set_variant(None);
    assert!(sparks.variant.is_none());
    emit(&mut sparks, contact(Vec3::Z * 10.0, 10.0));
    assert_eq!(sparks.emitted, 30, "without a variant the install's sparks play");
}
