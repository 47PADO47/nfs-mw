use super::*;

fn bc3(width: u32, height: u32) -> Texture {
    let rgba: Vec<u8> =
        (0..width * height).flat_map(|at| [(at % 200) as u8, 90, 160, 255 - (at % 100) as u8]).collect();
    let level = blackbox_tpk::Level { width, height, rgba };
    let data = blackbox_tpk::encode_level(blackbox_tpk::PixelFormat::Dxt5, &level).unwrap();
    Texture { width, height, depth: 1, array_size: 1, mip_levels: 1, dxgi_format: 77, format: PixelFormat::Bc3, data }
}

fn size_of_dds(bytes: &[u8]) -> (u32, u32) {
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    (word(16), word(12))
}

#[test]
fn a_picture_within_the_size_is_written_as_the_install_has_it() {
    let texture = bc3(64, 16);
    let bytes = picture(&texture, 64).unwrap();
    assert_eq!(size_of_dds(&bytes), (64, 16));
    assert_eq!(&bytes[128..], &texture.data[..], "the compressed pixels are copied");
}

#[test]
fn a_wider_picture_is_halved_until_it_fits_and_reads_back_as_a_dds() {
    let texture = bc3(64, 16);
    let bytes = picture(&texture, 16).unwrap();
    assert_eq!(size_of_dds(&bytes), (16, 4));
    let read = blackbox_tpk::read_dds("D", &bytes).unwrap();
    assert_eq!((read.width, read.height, read.format), (16, 4, blackbox_tpk::PixelFormat::Dxt5));
}

#[test]
fn the_files_are_named_in_upper_case() {
    assert_eq!(dome_file("night"), "NIGHT.DDS");
    assert_eq!(profile::replace_manifest(true).matches("SKY/MW2012/MOON.DDS").count(), 1);
}

fn install() -> std::path::PathBuf {
    std::env::var_os("CHAMELEON_GAME_DIR")
        .or_else(|| std::env::var_os("MW2012_GAME_DIR"))
        .map(Into::into)
        .expect("a 2012 install")
}

#[test]
#[ignore = "reads the 2012 install"]
fn the_installed_sky_is_converted_to_a_profile_the_game_reads_with_every_picture() {
    let source = Source::open(&install()).unwrap();
    for size in [2048, 1024] {
        let out = std::env::temp_dir().join(format!("nfsmw-sky-real-{}-{size}", std::process::id()));
        let config = Config { sky_size: size, ..Config::default() };
        let started = std::time::Instant::now();
        write(&source, &config, &out);
        let folder = out.join("files").join(FOLDER);
        let text = std::fs::read_to_string(folder.join("profile.toml")).expect("a profile");
        let profile = blackbox_modding::content::parse_sky_profile(&text).expect("the game reads it");
        assert_eq!((profile.domes.len(), profile.weathers.len()), (6, 3));
        assert!(profile.moon.is_some());
        let mut bytes = 0;
        for dome in &profile.domes {
            let file = std::fs::read(out.join("files").join(&dome.file)).unwrap();
            bytes += file.len();
            let texture = blackbox_tpk::read_dds(&dome.name, &file).expect(&dome.file);
            assert_eq!(texture.width, size, "{}", dome.name);
        }
        let moon = std::fs::read(folder.join(MOON_FILE)).unwrap();
        bytes += moon.len();
        let moon = blackbox_tpk::read_dds("moon", &moon).unwrap();
        assert_eq!((moon.width, moon.height, moon.mip_levels), (64, 64, 7));
        assert!(out.join("replace/sky-mw2012.toml").is_file());
        println!("sky_size {size}: {} KiB of pictures, {} ms", bytes / 1024, started.elapsed().as_millis());
        let _ = std::fs::remove_dir_all(out);
    }
}
