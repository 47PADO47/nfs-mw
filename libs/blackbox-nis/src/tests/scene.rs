use super::build::{character, chunk, container, header};
use crate::{ANIM_SCENE, Error, SCENE_ENTITY, SCENE_HEADER, Scene, SceneType, find_scene};

const HASH: u32 = 0x0A0B_0C0D;

/// A bundle with one scene: a header and two characters, plus a padding child that the reader must ignore.
fn two_character_bundle() -> Vec<u8> {
    let head = chunk(SCENE_HEADER, &header("Alpha", HASH, [2, 1, 1, 2, 0, 240], "GRAPHIC", "Two cars leave the line"));
    let first = chunk(SCENE_ENTITY, &character(1, 0x1111_0001, [1.5, -4.5, 0.0], [0x21, 0x22, 0x23, 0], 0x31, 0x41));
    let second = chunk(SCENE_ENTITY, &character(1, 0x1111_0002, [1.0, -4.0, 0.0], [0x51, 0, 0, 0], 0x61, 0x71));
    container(ANIM_SCENE, &[chunk(0, &[]), head, first, second])
}

fn read_scene(bundle: &[u8]) -> Scene {
    find_scene(bundle).expect("the bundle reads").expect("the bundle has a scene")
}

#[test]
fn reads_the_header_fields() {
    let scene = read_scene(&two_character_bundle());
    assert_eq!(scene.name_hash, HASH);
    assert_eq!(scene.name, "Alpha");
    assert_eq!(scene.description_pointer, 0);
    assert_eq!(scene.scene_type, SceneType::Type2);
    assert!(scene.have_layout);
    assert!(scene.have_car_animation);
    assert_eq!(scene.number_of_cars, 2);
    assert_eq!(scene.start_frame, 0);
    assert_eq!(scene.vanish_frame, 240);
    assert_eq!(scene.overlay_name, "GRAPHIC");
    assert_eq!(scene.description, "Two cars leave the line");
}

#[test]
fn reads_the_characters_in_file_order() {
    let scene = read_scene(&two_character_bundle());
    assert_eq!(scene.characters.len(), 2);
    let first = &scene.characters[0];
    assert_eq!(first.type_id, 1);
    assert_eq!(first.instance_name_hash, 0x1111_0001);
    assert_eq!(first.parent_name_hash, 0);
    assert_eq!(first.play_flags, 0x20);
    assert_eq!(first.local_matrix.w_axis.truncate().to_array(), [1.5, -4.5, 0.0]);
    assert_eq!(first.local_matrix.w_axis.w, 1.0);
    assert_eq!(first.lod_name_hashes, [0x21, 0x22, 0x23, 0]);
    assert_eq!(first.skeletal_anim_hash, 0x31);
    assert_eq!(first.skeleton_name_hash, 0x41);
    assert_eq!(first.play_speed, 1.0);
    assert_eq!(first.play_delay, 0.0);
    let second = &scene.characters[1];
    assert_eq!(second.instance_name_hash, 0x1111_0002);
    assert_eq!(second.local_matrix.w_axis.truncate().to_array(), [1.0, -4.0, 0.0]);
    assert_eq!(second.skeletal_anim_hash, 0x61);
    assert_eq!(second.skeleton_name_hash, 0x71);
}

#[test]
fn finds_the_container_inside_outer_containers() {
    let outer = container(0x8000_1000, &[chunk(0x0003_B200, &[1, 2, 3, 4]), two_character_bundle()]);
    let scene = read_scene(&outer);
    assert_eq!(scene.name, "Alpha");
    assert_eq!(scene.characters.len(), 2);
}

#[test]
fn a_bundle_without_a_scene_has_none() {
    let bundle = chunk(0x0003_B200, &[1, 2, 3, 4]);
    assert_eq!(find_scene(&bundle), Ok(None));
}

#[test]
fn a_scene_without_a_header_is_rejected() {
    let bundle = container(ANIM_SCENE, &[chunk(SCENE_ENTITY, &character(1, 1, [0.0; 3], [0; 4], 0, 0))]);
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(_))));
}

#[test]
fn a_scene_with_two_headers_is_rejected() {
    let head = chunk(SCENE_HEADER, &header("Alpha", HASH, [0, 0, 0, 0, 0, 30], "", ""));
    let bundle = container(ANIM_SCENE, &[head.clone(), head]);
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(_))));
}

#[test]
fn a_short_header_is_truncated() {
    let bundle = container(ANIM_SCENE, &[chunk(SCENE_HEADER, &[0u8; 0x20])]);
    assert_eq!(find_scene(&bundle), Err(Error::Truncated { what: "scene header", offset: 0x20 }));
}

#[test]
fn an_undefined_scene_type_is_rejected() {
    let head = chunk(SCENE_HEADER, &header("Alpha", HASH, [9, 0, 0, 0, 0, 30], "", ""));
    let bundle = container(ANIM_SCENE, &[head]);
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(message)) if message.contains("scene type 9")));
}

#[test]
fn a_flag_other_than_zero_or_one_is_rejected() {
    let head = chunk(SCENE_HEADER, &header("Alpha", HASH, [0, 2, 0, 0, 0, 30], "", ""));
    let bundle = container(ANIM_SCENE, &[head]);
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(message)) if message.contains("HaveLayout")));
}

#[test]
fn a_description_without_a_nul_is_rejected() {
    let mut head = header("Alpha", HASH, [0, 0, 0, 0, 0, 30], "", "");
    head.truncate(0x40);
    head.extend_from_slice(b"abc");
    let bundle = container(ANIM_SCENE, &[chunk(SCENE_HEADER, &head)]);
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(_))));
}

#[test]
fn a_character_record_needs_its_padding() {
    let mut record = character(1, 1, [0.0; 3], [0; 4], 0, 0);
    record[0] = 0;
    let bundle = container(
        ANIM_SCENE,
        &[chunk(SCENE_HEADER, &header("Alpha", HASH, [0, 0, 0, 0, 0, 30], "", "")), chunk(SCENE_ENTITY, &record)],
    );
    assert!(matches!(find_scene(&bundle), Err(Error::Invalid(message)) if message.contains("0x11 padding")));
}

#[test]
fn a_character_record_has_152_bytes() {
    let short = chunk(SCENE_ENTITY, &character(1, 1, [0.0; 3], [0; 4], 0, 0)[..151]);
    let bundle =
        container(ANIM_SCENE, &[chunk(SCENE_HEADER, &header("Alpha", HASH, [0, 0, 0, 0, 0, 30], "", "")), short]);
    assert!(matches!(find_scene(&bundle), Err(Error::Truncated { what: "character record", .. })));
}
