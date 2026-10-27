//! Converts cars of brands the 2005 game does not have from the real 2012 install and checks the manufacturers the mod
//! writes for them: a manifest the game's parser accepts and a PNG of the logo canvas with something drawn on it. Set
//! `MW2012_GAME_DIR` (the 2012 install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nfsmw_2012_content::job::{self, Finished, Start};

/// Brands that only the 2012 game has (a car of each is asked for by name fragment).
const NEW_BRANDS: [&str; 3] = ["BAC", "Bentley", "Koenigsegg"];

fn mod_folder(source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-brands-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    let cars = NEW_BRANDS.iter().map(|brand| format!("'{brand}'")).collect::<Vec<_>>().join(", ");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = [{cars}, 'Porsche']\n")).unwrap();
    dir
}

fn read_png(path: &Path) -> (u32, u32, png::ColorType, png::BitDepth, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(std::fs::read(path).unwrap())).read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    data.truncate(info.buffer_size());
    (info.width, info.height, info.color_type, info.bit_depth, data)
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn brands_the_2005_game_lacks_get_a_manufacturer_with_their_2012_logo() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let dir = mod_folder(&PathBuf::from(source));
    let Start::Started { job: running, .. } = job::start(&dir, None).unwrap() else {
        panic!("the first start must convert")
    };
    let started = Instant::now();
    let report = loop {
        if let Some(finished) = running.poll() {
            let Finished::Done(report) = finished else { panic!("the conversion failed") };
            break report;
        }
        assert!(started.elapsed() < Duration::from_secs(600), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(job::promote(&dir).unwrap());
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);

    let mut brands = Vec::new();
    for entry in std::fs::read_dir(dir.join("manufacturers")).unwrap().flatten() {
        let path = entry.path();
        let id = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let manifest =
            blackbox_modding::content::parse_manufacturer(&id, &std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!manifest.overriding, "the 2012 mod never replaces a logo the game has");
        let (width, height, color, depth, pixels) = read_png(&dir.join("files").join(&manifest.logo));
        assert_eq!((width, height), (300, 150), "{id}: a 2:1 canvas");
        assert_eq!((color, depth), (png::ColorType::Rgba, png::BitDepth::Eight));
        let drawn = pixels.chunks(4).filter(|p| p[3] > 128).count();
        assert!(drawn > 500, "{id}: the logo is drawn ({drawn} opaque pixels)");
        let corner = &pixels[..4];
        assert_eq!(corner[3], 0, "{id}: the canvas is transparent around the logo");
        brands.push(manifest.name);
    }
    let has = |brand: &str| brands.iter().any(|name| name.eq_ignore_ascii_case(brand));
    for brand in NEW_BRANDS {
        let converted = report.imported.iter().any(|car| car.name.to_lowercase().starts_with(&brand.to_lowercase()));
        assert!(!converted || has(brand), "a {brand} was converted and has no manufacturer: {brands:?}");
    }
    assert!(has("Porsche"), "a brand the 2005 game has is written too: {brands:?}");
    assert!(report.imported.iter().any(|car| car.name.starts_with("BAC")), "no BAC car matched");
    println!("brands written: {brands:?}\nin {}", dir.display());
}
