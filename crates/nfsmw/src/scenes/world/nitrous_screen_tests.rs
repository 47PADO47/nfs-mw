use super::*;

fn screen() -> ScreenProfile {
    let text = "format = 1\n[screen]\nfov = 8.0\npull_back = 1.0\nshake = 0.02\nedge = [0.1, 0.2, 0.3]\nfade_in = 1.0\nfade_out = 1.0";
    blackbox_modding::content::parse_nitrous_profile(text).unwrap().screen.unwrap()
}

fn lens() -> Lens {
    Lens::look_at(Vec3::new(0.0, -5.0, 2.0), Vec3::new(0.0, 0.0, 2.0), 50.0, NEAR_PLANE, 16.0 / 9.0)
}

#[test]
fn without_a_profile_nothing_moves_and_the_burn_is_forgotten() {
    let mut state = NitrousScreen::default();
    assert_eq!(state.step(Some(screen()), true, 0.5), 0.5);
    assert!(!state.camera().is_zero());
    assert_eq!(state.step(None, true, 0.5), 0.0);
    assert!(state.camera().is_zero());
    assert_eq!(state.step(Some(screen()), true, 0.25), 0.25, "the level starts again from rest");
}

#[test]
fn the_nudge_follows_the_burn_and_dies_out_after_it() {
    let mut state = NitrousScreen::default();
    state.step(Some(screen()), true, 1.0);
    let full = state.camera();
    assert_eq!((full.fov, full.pull_back), (8.0, 1.0));
    state.step(Some(screen()), false, 0.5);
    assert_eq!(state.camera().fov, 4.0, "the field of view falls with the level");
    state.step(Some(screen()), false, 5.0);
    assert_eq!((state.camera().fov, state.camera().pull_back), (0.0, 0.0));
    assert_eq!(state.camera().shake, [0.0; 3]);
}

#[test]
fn a_zero_nudge_leaves_the_lens_as_it_is() {
    assert_eq!(CameraNudge::default().apply(lens(), 16.0 / 9.0, true), lens());
}

#[test]
fn the_pull_back_moves_the_eye_behind_and_keeps_the_look() {
    let nudge = CameraNudge { pull_back: 2.0, ..Default::default() };
    let moved = nudge.apply(lens(), 16.0 / 9.0, true);
    assert!((moved.position - Vec3::new(0.0, -7.0, 2.0)).length() < 1e-5, "{:?}", moved.position);
    assert_eq!(moved.forward, lens().forward);
    // A point straight ahead is two metres further from the eye in view space.
    let ahead = Vec3::new(0.0, 0.0, 2.0);
    let before = lens().view.transform_point3(ahead);
    let after = moved.view.transform_point3(ahead);
    assert!(((after.z - before.z).abs() - 2.0).abs() < 1e-4, "{before:?} {after:?}");
    let mounted = nudge.apply(lens(), 16.0 / 9.0, false);
    assert_eq!(mounted.position, lens().position, "a mounted camera keeps its place");
}

#[test]
fn the_field_of_view_widens_within_its_limits() {
    let nudge = CameraNudge { fov: 8.0, ..Default::default() };
    assert_eq!(nudge.apply(lens(), 16.0 / 9.0, true).fov_y_degrees, 58.0);
    let huge = CameraNudge { fov: 60.0, ..Default::default() };
    assert_eq!(huge.apply(Lens { fov_y_degrees: 120.0, ..lens() }, 16.0 / 9.0, true).fov_y_degrees, 150.0);
}

#[test]
fn the_shake_moves_the_eye_sideways_and_up() {
    let nudge = CameraNudge { shake: [0.01, 0.02, 0.0], ..Default::default() };
    let moved = nudge.apply(lens(), 16.0 / 9.0, true);
    let delta = moved.position - lens().position;
    assert!((delta.length() - (0.01f32.powi(2) + 0.02f32.powi(2)).sqrt()).abs() < 1e-6, "{delta:?}");
    assert!(delta.dot(lens().forward).abs() < 1e-6, "a sideways shake does not move the eye along the view");
}

#[test]
fn the_grade_is_neutral_without_colour_keys_and_mixes_with_the_level_otherwise() {
    let none =
        blackbox_modding::content::parse_nitrous_profile("format = 1\n[screen]\nfov = 5.0").unwrap().screen.unwrap();
    assert!(grade_for(&none, 1.0).is_neutral());
    let grade = grade_for(&screen(), 1.0);
    assert!(grade.edge.iter().zip([0.1, 0.2, 0.3]).all(|(a, b)| (a - b).abs() < 1e-6));
    assert!(grade_for(&screen(), 0.0).is_neutral(), "no level, no grade, so the pass leaves the chain");
}
