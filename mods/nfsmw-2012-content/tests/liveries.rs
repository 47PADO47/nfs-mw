//! The 2012 liveries as variants of one car, with the real install: 93 bundles are 73 cars, and the five vehicles that
//! have liveries offer them (their own bundle's picture and the four others') in the car's own `TEXTURES.BIN`. Set
//! `MW2012_GAME_DIR` (the 2012 install) and run with `--ignored`.

use std::path::Path;

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use blackbox_tpk::{PixelFormat, decode_rgba8_cutout, read_texture_packs};
use nfsmw_2012_content::config::Config;

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR) and writes about 300 MB"]
fn the_five_vehicles_with_liveries_are_five_cars_with_five_liveries_each() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-liveries-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let threads = nfsmw_2012_content::import::default_threads();
    let report = nfsmw_2012_content::import::run(&Config::default(), Path::new(&source), &dir, threads).unwrap();
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    assert_eq!(report.imported.len(), 73, "93 bundles, 20 of them liveries of the other 73 cars' vehicles");

    let with: Vec<_> = report.imported.iter().filter(|car| car.liveries > 0).collect();
    assert_eq!(with.len(), 5, "{with:?}");
    assert!(with.iter().all(|car| car.liveries == 5), "{with:?}");
    let mut names: Vec<&str> = with.iter().map(|car| car.name.as_str()).collect();
    names.sort_unstable();
    println!("cars with liveries: {names:?}");

    // The manifest lists five numbered liveries, and the car's TEXTURES.BIN holds the five pictures as DXT1.
    for car in &with {
        let text =
            std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
        let table = parse_car(&car.id, &text).unwrap().customization.unwrap();
        assert_eq!(table.liveries.len(), 5, "{}", car.id);
        let packs =
            read_texture_packs(&std::fs::read(dir.join("files/CARS").join(&car.id).join("TEXTURES.BIN")).unwrap())
                .unwrap();
        let textures: Vec<_> = packs.iter().flat_map(|pack| pack.textures.iter()).collect();
        let mut clear_texels = Vec::new();
        for (at, livery) in table.liveries.iter().enumerate() {
            assert_eq!(livery.name, format!("Livery {}", at + 1));
            let picture = textures
                .iter()
                .find(|texture| texture.name_hash == bstring_hash(&livery.texture))
                .unwrap_or_else(|| panic!("{}: no texture {}", car.id, livery.texture));
            assert_eq!(picture.format, PixelFormat::Dxt1);
            assert_eq!((picture.width, picture.height), (1024, 512), "cut to the default 1024 limit");
            let rgba = decode_rgba8_cutout(picture).unwrap();
            clear_texels.push(rgba.chunks(4).filter(|px| px[3] == 0).count());
        }
        println!("{}: transparent texels per livery {clear_texels:?}", car.id);
    }

    // Every other car is as it was: no livery key.
    for entry in std::fs::read_dir(dir.join("cars")).unwrap().flatten() {
        let stem = entry.path().file_stem().unwrap().to_string_lossy().into_owned();
        if with.iter().any(|car| car.id.eq_ignore_ascii_case(&stem)) {
            continue;
        }
        let text = std::fs::read_to_string(entry.path()).unwrap();
        assert!(!text.contains("liveries"), "{stem}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
