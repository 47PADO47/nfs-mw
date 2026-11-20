use super::*;

fn clouds(texture: u32, lit: f32, ambient: f32) -> Clouds {
    Clouds { texture, lit: [lit, lit, lit, 1.0], ambient: [ambient, ambient, ambient, 1.0], params: [0.0; 7] }
}

fn day() -> Timeline {
    let key = |hour, texture| Key { hour, keyframe: texture, clouds: clouds(texture, 1.0, 0.5) };
    Timeline { id: 1, keys: vec![key(0.0, 10), key(5.0, 20), key(12.0, 30), key(21.0, 40)] }
}

#[test]
fn the_tint_is_the_mean_of_the_two_colours() {
    assert_eq!(clouds(1, 1.0, 0.5).tint(), [0.75; 3]);
}

#[test]
fn around_finds_the_keys_before_and_after_and_how_far_between() {
    let timeline = day();
    let (a, b, t) = timeline.around(8.5).unwrap();
    assert_eq!((a.hour, b.hour), (5.0, 12.0));
    assert!((t - 0.5).abs() < 1e-6);
    let (a, b, t) = timeline.around(5.0).unwrap();
    assert_eq!((a.hour, b.hour, t), (5.0, 12.0, 0.0));
}

#[test]
fn around_wraps_past_midnight() {
    let timeline = day();
    let (a, b, t) = timeline.around(22.5).unwrap();
    assert_eq!((a.hour, b.hour), (21.0, 0.0));
    assert!((t - 0.5).abs() < 1e-6, "{t}");
    let (a, b, _) = timeline.around(24.0 + 1.0).unwrap();
    assert_eq!((a.hour, b.hour), (0.0, 5.0));
}

#[test]
fn a_timeline_of_one_key_holds_it_and_an_empty_one_has_none() {
    let key = Key { hour: 7.0, keyframe: 1, clouds: clouds(1, 1.0, 1.0) };
    let one = Timeline { id: 1, keys: vec![key] };
    let (a, b, t) = one.around(3.0).unwrap();
    assert_eq!((a.hour, b.hour, t), (7.0, 7.0, 0.0));
    assert!(Timeline { id: 2, keys: vec![] }.around(3.0).is_none());
}

#[test]
fn numbered_ids_carry_the_class_byte() {
    assert_eq!(numbered(0x3F3C), 0x0100_0000_0000_3F3C);
}

fn install_bundle() -> Vec<u8> {
    let root =
        std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR")).expect("a game dir");
    std::fs::read(std::path::Path::new(&root).join(BUNDLE_PATH)).expect("ENVIRONMENT.BNDL")
}

#[test]
#[ignore = "reads the 2012 install"]
fn the_installed_timelines_name_the_dome_slots_of_the_day() {
    let data = install_bundle();
    let bundle = Bundle::parse(&data).unwrap();
    let timelines = timelines(&bundle).unwrap();
    assert_eq!(timelines.len(), 25);
    let standard = timelines.iter().find(|t| t.id == 0x69CBB).expect("the first timeline");
    assert_eq!(standard.keys.len(), 21);
    let dome = |hour: f32| standard.around(hour).unwrap().0.clouds.texture;
    assert_eq!([dome(2.0), dome(8.0), dome(13.0), dome(18.0), dome(22.0)], [0x3F34, 0x3F3C, 0x3F46, 0x3F40, 0x3F3E]);
    let fair = timelines.iter().find(|t| t.id == 0x124583).expect("the fair timeline");
    assert_eq!(fair.around(15.0).unwrap().0.clouds.texture, 0x3F3A);
    let noon = standard.around(12.0).unwrap().0.clouds;
    assert!(noon.lit[2] > noon.lit[0], "noon clouds are lit bluish: {noon:?}");
    for timeline in &timelines {
        for key in &timeline.keys {
            let picture = texture(&bundle, key.clouds.texture).unwrap();
            assert!(picture.width >= 1024 && picture.height >= 256, "{} {:X}", picture.width, key.clouds.texture);
        }
    }
}

#[test]
#[ignore = "reads the 2012 install"]
fn the_installed_moon_is_a_64_pixel_picture_with_mips() {
    let data = install_bundle();
    let bundle = Bundle::parse(&data).unwrap();
    let key = moon_texture(&bundle).unwrap();
    assert_eq!(key, 0x3F4A);
    let picture = texture(&bundle, key).unwrap();
    assert_eq!((picture.width, picture.height, picture.mip_levels), (64, 64, 7));
}
