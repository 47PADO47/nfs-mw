use super::*;

const DT: f32 = 1.0 / 60.0;

fn bike(x: f32) -> Bike {
    Bike { position: Vec3::new(x, 0.0, 0.5), heading: 0.0, velocity: Vec3::ZERO }
}

/// Run `seconds` of steps with the rider at `at` in `phase`.
fn run(cam: &mut RiderCam, phase: Phase, at: Option<Vec3>, bike: Bike, seconds: f32) {
    for _ in 0..(seconds / DT).round() as u32 {
        cam.step(phase, at, bike, DT);
    }
}

#[test]
fn the_subject_is_the_rider_while_he_is_fallen_or_jogging_and_the_bike_otherwise() {
    assert_eq!(subject(Phase::Riding, true), Subject::Bike, "remounted or never fell");
    assert_eq!(subject(Phase::Fallen, true), Subject::Rider);
    assert_eq!(subject(Phase::Jogging, true), Subject::Rider);
    assert_eq!(subject(Phase::Fallen, false), Subject::Bike, "a rider with no body to look at: the bike");
    assert_eq!(subject(Phase::Jogging, false), Subject::Bike);
}

#[test]
fn the_phases_off_the_bike() {
    assert!(!Phase::Riding.off_bike());
    assert!(Phase::Fallen.off_bike() && Phase::Jogging.off_bike());
}

#[test]
fn a_rider_on_the_bike_never_moves_the_camera() {
    let mut cam = RiderCam::default();
    run(&mut cam, Phase::Riding, None, bike(0.0), 2.0);
    assert_eq!(cam.weight(), 0.0);
    assert!(!cam.active() && cam.focus().is_none());
    let point = Vec3::new(3.0, 4.0, 0.5);
    assert_eq!(cam.chase_point(point, 0.7), point);
    assert_eq!(cam.heading(1.0), 1.0);
}

#[test]
fn the_camera_moves_to_the_rider_in_the_time_asked_and_stays() {
    let mut cam = RiderCam::default();
    let rider = Some(Vec3::new(12.0, 3.0, 0.2));
    run(&mut cam, Phase::Fallen, rider, bike(0.0), TO_RIDER_S * 0.5);
    let half = cam.weight();
    assert!(half > 0.3 && half < 0.7, "half way, eased: {half}");
    run(&mut cam, Phase::Fallen, rider, bike(0.0), TO_RIDER_S);
    assert_eq!(cam.weight(), 1.0);
    // On the rider the chase point is his, looking RIDER_LOOK_UP_M above his pelvis.
    let point = cam.chase_point(Vec3::new(0.0, 0.0, 0.5), 0.7);
    assert!((point.x - 12.0).abs() < 1e-4 && (point.y - 3.0).abs() < 1e-4);
    assert!((point.z + 0.7 - RIDER_LOOK_UP_M - 0.2).abs() < 1e-4, "the camera looks at him: {point}");
}

#[test]
fn the_chase_point_blends_between_the_bike_and_the_rider() {
    let mut cam = RiderCam::default();
    let rider = Vec3::new(10.0, 0.0, 0.2);
    run(&mut cam, Phase::Fallen, Some(rider), bike(0.0), TO_RIDER_S * 0.5);
    let point = cam.chase_point(Vec3::ZERO, 0.0);
    assert!(point.x > 2.0 && point.x < 8.0, "between the two: {point}");
}

#[test]
fn the_camera_follows_a_rider_who_moves_and_eases_into_a_jump_instead_of_snapping() {
    let mut cam = RiderCam::default();
    run(&mut cam, Phase::Fallen, Some(Vec3::new(5.0, 0.0, 0.3)), bike(0.0), 1.0);
    cam.step(Phase::Fallen, Some(Vec3::new(9.0, 0.0, 0.3)), bike(0.0), DT);
    let point = cam.chase_point(Vec3::ZERO, 0.0);
    assert!(point.x > 5.0 && point.x < 6.0, "a jump of 4 m in one step is not followed in one frame: {point}");
    assert!(cam.velocity().x > 5.0 && cam.velocity().x <= FOLLOW_MAX_ACCEL * DT + 1e-3, "{}", cam.velocity());
    run(&mut cam, Phase::Fallen, Some(Vec3::new(9.0, 0.0, 0.3)), bike(0.0), 1.0);
    let point = cam.chase_point(Vec3::ZERO, 0.0);
    assert!((point.x - 9.0).abs() < 0.05, "and arrives: {point}");
}

/// The point the camera follows (the pelvis, `focus`) for a rider sliding along +X at `speed` whose pelvis shakes by `noise` metres.
fn slide(speed: f32, noise: f32, seconds: f32) -> Vec<(Vec3, f32)> {
    let mut cam = RiderCam::default();
    let mut out = Vec::new();
    for i in 0..(seconds / DT) as u32 {
        let jitter = if i % 2 == 0 { noise } else { -noise };
        let at = Vec3::new(speed * i as f32 * DT + jitter, jitter, 0.3 - jitter);
        cam.step(Phase::Fallen, Some(at), bike(0.0), DT);
        out.push((cam.focus().unwrap_or(Vec3::ZERO), cam.heading));
    }
    out
}

/// The mean magnitude of the second difference of a path, as an acceleration (m/s^2).
fn shake(path: &[Vec3]) -> f32 {
    let sum: f32 = path.windows(3).map(|w| (w[2] - 2.0 * w[1] + w[0]).length() / (DT * DT)).sum();
    sum / (path.len() - 2) as f32
}

#[test]
fn the_pelvis_of_a_shaking_ragdoll_does_not_shake_the_followed_point() {
    // The measured pelvis of a sliding rider moved by tens of cm/s^2 of noise: 6 cm of alternating noise at 60 Hz stands for it.
    let quiet: Vec<Vec3> = slide(6.0, 0.0, 3.0).iter().map(|&(p, _)| p).skip(60).collect();
    let noisy: Vec<Vec3> = slide(6.0, 0.06, 3.0).iter().map(|&(p, _)| p).skip(60).collect();
    assert!(shake(&quiet) < 1.0, "a steady slide is a steady point: {}", shake(&quiet));
    assert!(shake(&noisy) < 100.0, "6 cm of noise shakes the point by {} m/s^2 (the raw one by 1500)", shake(&noisy));
}

#[test]
fn the_point_never_accelerates_beyond_the_cap_and_never_leaves_the_path_by_more_than_the_lag() {
    let path = slide(25.0, 0.0, 4.0);
    let speeds: Vec<Vec3> = path.iter().map(|&(p, _)| p).collect();
    let worst = speeds.windows(3).map(|w| (w[2] - 2.0 * w[1] + w[0]).length() / (DT * DT)).fold(0.0, f32::max);
    // The second difference of positions is not exactly the change of the velocity the cap holds: a quarter of room.
    assert!(worst <= FOLLOW_MAX_ACCEL * 1.25, "{worst}");
    let last = speeds.len() as f32 * DT;
    let lag = 25.0 * (last - DT) - speeds[speeds.len() - 1].x;
    assert!(lag < 2.0 * 25.0 / FOLLOW_OMEGA + 0.2, "{lag} m behind at 25 m/s");
}

#[test]
fn the_heading_does_not_flip_with_the_shake_of_a_slow_slide() {
    // At 2 m/s the shake's velocity is 7 m/s the other way: taken raw, the heading turned back and forth.
    let headings: Vec<f32> = slide(2.0, 0.06, 4.0).iter().map(|&(_, h)| h).skip(60).collect();
    let swing = headings.iter().cloned().fold(f32::MIN, f32::max) - headings.iter().cloned().fold(f32::MAX, f32::min);
    assert!(swing < 0.3, "the heading swung {swing} rad");
}

#[test]
fn the_heading_turns_at_a_limited_rate() {
    let mut cam = RiderCam::default();
    // A slide that turns round at once: the rider moves along +X, then along -X.
    let mut at = Vec3::new(0.0, 0.0, 0.3);
    for _ in 0..90 {
        at.x += 8.0 * DT;
        cam.step(Phase::Fallen, Some(at), bike(0.0), DT);
    }
    assert!(cam.heading.abs() < 0.05, "along +X: {}", cam.heading);
    let mut worst = 0.0_f32;
    for _ in 0..180 {
        let before = cam.heading;
        at.x -= 8.0 * DT;
        cam.step(Phase::Fallen, Some(at), bike(0.0), DT);
        worst = worst.max(blend_angle(0.0, cam.heading - before, 1.0).abs());
    }
    assert!(worst <= HEADING_MAX_RATE * DT + 1e-4, "{worst} rad in a step");
    assert!((cam.heading.abs() - std::f32::consts::PI).abs() < 0.05, "it does turn round in the end: {}", cam.heading);
}

#[test]
fn the_followed_point_is_kept_above_the_ground_it_sank_into() {
    let above = Vec3::new(1.0, 2.0, 5.0);
    assert_eq!(above_ground(above, Some(4.0)), above, "above the ground: as it is");
    assert_eq!(above_ground(above, None), above, "no ground known: as it is");
    let sunk = above_ground(Vec3::new(1.0, 2.0, 3.9), Some(4.0));
    assert!((sunk.z - (4.0 + GROUND_CLEARANCE_M)).abs() < 1e-6 && sunk.x == 1.0 && sunk.y == 2.0, "{sunk}");
    let lost = Vec3::new(1.0, 2.0, 1.0);
    assert_eq!(above_ground(lost, Some(4.0)), lost, "3 m under it is under the map, not a pelvis in the road");
    assert_eq!(above_ground(above, Some(f32::NAN)), above, "a ground that is not a number is no ground");
}

#[test]
fn the_camera_on_a_rider_who_sank_looks_above_the_ground() {
    let mut cam = RiderCam::default();
    let sunk = Some(Vec3::new(12.0, 3.0, 9.8));
    cam.set_floor(Some(10.0));
    run(&mut cam, Phase::Fallen, sunk, bike(0.0), TO_RIDER_S * 2.0);
    let focus = cam.focus().expect("on the rider");
    assert!(focus.z >= 10.0 + GROUND_CLEARANCE_M - 1e-5, "{focus}");
    let shown = cam.focus_at(0.5).expect("on the rider");
    assert!(shown.z >= 10.0 + GROUND_CLEARANCE_M - 1e-5, "{shown}");
}

#[test]
fn the_heading_is_the_direction_of_the_slide_while_it_lasts_and_then_held() {
    let mut cam = RiderCam::default();
    // The bike heads along +X; the rider slides off towards +Y at 6 m/s.
    let mut at = Vec3::new(0.0, 0.0, 0.3);
    for _ in 0..60 {
        at.y += 6.0 * DT;
        cam.step(Phase::Fallen, Some(at), bike(0.0), DT);
    }
    assert!((cam.heading - std::f32::consts::FRAC_PI_2).abs() < 0.01, "towards +Y: {}", cam.heading);
    let held = cam.heading;
    run(&mut cam, Phase::Fallen, Some(at), bike(0.0), 1.0);
    assert!((cam.heading - held).abs() < 1e-6, "lying still keeps the heading");
}

#[test]
fn the_jogging_rider_faces_the_bike() {
    let mut cam = RiderCam::default();
    // The rider stands 10 m behind the bike on the -X side, jogging towards it.
    let mut at = Vec3::new(-10.0, 0.0, 1.0);
    for _ in 0..10 {
        at.x += 3.0 * DT;
        cam.step(Phase::Jogging, Some(at), bike(0.0), DT);
    }
    assert!(cam.heading.abs() < 1e-4, "towards +X, where the bike is: {}", cam.heading);
    // Next to the bike the heading is held, not spun round.
    cam.step(Phase::Jogging, Some(Vec3::new(0.1, 0.2, 1.0)), bike(0.0), DT);
    assert!(cam.heading.abs() < 1e-4);
}

#[test]
fn the_camera_keeps_to_the_rider_while_he_jogs_and_goes_back_to_the_bike_when_he_remounts() {
    let mut cam = RiderCam::default();
    let rider = Some(Vec3::new(8.0, 0.0, 1.0));
    run(&mut cam, Phase::Fallen, rider, bike(0.0), 3.0);
    run(&mut cam, Phase::Jogging, rider, bike(0.0), 1.0);
    assert_eq!(cam.weight(), 1.0, "jogging is still the rider's shot");
    // He remounts: the host has no rider body off the bike any more.
    run(&mut cam, Phase::Riding, None, bike(0.0), TO_BIKE_S * 0.5);
    let back = cam.weight();
    assert!(back > 0.3 && back < 0.7, "easing back to the bike: {back}");
    run(&mut cam, Phase::Riding, None, bike(0.0), TO_BIKE_S);
    assert_eq!(cam.weight(), 0.0);
    assert!(cam.focus().is_none());
}

#[test]
fn after_the_remount_the_blend_follows_the_moving_bike_without_a_jump() {
    let mut cam = RiderCam::default();
    // He is on the seat, 1 m above the bike's origin, when he remounts.
    run(&mut cam, Phase::Jogging, Some(Vec3::new(0.0, 0.0, 1.5)), bike(0.0), 1.0);
    cam.step(Phase::Riding, None, bike(0.0), DT);
    let before = cam.chase_point(bike(0.0).position, 0.5);
    // The bike rides away at 20 m/s for a moment: the camera's point stays with it.
    let mut later = bike(0.0);
    later.position.x += 0.3;
    cam.step(Phase::Riding, None, later, DT);
    let after = cam.chase_point(later.position, 0.5);
    assert!((after.x - before.x - 0.3).abs() < 0.05, "{before} -> {after}");
}

#[test]
fn a_rider_with_no_body_leaves_the_camera_on_the_bike() {
    let mut cam = RiderCam::default();
    run(&mut cam, Phase::Fallen, None, bike(0.0), 3.0);
    assert_eq!(cam.weight(), 0.0, "nobody to look at: the bike");
    assert_eq!(cam.chase_point(Vec3::X, 0.5), Vec3::X);
}

#[test]
fn a_placement_cuts_the_camera_to_the_bike_until_the_rider_is_on_it() {
    let mut cam = RiderCam::default();
    let rider = Some(Vec3::new(20.0, 0.0, 0.2));
    run(&mut cam, Phase::Fallen, rider, bike(0.0), 2.0);
    cam.cut_to_bike();
    assert_eq!(cam.weight(), 0.0);
    run(&mut cam, Phase::Jogging, rider, bike(0.0), 1.0);
    assert_eq!(cam.weight(), 0.0, "the rider is still blending in from where he lay: not followed");
    run(&mut cam, Phase::Riding, None, bike(0.0), DT);
    run(&mut cam, Phase::Fallen, rider, bike(0.0), 1.0);
    assert_eq!(cam.weight(), 1.0, "a new fall is followed again");
}

#[test]
fn headings_blend_the_short_way_round() {
    use std::f32::consts::PI;
    let a = blend_angle(PI - 0.1, -PI + 0.1, 0.5);
    assert!((a.abs() - PI).abs() < 1e-4, "across the seam, not through zero: {a}");
    assert!((blend_angle(0.0, 1.0, 0.25) - 0.25).abs() < 1e-6);
    assert_eq!(blend_angle(2.0, -1.0, 0.0), 2.0);
}

#[test]
fn nothing_can_bust_the_rider_while_he_is_off_the_bike_or_for_a_while_after() {
    let mut cam = RiderCam::default();
    assert!(!cam.bust_grace());
    run(&mut cam, Phase::Fallen, Some(Vec3::Z), bike(0.0), 5.0);
    assert!(cam.bust_grace(), "lying down");
    run(&mut cam, Phase::Jogging, Some(Vec3::Z), bike(0.0), 5.0);
    assert!(cam.bust_grace(), "jogging back, however long it takes");
    cam.step(Phase::Riding, None, bike(0.0), DT);
    assert!(cam.bust_grace(), "just remounted");
    run(&mut cam, Phase::Riding, None, bike(0.0), BIKE_RESET_GRACE_S - 0.5);
    assert!(cam.bust_grace(), "still in the grace");
    run(&mut cam, Phase::Riding, None, bike(0.0), 1.0);
    assert!(!cam.bust_grace(), "over after {BIKE_RESET_GRACE_S} s");
}

#[test]
fn a_reset_by_hand_while_riding_starts_the_grace() {
    let mut cam = RiderCam::default();
    cam.note_reset();
    assert!(cam.bust_grace());
    run(&mut cam, Phase::Riding, None, bike(0.0), BIKE_RESET_GRACE_S + 0.1);
    assert!(!cam.bust_grace());
}

#[test]
fn a_bad_step_does_nothing() {
    let mut cam = RiderCam::default();
    cam.step(Phase::Fallen, Some(Vec3::ZERO), bike(0.0), 0.0);
    cam.step(Phase::Fallen, Some(Vec3::ZERO), bike(0.0), f32::NAN);
    assert!(!cam.bust_grace() && cam.weight() == 0.0);
}

#[test]
fn the_camera_is_lifted_over_the_rider_by_the_weight() {
    let mut cam = RiderCam::default();
    assert_eq!(cam.lifted_height(0.5, 2.0), 0.5, "on the bike nothing changes");
    run(&mut cam, Phase::Fallen, Some(Vec3::Z), bike(0.0), 2.0);
    let lifted = cam.lifted_height(0.5, 2.0);
    assert!((lifted * 2.0 - 1.0 - RIDER_CAMERA_LIFT_M).abs() < 1e-5, "{lifted}");
    assert!(cam.lifted_height(0.5, 0.0).is_finite(), "a preferred height of nothing");
}
