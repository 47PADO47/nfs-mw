//! The `[xenon_effects]` options on the trail and the sparks (`settings/xenon_effects.rs`).

use super::*;
use crate::settings::xenon_effects::XenonEffectsLayer;
use glam::Quat;
use nfsmw_data::vehicle_effects::EmitterStyle;

const STEP: f32 = 1.0 / 60.0;

fn tuned(layer: XenonEffectsLayer) -> XenonEffects {
    XenonEffects::from_layer(layer)
}

fn style() -> EmitterStyle {
    EmitterStyle {
        color: [0.8, 1.0, 0.8, 0.15],
        life: 0.25,
        count: 15.0,
        velocity: [0.0, 0.0, -1.0],
        length: 200.0,
        height: 255.0,
        ..EmitterStyle::default()
    }
}

fn pose() -> CarPose {
    CarPose {
        position: Vec3::new(5.0, 7.0, 2.0),
        rotation: Quat::IDENTITY,
        wheels: [nfsmw_data::car::WheelPose::default(); 4],
    }
}

fn effects(tuning: XenonEffects) -> VehicleEffects {
    let mut effects = VehicleEffects::new(VisualEffectsData { trail: Some(style()), ..VisualEffectsData::default() });
    effects.set_enabled(false, true);
    effects.set_tuning(tuning);
    effects
}

#[test]
fn the_nitrous_trail_shows_below_the_speed_only_with_the_option_and_at_full_strength() {
    let mut off = effects(XenonEffects::default());
    off.step(&[], pose(), Vec3::X * 20.0, true, STEP);
    assert_eq!(off.trails.len(), 0);
    let mut on = effects(tuned(XenonEffectsLayer { nos_contrails: Some(true), ..Default::default() }));
    on.step(&[], pose(), Vec3::X * 20.0, false, STEP);
    assert_eq!(on.trails.len(), 0, "slow and without the nitrous nothing shows");
    on.step(&[], pose(), Vec3::X * 20.0, true, STEP);
    assert_eq!(on.trails.len(), 15);
    assert_eq!(streak_strength(20.0, true, &on.tuning), 0.75, "the view streaks follow");
    assert_eq!(streak_strength(20.0, true, &off.tuning), 0.0);
}

#[test]
fn the_trail_speed_and_rate_follow_the_options() {
    let mut e = effects(tuned(XenonEffectsLayer { contrail_speed: Some(20.0), ..Default::default() }));
    e.step(&[], pose(), Vec3::X * 25.0, false, STEP);
    assert_eq!(e.trails.len(), 15, "25 m/s is over a 20 m/s threshold");
    let mut slow = effects(tuned(XenonEffectsLayer { contrail_rate: Some(30.0), ..Default::default() }));
    for _ in 0..60 {
        slow.step(&[], pose(), Vec3::X * 60.0, false, STEP);
    }
    assert_eq!(slow.trails.emitted, 30 * 15, "30 dispatches in a second");
}

#[test]
fn the_particle_caps_follow_the_options_and_a_lowered_cap_trims_at_once() {
    let mut e = effects(tuned(XenonEffectsLayer { max_trails: Some(40), ..Default::default() }));
    for _ in 0..30 {
        e.step(&[], pose(), Vec3::X * 60.0, false, STEP);
    }
    assert_eq!(e.trails.len(), 40);
    e.set_tuning(tuned(XenonEffectsLayer { max_trails: Some(16), ..Default::default() }));
    assert_eq!(e.trails.len(), 16);
}

#[test]
fn a_cutscene_trail_needs_the_option_and_the_trail_switch_and_ages_with_the_scene_clock() {
    let mut off = effects(XenonEffects::default());
    off.cutscene_step(pose(), Vec3::X * 60.0, STEP);
    assert_eq!(off.trails.len(), 0);
    let mut on = effects(tuned(XenonEffectsLayer { nis_contrails: Some(true), ..Default::default() }));
    on.cutscene_step(pose(), Vec3::X * 60.0, STEP);
    assert_eq!(on.trails.len(), 15);
    on.cutscene_step(pose(), Vec3::X * 5.0, 1.0);
    assert_eq!(on.trails.len(), 0, "a slow car in a long frame lets the old trail expire");
    on.set_enabled(false, false);
    on.cutscene_step(pose(), Vec3::X * 60.0, STEP);
    assert_eq!(on.trails.len(), 0, "the speed trails setting is off");
    on.set_enabled(false, true);
    for (velocity, dt) in [(Vec3::NAN, STEP), (Vec3::X * 60.0, 0.0), (Vec3::X * 60.0, f32::NAN)] {
        on.cutscene_step(pose(), velocity, dt);
    }
    assert_eq!(on.trails.len(), 0);
}
