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
fn the_camera_follows_a_rider_who_moves() {
    let mut cam = RiderCam::default();
    run(&mut cam, Phase::Fallen, Some(Vec3::new(5.0, 0.0, 0.3)), bike(0.0), 1.0);
    run(&mut cam, Phase::Fallen, Some(Vec3::new(9.0, 0.0, 0.3)), bike(0.0), DT);
    let point = cam.chase_point(Vec3::ZERO, 0.0);
    assert!((point.x - 9.0).abs() < 1e-4, "the point is where he is now: {point}");
    assert!(cam.velocity().x > 100.0, "a jump of 4 m in a step is a velocity: {}", cam.velocity());
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
