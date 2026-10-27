//! The plate designs of the converted cars, with the real install: the shared set is written once under `files/PLATES/MW2012/`,
//! every car's plate names its index, the game's own design code draws each design on a car's glyph sheet, and the set is the
//! size the mod promises. Set `MW2012_GAME_DIR` (the 2012 install) and run with `--ignored`. The test also writes
//! `plate-designs-sample.png` (a plate with three designs) into the temporary folder it prints, for a look.

use std::path::{Path, PathBuf};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use blackbox_tpk::read_texture_packs;
use game_install::GameDir;
use nfsmw_2012_content::config::Config;
use nfsmw_data::car::mods::custom::{Plate, plate_design};

fn convert(source: &Path) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-plate-designs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = Config::parse("cars = ['POR911CARR12']\n").unwrap();
    let report = nfsmw_2012_content::import::run(&config, source, &dir, 2).unwrap();
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    let id = report.imported[0].id.clone();
    (dir, id)
}

fn png(path: &Path, width: u32, height: u32, bgra: &[u8]) {
    let rgba: Vec<u8> = bgra.chunks(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&rgba).unwrap();
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_designs_are_written_once_and_the_plate_draws_each_one() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let (dir, id) = convert(Path::new(&source));
    let folder = dir.join("files/PLATES/MW2012");
    let pack_size = std::fs::metadata(folder.join("DESIGNS.BIN")).unwrap().len();
    let index_size = std::fs::metadata(folder.join("DESIGNS.TOML")).unwrap().len();
    println!("{}: pack {} KB, index {} KB", dir.display(), pack_size / 1024, index_size / 1024);
    assert!(pack_size < 6 * 1024 * 1024, "469 pictures of about 8 KB: {pack_size}");

    // The car's manifest names the index, and the index lists a design for each distinct picture.
    let manifest = std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", id.to_ascii_lowercase()))).unwrap();
    let table = parse_car(&id, &manifest).unwrap().customization.unwrap().plate.unwrap();
    assert_eq!(table.designs.as_deref(), Some("PLATES/MW2012/DESIGNS.TOML"));
    let text = std::fs::read_to_string(folder.join("DESIGNS.TOML")).unwrap();
    let set = plate_design::parse_index(table.designs.as_deref().unwrap(), &text).unwrap();
    assert_eq!(set.designs.len(), 469);
    assert_eq!(set.designs[0].name, "START ME UP");
    assert_eq!(set.designs[0].group.as_deref(), Some("Base game"));
    let mut names: Vec<&str> = set.designs.iter().map(|design| design.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 469, "every name is unique");

    // The pack holds the stored pictures: 256 x 64 DXT1.
    let pack = read_texture_packs(&std::fs::read(folder.join("DESIGNS.BIN")).unwrap()).unwrap();
    let textures: usize = pack.iter().map(|pack| pack.textures.len()).sum();
    assert_eq!(textures, 469);

    // The game's plate code draws a design on the car's own sheet.
    let textures = std::fs::read(dir.join("files/CARS").join(&id).join("TEXTURES.BIN")).unwrap();
    let sheet = read_texture_packs(&textures)
        .unwrap()
        .into_iter()
        .flat_map(|pack| pack.textures)
        .find(|texture| texture.name_hash == bstring_hash(&table.picture))
        .expect("the glyph sheet is in the car's textures");
    let plate = Plate {
        text: table.text.clone(),
        picture: table.picture.clone(),
        glyphs: table.glyphs.clone(),
        columns: table.columns,
        rows: table.rows,
        slots: table.slots,
        aliases: table.aliases.clone(),
        designs: table.designs.clone(),
        default_design: table.default_design.clone(),
    };
    let game = GameDir::open(dir.join("files")).unwrap();
    let plain = plate.compose(&sheet, "NEED4SPD").unwrap();
    let mut sample = Vec::new();
    let mut rows = Vec::new();
    for design in [&set.designs[0], &set.designs[9], &set.designs[200]] {
        let ground = plate_design::ground(&game, &plate, &design.id).unwrap();
        let drawn = plate.compose_with(&sheet, &sheet, Some("NEED4SPD"), Some(&ground)).unwrap();
        assert_ne!(drawn.data, plain.data, "{} changes the plate", design.name);
        assert_eq!((drawn.width, drawn.height), (512, 512));
        // The first row of the sheet: the eight cells the plate shows.
        let row_bytes = 512 * 4;
        rows.push(drawn.data[..row_bytes * 128].to_vec());
    }
    rows.push(plain.data[..512 * 4 * 128].to_vec());
    for row in &rows {
        sample.extend_from_slice(row);
    }
    let path = dir.join("plate-designs-sample.png");
    png(&path, 512, 128 * rows.len() as u32, &sample);
    println!("sample: {}", path.display());
}
