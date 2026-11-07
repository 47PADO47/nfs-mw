//! The helmet camera's optics: the field of view that keeps the handlebar and the instrument panel in the picture, at any
//! aspect and speed (`docs/specs/ride6-bike-cameras.md`, section 5).

use blackbox_gfx::Projection;

use super::*;

const WIDE: f32 = 16.0 / 9.0;

fn helmet() -> BikeCamera {
    BikeCamera::derived(1.45, 0.73)
}

/// The rider's eyes of a sport bike in the model's frame (x forward, y left, z up), m.
const EYES: Vec3 = Vec3::new(0.0, 0.0, 1.25);

/// What a helmet camera must show of a sport bike: the two grips (0.26 m ahead of the eyes, 0.34 m to either side, 0.33 m
/// lower: 52 degrees down, the median of the imported bikes' 20 to 60) and the instrument panel between them, ahead of the
/// steering head and as low as the grips.
const COCKPIT: [Vec3; 5] = [
    Vec3::new(0.26, 0.34, 0.92),
    Vec3::new(0.26, -0.34, 0.92),
    Vec3::new(0.40, 0.0, 0.95),
    Vec3::new(0.34, 0.12, 0.93),
    Vec3::new(0.34, -0.12, 0.93),
];

/// A deeper cockpit: the grips 56 degrees down and 51 degrees to the side (the 90th percentile of the imported bikes is 54 and 52).
const DEEP_COCKPIT: [Vec3; 2] = [Vec3::new(0.24, 0.30, 0.90), Vec3::new(0.24, -0.30, 0.90)];

/// Where a point of the model's frame (the bike upright at the world's origin, heading +x) is in the picture, as a share of its
/// half width and half height from the middle; `None` behind the camera.
fn in_picture(lens: &Lens, at: Vec3) -> Option<[f32; 2]> {
    let clip = lens.view_proj() * at.extend(1.0);
    (clip.w > 0.0).then(|| [clip.x / clip.w, clip.y / clip.w])
}

fn picture_of(mount: &Mount, aspect: f32) -> Lens {
    let eye = mount.place(Vec3::ZERO, Quat::IDENTITY, EYES);
    lens(eye, Attitude { yaw: 0.0, pitch: 0.0, roll: 0.0 }, mount, aspect)
}

fn assert_all_in_picture(points: &[Vec3], lens: &Lens, label: &str) {
    for &point in points {
        let at = in_picture(lens, point).unwrap_or_else(|| panic!("{label}: {point:?} is behind the camera"));
        assert!(at[0].abs() < 0.98 && at[1].abs() < 0.98, "{label}: {point:?} is out of the picture at {at:?}");
    }
}

#[test]
fn the_helmet_camera_is_wider_than_rides_own_and_a_bike_with_a_wider_one_keeps_it() {
    let reference = Mount::helmet(&helmet().helmet, false);
    assert!((reference.fov_y(WIDE) - HELMET_FOV_DEG).abs() < 1e-3, "{}", reference.fov_y(WIDE));
    assert!(reference.fov_y(WIDE) > vertical_fov(91.49284) + 10.0, "RIDE 6's own 60 degrees is the narrow one");
    assert!((70.0..=95.0).contains(&HELMET_FOV_DEG), "a wide cockpit view and not a fisheye: {HELMET_FOV_DEG}");
    let mut wide = helmet();
    wide.helmet.fov = 130.0;
    assert_eq!(Mount::helmet(&wide.helmet, false).fov, 130.0);
    let front = Mount::front(&helmet().front, false);
    assert_eq!(front.fov_y(WIDE), vertical_fov(front.fov), "the front camera is not widened");
}

#[test]
fn a_wider_screen_shows_more_at_the_sides_and_a_narrower_one_keeps_the_sides_up_to_a_limit() {
    let mount = Mount::helmet(&helmet().helmet, false);
    let wide_horizontal =
        |aspect: f32| 2.0 * ((mount.fov_y(aspect).to_radians() * 0.5).tan() * aspect).atan().to_degrees();
    assert_eq!(mount.fov_y(21.0 / 9.0), mount.fov_y(WIDE), "the vertical field of view is kept: Hor+");
    assert!(wide_horizontal(21.0 / 9.0) > wide_horizontal(WIDE) + 15.0);
    let minimum = horizontal_fov(HELMET_FOV_DEG);
    for aspect in [16.0 / 10.0, 4.0 / 3.0] {
        assert!(wide_horizontal(aspect) >= minimum - 1e-2, "{aspect}: {} against {minimum}", wide_horizontal(aspect));
        assert!(mount.fov_y(aspect) <= MAX_FOV_DEG);
    }
    // A square or portrait picture (a split screen) stops at the widest vertical field of view.
    assert_eq!(mount.fov_y(1.0), MAX_FOV_DEG);
    assert_eq!(mount.fov_y(0.5), MAX_FOV_DEG);
    // A camera with no floor (the front one) keeps its own at every aspect.
    let front = Mount::front(&helmet().front, false);
    assert_eq!(front.fov_y(4.0 / 3.0), front.fov_y(WIDE));
}

#[test]
fn the_grips_and_the_instrument_panel_are_in_the_helmet_picture_at_every_usual_aspect() {
    let mount = Mount::helmet(&helmet().helmet, false);
    for (aspect, label) in [(WIDE, "16:9"), (21.0 / 9.0, "21:9"), (16.0 / 10.0, "16:10"), (4.0 / 3.0, "4:3")] {
        let shot = picture_of(&mount, aspect);
        assert_all_in_picture(&COCKPIT, &shot, label);
        assert_all_in_picture(&DEEP_COCKPIT, &shot, label);
    }
}

#[test]
fn the_data_cameras_own_field_of_view_cut_the_grips_off_and_the_wider_one_does_not() {
    // RIDE 6's 91.49 degrees (60 vertical) with the data's tilt and the old 8 extra degrees down.
    let narrow =
        Mount { fov: 91.49284, min_horizontal_deg: 0.0, pitch_deg: -18.0, ..Mount::helmet(&helmet().helmet, false) };
    let old = picture_of(&narrow, WIDE);
    let cut = DEEP_COCKPIT.iter().filter(|point| in_picture(&old, **point).is_none_or(|at| at[1].abs() >= 1.0));
    assert_eq!(cut.count(), 2, "the deep cockpit's grips are below the old picture");
    assert_all_in_picture(&DEEP_COCKPIT, &picture_of(&Mount::helmet(&helmet().helmet, false), WIDE), "now");
}

#[test]
fn the_field_of_view_widens_with_speed_from_nothing_to_its_limit_and_never_narrows() {
    assert_eq!(speed_widening(0.0, 1.0), 0.0);
    assert!((speed_widening(80.0, 1.0) - HELMET_SPEED_WIDEN_DEG).abs() < 1e-4);
    assert!((speed_widening(300.0, 1.0) - HELMET_SPEED_WIDEN_DEG).abs() < 1e-4, "no more than the top");
    assert!((speed_widening(-80.0, 1.0) - HELMET_SPEED_WIDEN_DEG).abs() < 1e-4, "reverse counts as speed");
    assert_eq!(speed_widening(80.0, 0.0), 0.0, "an intensity of 0 keeps the rest field of view");
    assert!((speed_widening(80.0, 0.5) - HELMET_SPEED_WIDEN_DEG * 0.5).abs() < 1e-4);
    let mut last = 0.0;
    for speed in 0..=90 {
        let widening = speed_widening(speed as f32, 1.0);
        assert!(widening >= last, "{speed}");
        last = widening;
    }
    let rest = Mount::helmet(&helmet().helmet, false);
    let fast = rest.with_widening(speed_widening(80.0, 1.0));
    assert!((fast.fov_y(WIDE) - rest.fov_y(WIDE) - HELMET_SPEED_WIDEN_DEG).abs() < 1e-3);
    assert_all_in_picture(&DEEP_COCKPIT, &picture_of(&fast, WIDE), "at speed");
}

#[test]
fn the_near_plane_is_the_datas_ten_centimetres_and_the_panel_is_beyond_it() {
    let mount = Mount::helmet(&helmet().helmet, false);
    assert_eq!(mount.near, 0.1);
    let shot = picture_of(&mount, WIDE);
    let Projection::PerspectiveInfiniteReverse { near, .. } = shot.projection else { panic!("a perspective") };
    assert_eq!(near, mount.near);
    for point in COCKPIT.iter().chain(&DEEP_COCKPIT) {
        let depth = (*point - EYES).dot(shot.forward);
        assert!(depth > near * 2.0, "{point:?} is {depth} m from the lens: too near");
    }
}
