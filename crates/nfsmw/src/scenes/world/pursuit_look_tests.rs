//! The pursuit look's start rule and its three effects (`docs/specs/pursuit-look.md`).

use super::*;
use blackbox_gfx::ColourTreatment;

fn treatment(tint: f32) -> ColourTreatment {
    ColourTreatment {
        tint: [tint; 3],
        luminance: [0.6, 0.5, 0.1],
        desaturation: 0.5,
        black_intensity: 0.9,
        colour_intensity: 1.75,
        black_curve: [0.0, 0.0, 1.0, 1.0],
        colour_curve: [0.0, 0.2, 0.5, 0.4],
    }
}

/// A look whose blend holds full weight at its start and ends at 0 after 5 s, a 0.6 s flash and a 10 s blur.
fn fixture() -> PursuitLook {
    let effect = |length, magnitude, heat_trigger, graph| Effect { length, magnitude, heat_trigger, graph };
    PursuitLook::new(Profile {
        look: Some(treatment(1.0)),
        transition: Some(effect(5.0, 1.0, 0.0, [1.0, 1.0, 0.5, 0.0])),
        pulse: Some(effect(0.6, 4.0, 0.1, [0.5, 2.0, -0.2, 0.0])),
        radial_blur: Some(effect(10.0, 1.0, 0.1, [1.0, 1.0, 0.6, 0.0])),
        arrest: Some(treatment(0.5)),
    })
}

fn weight(frame: &Frame) -> f32 {
    frame.look.weight
}

#[test]
fn the_first_frame_only_records_the_state() {
    let mut look = fixture();
    let frame = look.update(3.0, true, 0.016, 0.016);
    assert_eq!(weight(&frame), 0.0, "a load inside a pursuit does not fire");
    assert_eq!((frame.look.brightness, frame.radial_blur), (1.0, 0.0));
}

#[test]
fn a_pursuit_start_blends_flashes_and_blurs() {
    let mut look = fixture();
    look.update(2.4, false, 0.016, 0.016);
    let start = look.update(2.4, true, 0.016, 0.016);
    assert_eq!(start.look.target, Some(treatment(1.0)));
    assert_eq!(weight(&start), 1.0, "the blend starts at full weight");
    assert_eq!(start.look.brightness, 3.0, "the flash starts at its first ordinate times its magnitude, plus 1");
    assert_eq!(start.radial_blur, 1.0);
    let mut frame = start;
    for _ in 0..60 {
        frame = look.update(2.4, true, 0.05, 0.05);
    }
    assert!(weight(&frame) > 0.0 && weight(&frame) < 1.0, "it eases back: {}", weight(&frame));
    assert_eq!(frame.look.brightness, 1.0, "the flash is over after 0.6 s");
    for _ in 0..40 {
        frame = look.update(2.4, true, 0.05, 0.05);
    }
    assert_eq!(weight(&frame), 0.0, "back to the ordinary look after 5 s");
    assert!(frame.radial_blur > 0.0, "the blur runs for 10 s");
}

#[test]
fn only_a_rise_of_the_whole_heat_level_fires() {
    let mut look = fixture();
    look.update(2.0, true, 0.016, 0.016);
    assert_eq!(weight(&look.update(2.9, true, 0.016, 0.016)), 0.0, "2.0 to 2.9 is the same level");
    assert_eq!(weight(&look.update(3.0, true, 0.016, 0.016)), 1.0, "level 3 fires");
    let mut quiet = fixture();
    quiet.update(4.0, true, 0.016, 0.016);
    let frame = quiet.update(3.0, false, 0.016, 0.016);
    assert_eq!(weight(&frame), 0.0, "a heat drop and a pursuit's end do nothing");
}

#[test]
fn the_flash_runs_on_real_time_and_the_blend_on_world_time() {
    let mut look = fixture();
    look.update(2.0, false, 0.016, 0.016);
    look.update(2.0, true, 0.016, 0.016);
    // The speedbreaker stops the world clock: the blend holds, the flash still ends.
    let mut frame = look.update(2.0, true, 0.0, 0.0);
    for _ in 0..20 {
        frame = look.update(2.0, true, 0.0, 0.05);
    }
    assert_eq!(weight(&frame), 1.0);
    assert_eq!(frame.look.brightness, 1.0);
}

#[test]
fn a_scene_stops_the_blend_and_the_flash_and_its_end_does_not_fire() {
    let mut look = fixture();
    look.update(2.0, false, 0.016, 0.016);
    look.update(2.0, true, 0.016, 0.016);
    look.hold();
    let frame = look.update(2.0, true, 0.016, 0.016);
    assert_eq!(weight(&frame), 0.0);
    assert_eq!(frame.look.brightness, 1.0);
    assert!(frame.radial_blur > 0.0, "the blur runs on");
}

#[test]
fn an_arrest_shows_its_own_look_and_its_end_does_not_fire() {
    let mut look = fixture();
    look.update(2.0, false, 0.016, 0.016);
    look.update(2.0, true, 0.016, 0.016);
    let arrest = look.arrest();
    assert_eq!((arrest.look.target, weight(&arrest)), (Some(treatment(0.5)), 1.0), "the arrest look as it is");
    assert_eq!((arrest.look.brightness, arrest.radial_blur), (1.0, 0.0));
    let after = look.update(2.0, true, 0.016, 0.016);
    assert_eq!((weight(&after), after.look.brightness, after.radial_blur), (0.0, 1.0, 0.0));
}

#[test]
fn missing_data_leaves_out_only_its_part() {
    let mut partial = PursuitLook::new(Profile { look: None, ..fixture().profile });
    partial.update(2.0, false, 0.016, 0.016);
    let frame = partial.update(2.0, true, 0.016, 0.016);
    assert_eq!(frame.look.target, None, "no pursuit look: the ordinary grade");
    assert!(frame.look.brightness > 1.0 && frame.radial_blur > 0.0);
    let mut still = PursuitLook::new(Profile { transition: None, pulse: None, ..fixture().profile });
    still.update(2.0, false, 0.016, 0.016);
    let frame = still.update(2.0, true, 0.016, 0.016);
    assert_eq!((weight(&frame), frame.look.brightness), (0.0, 1.0), "no blend and no flash without their effects");
}

#[test]
fn a_chase_that_starts_with_a_reset_does_not_fire() {
    // A jump's chase: the original starts it inside the jump's reset, and the rewrite forgets the state when it starts.
    let mut look = fixture();
    look.update(1.0, false, 0.016, 0.016);
    look.reset();
    let frame = look.update(1.0, true, 0.016, 0.016);
    assert_eq!((weight(&frame), frame.look.brightness, frame.radial_blur), (0.0, 1.0, 0.0));
    let rise = look.update(2.0, true, 0.016, 0.016);
    assert_eq!(weight(&rise), 1.0, "a later heat rise still fires");
}

#[test]
fn the_helmet_camera_keeps_a_share_of_the_start_effects() {
    let mut look = fixture();
    look.update(2.4, false, 0.016, 0.016);
    let start = look.update(2.4, true, 0.016, 0.016);
    let helmet = start.scaled(HELMET_SHARE);
    assert_eq!(helmet.look.target, start.look.target, "the same look, less of it");
    assert!((helmet.look.weight - HELMET_SHARE).abs() < 1e-6, "grade {}", helmet.look.weight);
    assert!((helmet.look.brightness - (1.0 + 2.0 * HELMET_SHARE)).abs() < 1e-6, "flash {}", helmet.look.brightness);
    assert!((helmet.radial_blur - HELMET_SHARE).abs() < 1e-6, "blur {}", helmet.radial_blur);
}

#[test]
fn a_frame_with_no_effect_is_not_changed_by_the_share() {
    let quiet = fixture().update(3.0, true, 0.016, 0.016);
    assert_eq!(quiet.scaled(HELMET_SHARE), quiet);
    assert_eq!(quiet.scaled(1.0), quiet, "a full share is the frame itself");
}
