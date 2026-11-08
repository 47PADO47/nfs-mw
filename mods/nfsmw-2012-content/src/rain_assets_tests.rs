use super::*;

/// A 64 x 64 picture with a bright round dot of `radius` at `(cx, cy)` and a smaller one.
fn dots(radius: f32) -> Picture {
    let mut rgba = vec![0u8; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let big = ((x as f32 - 40.0).powi(2) + (y as f32 - 24.0).powi(2)).sqrt() <= radius;
            let small = ((x as f32 - 10.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt() <= 4.0;
            if big || small {
                rgba[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&[240, 240, 240, 255]);
            }
        }
    }
    Picture { width: 64, height: 64, rgba }
}

#[test]
fn the_largest_dot_becomes_the_lens_drop_bending_the_scene_more_at_its_rim() {
    let drop = lens_drop(&dots(12.0)).unwrap();
    assert_eq!((drop.width, drop.height), (DROP_SIZE, DROP_SIZE));
    let at = |x: u32, y: u32| {
        let i = ((y * DROP_SIZE + x) * 4) as usize;
        [drop.rgba[i], drop.rgba[i + 1], drop.rgba[i + 2], drop.rgba[i + 3]]
    };
    let centre = at(32, 32);
    assert!(
        (i32::from(centre[1]) - 128).abs() <= 4 && (i32::from(centre[2]) - 128).abs() <= 4,
        "no bend at the centre"
    );
    assert!(centre[3] > 200, "the body is opaque");
    let right = at(58, 32);
    assert!(right[1] < 100, "the right rim reads further left: {right:?}");
    let bottom = at(32, 58);
    assert!(bottom[2] < 100, "the bottom rim reads further up: {bottom:?}");
    assert_eq!(at(0, 0)[3], 0, "outside the drop there is nothing");
}

#[test]
fn a_picture_with_no_dot_is_refused() {
    let empty = Picture { width: 8, height: 8, rgba: vec![0; 8 * 8 * 4] };
    assert!(lens_drop(&empty).is_err());
}

#[test]
fn the_profiles_and_the_set_are_accepted_by_the_game() {
    let splash = blackbox_modding::content::parse_rain_splash_profile(&splash_profile_text()).unwrap();
    assert_eq!(splash.layers.len(), 2);
    assert!(splash.layers[0].texture.starts_with("RAINSPLASH/MW2012/"));
    let wet = blackbox_modding::content::parse_wet_road_profile(wet_road_profile_text()).unwrap();
    assert!(wet.film > 1.0 && wet.dry < 1.0);
    let set = blackbox_modding::content::parse_replace_set("rain-drops-mw2012", &drops_manifest()).unwrap();
    assert_eq!(set.kind, "rain_drops");
}

#[test]
fn the_files_are_written_where_the_profiles_name_them() {
    let out = std::env::temp_dir().join(format!("nfsmw-rain-{}", std::process::id()));
    let pictures = RainPictures { ring: dots(10.0), spray: dots(10.0), drop: lens_drop(&dots(12.0)).unwrap() };
    write_splash(&out, &pictures).unwrap();
    write_drops(&out, &pictures).unwrap();
    write_text(&out, WET_ROAD_FOLDER, "profile.toml", wet_road_profile_text()).unwrap();
    for path in [
        "files/RAINSPLASH/MW2012/RING.PNG",
        "files/RAINSPLASH/MW2012/SPRAY.PNG",
        "files/RAINSPLASH/MW2012/profile.toml",
        "files/REPLACE/RAIN_DROPS/RAINDROPPC.png",
        "files/WETROAD/MW2012/profile.toml",
        "replace/rain-drops-mw2012.toml",
    ] {
        assert!(out.join(path).is_file(), "{path}");
    }
    let _ = std::fs::remove_dir_all(&out);
}

/// The real pictures: a ring, a spray and a drop made from the round dots, read by ranges.
#[test]
#[ignore = "requires the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_gives_the_three_pictures_for_a_few_kib() {
    let root = std::env::var_os("NFSMW2012_DIR")
        .map_or_else(|| Path::new(r"D:\need for Speed - Most Wanted").to_owned(), Into::into);
    let source = Source::open(&root).unwrap();
    let (pictures, read) = find(&source).unwrap();
    println!("rain pictures: read {read} bytes");
    if let Some(dump) = std::env::var_os("FX_DUMP") {
        write_splash(Path::new(&dump), &pictures).unwrap();
        write_drops(Path::new(&dump), &pictures).unwrap();
    }
    assert_eq!((pictures.ring.width, pictures.spray.width), (64, 256));
    assert!(pictures.drop.rgba.chunks_exact(4).filter(|pixel| pixel[3] > 128).count() > 500);
    assert!(read < 400 * 1024, "read {read} bytes");
}
