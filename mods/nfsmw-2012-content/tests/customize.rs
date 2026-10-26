//! A converted car in the garage, with the real installs: it is offered the paints of its own 2012 bundle and nothing of
//! a stock car's parts, it has the pipes the nitrous flames come out of, and it drives with the mod's `[performance]`. Set
//! `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blackbox_hash::bstring_hash;
use game_install::GameDir;
use nfsmw_2012_content::job::{self, Finished, Start};
use nfsmw_data::car::customization::{Category, Inventory, Selection};
use nfsmw_data::car::mods::custom;
use nfsmw_data::car::mods::{self as car_mods, CarSpec, EngineLayer, EngineSound, Tuning};
use nfsmw_data::car::physics::{PerformancePackage, PhysicsData};
use nfsmw_data::car::{CarTables, LoadOptions, exhaust};

const CAR: &str = "POR911CARR12";

fn converted(source: &Path, game: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-customize-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    let text =
        format!("source = '{source}'\ncars = ['{CAR}']\n\n[performance]\npower = 1.2\n\n[car.{CAR}]\nbrakes = 1.3\n");
    std::fs::write(dir.join("config.toml"), text).unwrap();
    let Start::Started { job: running, .. } = job::start(&dir, game).unwrap() else {
        panic!("the first start converts")
    };
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
    assert!(job::promote(&dir).unwrap());
    dir
}

/// The car the way the host registers it from `cars/<id>.toml`.
/// The manifest's `[engine_sound]` table as the game reads it.
fn engine_sound_of(table: &toml::Value) -> EngineSound {
    let number = |value: &toml::Value| value.as_float().unwrap() as f32;
    let layers = table["layers"].as_array().unwrap();
    EngineSound {
        idle_rpm: number(&table["idle_rpm"]),
        redline_rpm: number(&table["redline_rpm"]),
        layers: layers
            .iter()
            .map(|layer| EngineLayer { file: layer["file"].as_str().unwrap().to_owned(), rpm: number(&layer["rpm"]) })
            .collect(),
    }
}

fn register(game: &GameDir, mod_dir: &Path) -> Tuning {
    let text =
        std::fs::read_to_string(mod_dir.join("cars").join(format!("{}.toml", CAR.to_ascii_lowercase()))).unwrap();
    let manifest: toml::Table = toml::from_str(&text).unwrap();
    let performance = manifest["performance"].as_table().unwrap();
    let number = |key: &str| performance[key].as_float().unwrap() as f32;
    let tuning = Tuning {
        power: number("power"),
        mass: number("mass"),
        grip: number("grip"),
        brakes: number("brakes"),
        nitrous: performance["nitrous"].as_bool().unwrap(),
        nitrous_power: number("nitrous_power"),
    };
    game.add_late_overlay("nfsmw-2012-content", mod_dir.join("files"), 0).unwrap();
    let tables = CarTables::load(game).unwrap();
    let spec = CarSpec {
        folder: CAR.to_owned(),
        owner: "nfsmw-2012-content".to_owned(),
        clone: manifest["clone"].as_str().unwrap().to_owned(),
        name: manifest["name"].as_str().map(str::to_owned),
        manufacturer: manifest.get("manufacturer").and_then(|brand| brand.as_str()).map(str::to_owned),
        player: true,
        cop_heat: None,
        geometry_in_mod: true,
        tuning,
        engine_sound: manifest.get("engine_sound").map(engine_sound_of),
    };
    let (cars, rejected) = car_mods::register(&tables.types, &[spec]);
    assert!(rejected.is_empty(), "{rejected:?}");
    car_mods::publish(cars);
    custom::publish(CAR, customization_of(&text));
    tuning
}

/// The manifest's `[customization]` table as the game registers it (paints only: that is all the 2012 cars have).
fn customization_of(text: &str) -> custom::Customization {
    let manifest = blackbox_modding::content::parse_car(&CAR.to_ascii_lowercase(), text).unwrap();
    let table = manifest.customization.expect("the manifest has a [customization] table");
    let finish = |paint: &blackbox_modding::content::Paint| match paint.finish {
        blackbox_modding::content::Finish::Gloss => custom::Finish::Gloss,
        blackbox_modding::content::Finish::Metallic => custom::Finish::Metallic,
        blackbox_modding::content::Finish::Chrome => custom::Finish::Chrome,
    };
    custom::Customization {
        paints: table
            .paints
            .iter()
            .map(|paint| custom::Paint { name: paint.name.clone(), color: paint.color, finish: finish(paint) })
            .collect(),
        default_paint: table.default_paint,
        rim_paints: Vec::new(),
        tints: Vec::new(),
        rims: Vec::new(),
        spoilers: Vec::new(),
        roof_scoops: Vec::new(),
        vinyls: Vec::new(),
        vinyl_colors: Vec::new(),
    }
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_converted_car_takes_its_own_paints_flames_and_tuning() {
    let (Some(source), Some(root)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let mod_dir = converted(Path::new(&source), Path::new(&root));
    let game = GameDir::open(Path::new(&root)).unwrap();
    let tuning = register(&game, &mod_dir);
    assert_eq!((tuning.power, tuning.brakes, tuning.nitrous), (1.2, 1.3, true));

    let tables = CarTables::load(&game).unwrap();
    let car_type = tables.car_type_for_folder(CAR).unwrap().clone();

    // The body has tail pipes of its own (the markers the nitrous flames come out of).
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let stock = nfsmw_data::car::load_customized(&game, CAR, &options, &Selection::default()).unwrap();
    let pipes = exhaust::pipe_frames(&stock.placements, &stock.solids);
    assert!(!pipes.is_empty(), "no tail pipe marker: the nitrous flames have nowhere to start");

    // The garage offers the car the paints of its own bundle, and none of the parts a stock car has.
    let inventory = Inventory::load(&game).unwrap();
    let own = inventory.catalog(&game, CAR).unwrap();
    for category in Category::ALL {
        println!("{:<24} {:>3}", category.label(), own.choices[category as usize].len());
    }
    let expected = custom::of(CAR).unwrap().paints.len() - 1;
    assert_eq!(own.choices[Category::Paint as usize].len(), expected, "the default paint is the stock one");
    for category in [Category::Spoiler, Category::Roof, Category::Vinyl, Category::Wheels, Category::Body] {
        assert!(own.choices[category as usize].is_empty(), "{}: the 2012 game has none", category.label());
    }

    // A paint changes the colour of the body's paint texture.
    let skin = bstring_hash(&format!("{CAR}_SKIN1"));
    assert!(stock.textures.contains_key(&skin), "the body's paint placeholder is the car's own");
    if let Some(paint) = own.choices[Category::Paint as usize].first() {
        let mut selection = Selection::default();
        selection.parts[Category::Paint as usize] = Some(paint.hash);
        let painted = nfsmw_data::car::load_customized(&game, CAR, &options, &selection).unwrap();
        assert_ne!(
            stock.textures[&skin].data, painted.textures[&skin].data,
            "the paint did not reach the body texture"
        );
    }

    // The tuning is on top of the clone's physics, and the car has nitrous.
    let physics = PhysicsData::load(&game).unwrap();
    let base = physics.car(&car_type.type_name).unwrap();
    let tuned = physics.car(CAR).unwrap();
    let ratio = tuned.spec.engine.torque[3] / base.spec.engine.torque[3];
    assert!((ratio - 1.2).abs() < 1e-4, "torque ratio {ratio}");
    assert!(tuned.spec.nos.present(), "{:?}", tuned.spec.nos);
    println!("nitrous {:?}, the clone's {:?}", tuned.spec.nos, base.spec.nos);
    let upgraded = physics.car_with_performance(CAR, PerformancePackage::default()).unwrap();
    assert_eq!(format!("{:?}", upgraded.spec.engine.torque), format!("{:?}", tuned.spec.engine.torque));
    // The car has engine loops of its own: WAV files in the mod's layer of the install, slowest first.
    let engine = car_mods::engine_sound(CAR).expect("the manifest names engine loops");
    println!("engine loops: {:?}", engine.layers);
    assert!(engine.idle_rpm > 0.0 && engine.redline_rpm > engine.idle_rpm);
    for layer in &engine.layers {
        let wav = game.read(&layer.file).unwrap();
        assert_eq!(&wav[..4], b"RIFF", "{}", layer.file);
    }
    assert!(engine.layers.windows(2).all(|pair| pair[0].rpm < pair[1].rpm));
    std::fs::remove_dir_all(&mod_dir).unwrap();
}
