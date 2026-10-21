use super::*;
use crate::scenes::world::scene_space::placed;
use blackbox_nis::SceneType;
use blackbox_nis::placement::{render_to_world, scene_to_render};
use std::f32::consts::FRAC_1_SQRT_2;

/// The eye of the first key of `IntroNisDD` in the install (`docs/specs/nis-scenes.md`, section 8): a type-4 scene.
const INTRO_NIS_DD_EYE: [f32; 3] = [1038.395, -11.174, 246.477];

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-3
}

/// A synthetic DOF buffer of three bones. Bone 1, the root, carries a translation and a rotation; bones 0 and 2 are
/// left at zero.
fn buffer() -> Vec<f32> {
    let mut dofs = vec![0.0; 3 * DOFS_PER_BONE];
    let root = DOFS_PER_BONE;
    dofs[root + TRANSLATION_SLOT..root + TRANSLATION_SLOT + 3].copy_from_slice(&[1.0, 2.0, 3.0]);
    dofs[root + ROTATION_SLOT..root + ROTATION_SLOT + 4].copy_from_slice(&[0.0, 0.0, 0.6, 0.8]);
    dofs
}

#[test]
fn a_channel_name_gives_its_slot_and_part() {
    assert_eq!(channel_of("IntroNis05ZPM_Car1_t"), Some((0, Part::Translation)));
    assert_eq!(channel_of("IntroNis05ZPM_Car8_q"), Some((7, Part::Rotation)));
    assert_eq!(channel_of("EndingNis04ZPM_Cop3_q"), Some((10, Part::Rotation)), "cop 3 is slot 10");
    assert_eq!(channel_of("ArrestF02ZPM_Cop8_t"), Some((15, Part::Translation)));
    assert_eq!(channel_of("IntroNis05ZPM_Car1_s"), None, "the _s channel is not read");
    assert_eq!(channel_of("IntroNis05ZPM_Car9_t"), None, "there is no ninth car");
    assert_eq!(channel_of("IntroNis05ZPM_Car1"), None, "the base name has no part");
    assert_eq!(channel_of("ArrestF02_Perp_s"), None, "a character clip is not a car");
}

#[test]
fn the_root_bone_is_the_lowest_dof_at_the_offset() {
    // Bone 2 translation at 32, bone 1 translation at 20, bone 1 rotation at 16, bone 0 scale at 0.
    let dofs = [32usize, 20, 16, 0];
    assert_eq!(first_at_offset(dofs.iter().copied(), TRANSLATION_SLOT), Some(20));
    assert_eq!(first_at_offset(dofs.iter().copied(), ROTATION_SLOT), Some(16));
    assert_eq!(first_at_offset(dofs.iter().copied(), 1), None, "no channel writes that slot");
}

#[test]
fn the_pose_is_read_from_the_root_bones_slots() {
    let dofs = buffer();
    let root = DOFS_PER_BONE;
    assert_eq!(read_translation(&dofs, root + TRANSLATION_SLOT), Vec3::new(1.0, 2.0, 3.0));
    let q = read_rotation(&dofs, root + ROTATION_SLOT);
    assert!((q - Quat::from_xyzw(0.0, 0.0, 0.6, 0.8)).length() < 1e-6, "{q:?}");
}

#[test]
fn a_rotation_is_normalised_and_a_missing_one_is_the_identity() {
    let mut dofs = buffer();
    let at = DOFS_PER_BONE + ROTATION_SLOT;
    dofs[at..at + 4].copy_from_slice(&[0.0, 0.0, 2.0, 0.0]);
    assert!((read_rotation(&dofs, at) - Quat::from_xyzw(0.0, 0.0, 1.0, 0.0)).length() < 1e-6);
    dofs[at..at + 4].copy_from_slice(&[0.0; 4]);
    assert_eq!(read_rotation(&dofs, at), Quat::IDENTITY, "a zero quaternion has no direction");
    assert_eq!(read_rotation(&dofs, dofs.len()), Quat::IDENTITY, "past the end of the buffer");
    assert_eq!(read_translation(&dofs, dofs.len()), Vec3::ZERO, "past the end of the buffer");
}

#[test]
fn a_car_is_set_five_metres_above_the_ground_under_it() {
    let spot = CarSpot { position: Vec3::new(1.0, 2.0, 40.0), rotation: Quat::IDENTITY };
    assert!(near(lifted(spot, Some(10.0)).position, Vec3::new(1.0, 2.0, 15.0)));
    assert_eq!(lifted(spot, None), spot, "no ground under the car, no lift");
}

#[test]
fn a_type_4_scene_is_authored_in_render_space_so_its_eye_stays_where_the_track_has_it() {
    let marker = Anchor::facing(Vec3::new(500.0, -200.0, 12.0), 1234);
    let anchor = placed(SceneType::Type4, marker);
    assert_eq!(anchor, Anchor::IDENTITY, "types 1 and 4 ignore the marker");
    let eye = Vec3::from_array(INTRO_NIS_DD_EYE);
    assert!(near(anchor.to_render(INTRO_NIS_DD_EYE), eye));
    assert!(near(CarSpot::in_scene(&anchor, eye, Quat::IDENTITY).position, eye));
}

#[test]
fn the_world_reading_of_the_type_4_eye_would_put_it_below_the_ground() {
    // The render frame is the one the spec gives (`nis-player.md` section 5.1): render to world is (-Y, Z, X). Read as a
    // world point instead, the eye would map to render (246.477, -1038.395, -11.174), a height of -11 m. The spec's
    // reading keeps the eye 246 m up, so the type-4 coordinates are not converted.
    let as_world = render_to_world().inverse().transform_point3(Vec3::from_array(INTRO_NIS_DD_EYE));
    assert!(near(as_world, Vec3::new(246.477, -1038.395, -11.174)), "{as_world:?}");
    let as_render = render_to_world().transform_point3(Vec3::from_array(INTRO_NIS_DD_EYE));
    assert!(near(as_render, Vec3::new(11.174, 246.477, 1038.395)), "{as_render:?}");
}

#[test]
fn types_0_2_and_3_are_placed_the_way_the_placement_library_places_them() {
    let origin = Vec3::new(300.0, -40.0, 8.0);
    let heading: u16 = 20000;
    let points = [[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [-3.0, 7.0, 2.0]];
    for scene_type in [SceneType::Type0, SceneType::Type2, SceneType::Type3] {
        let anchor = placed(scene_type, Anchor::facing(origin, heading));
        let library = scene_to_render(scene_type, origin, heading);
        for point in points {
            let ours = anchor.to_render(point);
            let theirs = library.transform_point3(Vec3::from_array(point));
            assert!(near(ours, theirs), "{point:?}: {ours:?} vs {theirs:?}");
        }
    }
}

#[test]
fn a_type_1_scene_stands_at_the_identity_like_type_4() {
    assert_eq!(placed(SceneType::Type1, Anchor::facing(Vec3::ONE, 0)), Anchor::IDENTITY);
    let library = scene_to_render(SceneType::Type1, Vec3::ONE, 0);
    assert!(near(library.transform_point3(Vec3::X), Vec3::X), "the library also gives the identity");
}

#[test]
fn a_car_rotation_turns_with_the_anchor_like_its_points() {
    // A turn of an eighth of a circle: the scene's +X is the render direction (cos 45, sin 45, 0).
    let origin = Vec3::new(1.0, 2.0, 3.0);
    let anchor = Anchor::facing(origin, 16384 + 8192);
    let spot = CarSpot::in_scene(&anchor, Vec3::X, Quat::IDENTITY);
    let turned = Vec3::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0);
    assert!(near(spot.position, origin + turned), "{:?}", spot.position);
    assert!(near(spot.rotation * Vec3::X, turned), "{:?}", spot.rotation * Vec3::X);
    let facing_the_axis = Anchor::facing(Vec3::ZERO, 16384);
    assert_eq!(facing_the_axis.turn, 0, "a marker along +x gives no turn");
    assert!((facing_the_axis.rotation() - Quat::IDENTITY).length() < 1e-6);
}
