//! A car against a wall: the collision spark variant plays it from the profile's gates (the 2012 numbers), not the install's.

use std::sync::Arc;

use blackbox_attrib::vlt_hash;
use blackbox_gfx::{BlendMode, TextureHandle};
use blackbox_modding::content::parse_sparks_profile;
use glam::Vec3;
use nfsmw_data::vehicle_effects::{PC_EMITTERS, PcSparkLink};

use super::super::{Loaded, SparkVariantFx};
use super::*;
use crate::scenes::world::space;

const STEP: f32 = 1.0 / 60.0;
/// The camera, in render space, near the contacts.
const NEAR: Vec3 = Vec3::new(4.0, -2.0, 3.0);

/// The 2012 gates (impact 0.153 + 0.847 * min(v / 22.1, 1), scrape from 0.73 to 1.37 m/s) with one streak in each channel.
const PROFILE: &str = r#"
format = 1
[hit]
closing_speed_full = 22.1
intensity_floor = 0.153
[[hit.layer]]
kind = "streak"
burst = 79.0
life = 0.85
speed = 8.0
[scrape]
speed_start = 0.73
speed_full = 1.37
[[scrape.layer]]
kind = "streak"
rate = 196.0
life = 1.0
speed = 4.0
"#;

fn loaded() -> SparkVariantFx {
    let profile = Arc::new(parse_sparks_profile(PROFILE).unwrap());
    let mut fx = SparkVariantFx::default();
    fx.set_wanted(Some(profile.clone()));
    let layers = profile.hit.items.len() + profile.scrape.items.len();
    let pictures = vec![Some((TextureHandle::from_raw(3), BlendMode::Additive)); layers];
    fx.loaded = Some(Loaded { profile, pictures, owned: Vec::new() });
    fx
}

/// The install's links of a car: a world hit from 1 and a world scrape from 5 m/s, a particle group on every surface.
fn link() -> Link {
    let particles =
        PcSparkLink { min: 1.0, max: 30.0, emitters: [None; PC_EMITTERS], quadratic: [0.0; 4], inherit_velocity: 1.0 };
    Link { effect: 1, min: 1.0, max: 30.0, visual_cull: 100.0, particles: Some(particles) }
}

fn install() -> SourceEffects {
    let default = vlt_hash("default");
    SourceEffects::default()
        .with_link(Action::HitWorld, default, Some(link()))
        .with_link(Action::ScrapeWorld, default, Some(Link { min: 5.0, ..link() }))
        .with_link(Action::HitObject, default, Some(link()))
        .with_link(Action::ScrapeObject, default, Some(Link { min: 5.0, ..link() }))
}

/// A wall facing +x (physics): the car goes into it at `closing` m/s and along it (z) at `sliding` m/s.
fn wall(kind: Kind, closing: f32, sliding: f32) -> RawContact {
    let relative = -Vec3::X * closing + Vec3::Z * sliding;
    RawContact::new(kind, 7, Vec3::new(2.0, 3.0, 4.0), Vec3::X, relative, 5.0, Vec3::ZERO)
}

fn actions(kind: Kind) -> (Action, Action) {
    match kind {
        Kind::Object => (Action::HitObject, Action::ScrapeObject),
        _ => (Action::HitWorld, Action::ScrapeWorld),
    }
}

/// One physics step of the host for a contact: the triggers, the play and the end of the step. Returns what was taken.
fn step(fx: &mut SparkVariantFx, c: Option<&RawContact>) -> (bool, bool) {
    let mut taken = (false, false);
    if let Some(c) = c {
        let (hit, scrape) = culls(&install(), actions(c.kind), c, (None, None));
        taken = fx.play(1, c, hit, scrape, NEAR);
    }
    fx.finish(STEP);
    taken
}

#[test]
fn a_wall_hit_at_20_m_s_throws_the_2012_burst_along_the_wall_normal() {
    let mut fx = loaded();
    // The wall does not slide, so the scrape channel is taken and plays nothing: the install's scrape group stays off too.
    assert_eq!(step(&mut fx, Some(&wall(Kind::World, 20.0, 0.0))), (true, true));
    // 0.153 + 0.847 * 20 / 22.1 = 0.919 of 79 sparks.
    assert!((72..=74).contains(&fx.len()), "{}", fx.len());
    let out = space::to_render([1.0, 0.0, 0.0]);
    for spark in &fx.sparks {
        assert!(spark.velocity.normalize().abs_diff_eq(out, 1e-3), "{:?}", spark.velocity);
        assert!(spark.velocity.length() <= 8.0 * 0.92 + 1e-3, "scaled by the strength");
        assert!(spark.point.abs_diff_eq(space::to_render([2.0, 3.0, 4.0]) + out * 0.02, 1e-4));
    }
}

#[test]
fn a_wall_hit_the_install_would_not_spark_is_played_and_a_tap_below_1_m_s_is_not() {
    let c = wall(Kind::World, 3.0, 0.0);
    let ramp = link().hit(c.closing.length_squared(), 0.5);
    assert_eq!(ramp, None, "the install needs a reaction of 1 m/s");
    let mut fx = loaded();
    assert_eq!(step(&mut fx, Some(&c)), (true, true));
    assert!(fx.len() > 20, "{}", fx.len());
    let mut tap = loaded();
    assert_eq!(step(&mut tap, Some(&wall(Kind::World, 0.9, 0.0))), (false, true));
    assert_eq!(tap.len(), 0);
}

#[test]
fn a_wall_scrape_at_0_5_m_s_throws_nothing() {
    let mut fx = loaded();
    let c = wall(Kind::World, 0.0, 0.5);
    for _ in 0..120 {
        // The profile has the scrape channel, so the install's group stays off, and there is no strength.
        assert_eq!(step(&mut fx, Some(&c)), (false, true));
    }
    assert_eq!(fx.len(), 0);
}

#[test]
fn a_wall_scrape_from_1_4_m_s_throws_the_full_rate_although_the_install_starts_at_5() {
    assert_eq!(Link { min: 5.0, ..link() }.scrape(2.0), None, "the install's scrape starts at 5 m/s");
    let c = wall(Kind::World, 0.0, 2.0);
    let mut fx = loaded();
    for _ in 0..30 {
        step(&mut fx, Some(&c));
    }
    // 196 a second for half a second, all alive (a life of a second).
    assert!((90..=105).contains(&fx.len()), "{}", fx.len());
    // Between 0.73 and 1.37 m/s the rate is part of the full one.
    let mut slow = loaded();
    for _ in 0..60 {
        step(&mut slow, Some(&wall(Kind::World, 0.0, 1.05)));
    }
    assert!((90..=105).contains(&slow.len()), "half of 196 a second: {}", slow.len());
}

#[test]
fn a_car_pressed_to_a_wall_keeps_scraping_across_the_steps_the_wall_does_not_react() {
    let c = wall(Kind::World, 0.0, 3.0);
    let mut fx = loaded();
    step(&mut fx, Some(&c));
    for _ in 0..4 {
        step(&mut fx, None);
    }
    assert_eq!(fx.scrapers.len(), 1, "still scraping from its last contact");
    let before = fx.len();
    step(&mut fx, None);
    assert!(fx.len() > before);
    for _ in 0..10 {
        step(&mut fx, None);
    }
    assert!(fx.scrapers.is_empty(), "a car that left the wall stops");
}

#[test]
fn a_prop_is_played_like_a_wall_and_the_ground_keeps_the_installs_acceptance() {
    let mut fx = loaded();
    assert_eq!(step(&mut fx, Some(&wall(Kind::Object, 10.0, 0.0))), (true, true));
    assert!(fx.len() > 0);
    let ground = wall(Kind::Ground, 10.0, 3.0);
    assert_eq!(culls(&install(), (Action::HitGround, Action::ScrapeGround), &ground, (None, None)), (None, None));
    assert_eq!(
        culls(&install(), (Action::HitGround, Action::ScrapeGround), &ground, (Some(link()), Some(link()))),
        (Some(100.0), Some(100.0))
    );
    let bottom = RawContact { bottom: true, ..wall(Kind::World, 10.0, 3.0) };
    assert_eq!(culls(&install(), (Action::BottomOut, Action::BottomScrape), &bottom, (None, None)), (None, None));
}

#[test]
fn a_surface_without_a_particle_group_is_not_played() {
    let bare = Link { particles: None, ..link() };
    let install = SourceEffects::default().with_link(Action::HitWorld, vlt_hash("default"), Some(bare));
    let c = wall(Kind::World, 20.0, 3.0);
    assert_eq!(culls(&install, (Action::HitWorld, Action::ScrapeWorld), &c, (None, None)), (None, None));
}
