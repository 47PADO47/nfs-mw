//! A converted car with liveries in the garage, with the real installs: the shop offers its five liveries with the vinyls,
//! and a livery lays its picture over the body paint (where the picture is transparent the paint shows). Set
//! `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use game_install::GameDir;
use nfsmw_2012_content::job::{self, Finished, Start};
use nfsmw_data::car::customization::{Category, Inventory, Selection};
use nfsmw_data::car::mods::custom;
use nfsmw_data::car::mods::{self as car_mods, CarSpec, CarStats};
use nfsmw_data::car::{CarTables, LoadOptions};

/// The 2012 Nissan Skyline R34 GT-R: five bundles, the first with a livery that lets the paint show through.
const CAR: &str = "NISSKYLINE99";

fn converted(source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-livery-garage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = ['{CAR}']\n")).unwrap();
    let Start::Started { job: running, .. } = job::start(&dir).unwrap() else { panic!("the first start converts") };
    let started = Instant::now();
    let finished = loop {
        if let Some(finished) = running.poll() {
            break finished;
        }
        assert!(started.elapsed() < Duration::from_secs(300), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    };
    let Finished::Done(report) = finished else { panic!("the conversion failed") };
    assert_eq!(report.imported.len(), 1, "skipped: {:?}", report.skipped);
    assert_eq!(report.imported[0].id, CAR);
    assert!(job::promote(&dir).unwrap());
    dir
}

fn register(game: &GameDir, mod_dir: &Path) {
    let text =
        std::fs::read_to_string(mod_dir.join("cars").join(format!("{}.toml", CAR.to_ascii_lowercase()))).unwrap();
    let manifest = parse_car(CAR, &text).unwrap();
    game.add_late_overlay("nfsmw-2012-content", mod_dir.join("files"), 0).unwrap();
    let tables = CarTables::load(game).unwrap();
    let spec = CarSpec {
        folder: CAR.to_owned(),
        owner: "nfsmw-2012-content".to_owned(),
        name: manifest.name.clone(),
        manufacturer: manifest.manufacturer.clone(),
        player: true,
        cop: None,
        geometry_in_mod: true,
        stats: CarStats::from(&manifest.stats),
        engine_sound: None,
        car_effects: None,
    };
    let (cars, rejected) = car_mods::register(&tables.types, &[spec]);
    assert!(rejected.is_empty(), "{rejected:?}");
    car_mods::publish(cars);
    let table = manifest.customization.unwrap();
    let finish = |paint: &blackbox_modding::content::Paint| match paint.finish {
        blackbox_modding::content::Finish::Gloss => custom::Finish::Gloss,
        blackbox_modding::content::Finish::Metallic => custom::Finish::Metallic,
        blackbox_modding::content::Finish::Chrome => custom::Finish::Chrome,
    };
    // The 2012 Skyline has one paint; the test adds a second to see the paint through a livery.
    let mut paints: Vec<custom::Paint> = table
        .paints
        .iter()
        .map(|paint| custom::Paint { name: paint.name.clone(), color: paint.color, finish: finish(paint) })
        .collect();
    paints.push(custom::Paint { name: "Test blue".into(), color: [0, 0, 255], finish: custom::Finish::Gloss });
    custom::publish(
        CAR,
        custom::Customization {
            paints,
            default_paint: table.default_paint,
            rim_paints: Vec::new(),
            tints: Vec::new(),
            rims: Vec::new(),
            spoilers: Vec::new(),
            roof_scoops: Vec::new(),
            vinyls: Vec::new(),
            vinyl_colors: Vec::new(),
            liveries: table
                .liveries
                .iter()
                .map(|livery| custom::Livery { name: livery.name.clone(), texture: livery.texture.clone() })
                .collect(),
            default_livery: table.default_livery,
        },
    );
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_livery_lays_its_picture_over_the_body_paint() {
    let (Some(source), Some(root)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let mod_dir = converted(Path::new(&source));
    let game = GameDir::open(Path::new(&root)).unwrap();
    register(&game, &mod_dir);

    let catalog = Inventory::load(&game).unwrap().catalog(&game, CAR).unwrap();
    let liveries = &catalog.choices[Category::Vinyl as usize];
    let labels: Vec<&str> = liveries.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, ["Livery 1", "Livery 2", "Livery 3", "Livery 4", "Livery 5"]);
    assert!(liveries.iter().all(|choice| catalog.vinyl_remaps[&choice.hash] == 0), "no recolourable channel");
    assert_eq!(catalog.choices[Category::Paint as usize].len(), 1, "the added paint is the one choice");

    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let skin = bstring_hash(&format!("{CAR}_SKIN1"));
    let paint = catalog.choices[Category::Paint as usize][0].hash;
    let drawn = |livery: Option<u32>, paint: Option<u32>| {
        let mut selection = Selection::default();
        selection.parts[Category::Vinyl as usize] = livery;
        selection.parts[Category::Paint as usize] = paint;
        catalog.validate(&selection).unwrap();
        let model = nfsmw_data::car::load_customized(&game, CAR, &options, &selection).unwrap();
        model.textures[&skin].clone()
    };

    let flat = drawn(None, None);
    assert_eq!((flat.width, flat.height), (4, 4), "no livery: the flat paint");
    // The first livery has transparent texels: the paint shows through, so another paint changes the skin.
    let (clear_a, clear_b) = (drawn(Some(liveries[0].hash), None), drawn(Some(liveries[0].hash), Some(paint)));
    assert_eq!((clear_a.width, clear_a.height), (1024, 512));
    assert_ne!(clear_a.data, clear_b.data, "the paint shows through the transparent part of the livery");
    // The second is opaque: the paint does not reach it.
    let (solid_a, solid_b) = (drawn(Some(liveries[1].hash), None), drawn(Some(liveries[1].hash), Some(paint)));
    assert_eq!(solid_a.data, solid_b.data, "an opaque livery hides the paint");
    assert_ne!(solid_a.data, clear_a.data, "two liveries are two pictures");
    std::fs::remove_dir_all(&mod_dir).unwrap();
}
