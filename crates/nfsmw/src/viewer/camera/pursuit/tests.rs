use super::*;
use crate::viewer::camera::ChaseCamera;

const TICK: f32 = 1.0 / 60.0;

fn cop() -> Vec3 {
    Vec3::new(8.0, 0.0, 0.0)
}

fn feed(meter: f32, busted: bool, cop: Option<Vec3>) -> PursuitFeed {
    PursuitFeed { meter, busted, cop, player: Vec3::ZERO }
}

fn run(cam: &mut PursuitCamera, seconds: f32, f: PursuitFeed) {
    for _ in 0..(seconds / TICK).round() as usize {
        cam.update(TICK, f);
    }
}

#[test]
fn the_first_rise_above_zero_starts_one_pan_per_catch() {
    let mut cam = PursuitCamera::default();
    cam.update(TICK, feed(0.0, false, Some(cop())));
    assert!(cam.catch.is_none(), "no pan with the meter empty");
    cam.update(TICK, feed(0.2, false, Some(cop())));
    assert!(cam.catch.is_some(), "the first rise above zero starts the pan");
    run(&mut cam, CATCH_SECS + 0.5, feed(0.5, false, Some(cop())));
    assert!(cam.catch.is_none(), "the pan ends on its own time");
    assert!(!cam.armed, "the same catch does not pan again");
    cam.update(TICK, feed(0.0, false, Some(cop())));
    assert!(cam.armed, "the meter draining to zero re-arms the pan");
    cam.update(TICK, feed(0.3, false, Some(cop())));
    assert!(cam.catch.is_some(), "the next catch pans");
}

#[test]
fn a_catch_needs_a_cop_to_frame() {
    let mut cam = PursuitCamera::default();
    cam.update(TICK, feed(0.2, false, None));
    assert!(cam.catch.is_none() && cam.armed, "no cop seen yet, so no pan and the pan stays armed");
}

#[test]
fn the_pan_weight_peaks_mid_pan_and_the_shot_ends_with_it() {
    let mut cam = PursuitCamera::default();
    cam.update(TICK, feed(0.2, false, Some(cop())));
    run(&mut cam, CATCH_SECS / 2.0, feed(0.2, false, Some(cop())));
    let peak = cam.shot.expect("a shot mid-pan").weight;
    assert!(peak > 0.95, "the pan is near full weight halfway: {peak}");
    run(&mut cam, CATCH_SECS, feed(0.2, false, Some(cop())));
    assert!(cam.shot.is_none(), "the chase camera has its way back");
}

#[test]
fn busted_starts_the_arrest_and_it_holds_for_its_time_only() {
    let mut cam = PursuitCamera::default();
    run(&mut cam, 0.5, feed(0.3, false, Some(cop())));
    cam.update(TICK, feed(1.0, true, Some(cop())));
    assert!(cam.arrest.is_some(), "busted starts the arrest framing");
    assert!(cam.catch.is_none(), "the caught pan gives way to the arrest");
    run(&mut cam, ARREST_SECS + 0.5, feed(1.0, true, Some(cop())));
    assert!(cam.arrest.is_none(), "the framing holds for its time only");
    assert!(cam.was_busted, "still busted until the next chase, so it does not replay");
}

#[test]
fn a_new_chase_arms_the_next_bust() {
    let mut cam = PursuitCamera::default();
    cam.update(TICK, feed(1.0, true, Some(cop())));
    assert!(cam.arrest.is_some());
    cam.update(TICK, feed(0.0, false, None));
    assert!(cam.arrest.is_none() && !cam.was_busted, "the new chase clears the bust");
    cam.update(TICK, feed(1.0, true, Some(cop())));
    assert!(cam.arrest.is_some(), "the next bust arrests again");
}

#[test]
fn the_arrest_weight_eases_in_and_out_and_holds() {
    assert_eq!(arrest_weight(0.0), 0.0);
    assert_eq!(arrest_weight(ARREST_SECS / 2.0), 1.0);
    assert!(arrest_weight(ARREST_SECS - 0.01) < 0.01);
}

#[test]
fn no_shot_leaves_the_chase_view_alone() {
    let base = ChaseCamera::default().view();
    let cam = PursuitCamera::default();
    assert_eq!(cam.shot(base), base);
}

#[test]
fn the_arrest_shot_blends_to_its_framing() {
    let mut cam = PursuitCamera::default();
    let base = ChaseCamera::default().view();
    cam.update(TICK, feed(1.0, true, Some(cop())));
    run(&mut cam, ARREST_SECS / 2.0, feed(1.0, true, Some(cop())));
    let framing = cam.arrest.expect("the arrest is running").framing;
    let shot = cam.shot(base);
    assert!((shot.position - framing.position).length() < 1e-3, "at full weight the framing is the view");
    assert!((shot.fov_degrees() - framing.fov_degrees).abs() < 1e-3);
}

#[test]
fn the_framing_stands_beside_the_pair_and_looks_between_them() {
    let player = Vec3::ZERO;
    let f = framing::between(player, cop());
    let middle = (player + cop()) * 0.5;
    assert!(f.position.z > middle.z, "the camera stands above the pair");
    assert!((f.position - middle).truncate().length() >= 6.0 - 1e-3, "it stands off to the side");
    assert!((f.target - middle).length() < 1.5, "it looks at the middle of the pair");
}
