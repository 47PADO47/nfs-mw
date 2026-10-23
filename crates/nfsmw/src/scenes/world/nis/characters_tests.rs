//! The pure parts of the characters: the record-to-asset matching, the clip's time and the part transform.

use blackbox_anim::Skeleton;
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::{Mat4, Quat, Vec3};

use super::*;

fn skeleton(name: &str, checksum: u16) -> Skeleton {
    Skeleton { name: name.to_owned(), checksum, bones: Vec::new() }
}

/// A solid with no geometry, named `name` (its name hash is the name's `bStringHash`).
fn solid(name: &str) -> Solid {
    Solid {
        name: name.to_owned(),
        name_hash: bstring_hash(name),
        version: 0x16,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
        transform: [0.0; 16],
        num_polys: 0,
        density: 0.0,
        texture_hashes: Vec::new(),
        light_material_hashes: Vec::new(),
        markers: Vec::new(),
        vertex_buffers: Vec::new(),
        vertices: Vec::new(),
        skin: None,
        secondary_uvs: None,
        tangents: None,
        indices: Vec::new(),
        groups: Vec::new(),
    }
}

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-5
}

#[test]
fn a_skeleton_is_found_by_name_then_by_checksum() {
    let skeletons = [skeleton("Bip20", 0x1658), skeleton("Bip24", 0xDBD5)];
    assert_eq!(skeleton_index(&skeletons, bstring_hash("Bip24"), None), Some(1));
    assert_eq!(skeleton_index(&skeletons, bstring_hash("Bip99"), Some(0x1658)), Some(0), "the checksum decides");
    assert_eq!(skeleton_index(&skeletons, bstring_hash("Bip99"), None), None);
    assert_eq!(skeleton_index(&skeletons, bstring_hash("Bip99"), Some(0x0001)), None);
}

#[test]
fn a_model_is_the_first_solid_that_a_lod_names() {
    let solids = [solid("FCOP01"), solid("CUFFS01")];
    let lods = [0, bstring_hash("CUFFS01"), bstring_hash("FCOP01"), 0];
    assert_eq!(solid_index(&solids, lods), Some(1), "LOD 1 is the first LOD that names a solid");
    assert_eq!(solid_index(&solids, [bstring_hash("FCOP01"), 0, 0, 0]), Some(0));
    assert_eq!(solid_index(&solids, [0; 4]), None, "no LOD, no model");
    assert_eq!(solid_index(&solids, [bstring_hash("PERP01"), 0, 0, 0]), None, "a name the bundle lacks");
}

#[test]
fn a_clip_runs_at_half_the_scene_time_after_the_delay() {
    let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
    assert!(close(clip_seconds(2.0, 1.0, 0.0, false, 4.0), 1.0));
    assert!(close(clip_seconds(2.0, 1.0, 1.0, false, 4.0), 0.5), "the delay is taken off first");
    assert!(close(clip_seconds(0.5, 2.0, 0.0, false, 4.0), 0.5), "the play speed multiplies");
    assert!(close(clip_seconds(10.0, 1.0, 0.0, true, 4.0), 1.0), "a looping clip wraps at its length");
    assert!(close(clip_seconds(10.0, 1.0, 0.0, false, 4.0), 5.0), "a clip that does not loop is not wrapped here");
}

#[test]
fn the_scene_to_render_matches_the_anchor() {
    let anchor = Anchor { origin: Vec3::new(5.0, 6.0, 7.0), turn: 16384 };
    let point = Vec3::new(1.0, 0.0, 0.0);
    let expected = anchor.to_render([1.0, 0.0, 0.0]);
    assert!(near(scene_to_render(&anchor).transform_point3(point), expected));
    assert!(near(expected, Vec3::new(5.0, 7.0, 7.0)), "a quarter turn sends +x to +y");
}

#[test]
fn a_part_follows_the_bone_then_the_placement_and_the_lift() {
    let placement = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let bone = Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0));
    let to_render = Mat4::IDENTITY;
    let at = part_transform(to_render, placement, bone).transform_point3(Vec3::ZERO);
    assert!(near(at, Vec3::new(1.0, 2.0, 4.05)), "{at:?}");
    let moved = part_transform(Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)), placement, bone);
    assert!(near(moved.transform_point3(Vec3::ZERO), Vec3::new(11.0, 2.0, 4.05)));
}

#[test]
fn a_rotated_bone_turns_the_part_about_its_joint() {
    let quarter = Mat4::from_quat(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    let at = part_transform(Mat4::IDENTITY, Mat4::IDENTITY, quarter).transform_point3(Vec3::new(1.0, 0.0, 0.0));
    assert!(near(at, Vec3::new(0.0, 1.0, LIFT)), "{at:?}");
}

#[test]
fn an_empty_set_of_characters_adds_nothing() {
    let mut out = Vec::new();
    Characters::default().instances(0.0, &Anchor::IDENTITY, &mut out);
    assert!(out.is_empty());
}
