//! A converted car in the garage, with the real installs: it is offered the paints of its own 2012 bundle and nothing of
//! a stock car's parts, it has the pipes the nitrous flames come out of, and it is built from its own `[stats]`. Set
//! `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use game_install::GameDir;
use nfsmw_2012_content::job::{self, Finished, Start};
use nfsmw_data::car::customization::{Category, Inventory, Selection};
use nfsmw_data::car::mods::custom;
use nfsmw_data::car::mods::{self as car_mods, CarSpec, CarStats, EngineLayer, EngineSound};
use nfsmw_data::car::physics::{CarBounds, PerformancePackage, PhysicsData, physics_from_stats};
use nfsmw_data::car::{CarTables, LoadOptions, exhaust, slot};

const CAR: &str = "POR911CARR12";

fn converted(source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-customize-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    let text =
        format!("source = '{source}'\ncars = ['{CAR}']\n\n[performance]\npower = 1.2\n\n[car.{CAR}]\nbrakes = 1.3\n");
    std::fs::write(dir.join("config.toml"), text).unwrap();
    let Start::Started { job: running, .. } = job::start(&dir, None).unwrap() else {
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
            .map(|layer| EngineLayer::classic(layer["file"].as_str().unwrap(), layer.get("rpm").map_or(0.0, number)))
            .collect(),
    }
}

fn register(game: &GameDir, mod_dir: &Path) -> CarStats {
    let text =
        std::fs::read_to_string(mod_dir.join("cars").join(format!("{}.toml", CAR.to_ascii_lowercase()))).unwrap();
    let manifest: toml::Table = toml::from_str(&text).unwrap();
    // The game's own parser checks the `[stats]` table; the car is registered with the stats it reads.
    let stats = CarStats::from(&parse_car(CAR, &text).unwrap().stats);
    game.add_late_overlay("nfsmw-2012-content", mod_dir.join("files"), 0).unwrap();
    let tables = CarTables::load(game).unwrap();
    let spec = CarSpec {
        folder: CAR.to_owned(),
        owner: "nfsmw-2012-content".to_owned(),
        name: manifest["name"].as_str().map(str::to_owned),
        manufacturer: manifest.get("manufacturer").and_then(|brand| brand.as_str()).map(str::to_owned),
        player: true,
        cop: None,
        geometry_in_mod: true,
        stats: stats.clone(),
        engine_sound: manifest.get("engine_sound").map(engine_sound_of),
        car_effects: None,
    };
    let (cars, rejected) = car_mods::register(&tables.types, &[spec]);
    assert!(rejected.is_empty(), "{rejected:?}");
    car_mods::publish(cars);
    custom::publish(CAR, customization_of(&text));
    stats
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
        liveries: table
            .liveries
            .iter()
            .map(|livery| custom::Livery { name: livery.name.clone(), texture: livery.texture.clone() })
            .collect(),
        default_livery: table.default_livery,
    }
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_converted_car_takes_its_own_paints_flames_and_tuning() {
    let (Some(source), Some(root)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let mod_dir = converted(Path::new(&source));
    let game = GameDir::open(Path::new(&root)).unwrap();
    let stats = register(&game, &mod_dir);
    // The 2012 Porsche 911 Carrera S has 440 N m at 5600 rpm and 1300 N m brakes at the front; the settings of this test
    // multiply them by 1.2 and 1.3. Its nitrous is the 2012 car's.
    assert!((stats.peak_torque_nm() - 440.0 * 1.2).abs() < 0.5, "{}", stats.peak_torque_nm());
    assert!((stats.brake_torque_nm[0] - 1300.0 * 1.3).abs() < 1.0, "{:?}", stats.brake_torque_nm);
    assert!(stats.nitrous && (stats.nitrous_boost - 0.9).abs() < 1e-3);
    assert_eq!((stats.gear_ratios.len(), stats.final_drive), (7, 3.44));

    let tables = CarTables::load(&game).unwrap();
    let car_type = tables.car_type_for_folder(CAR).unwrap().clone();
    assert_eq!(car_type.type_name, CAR, "the car is its own table entry");

    // The body has tail pipes of its own (the markers the nitrous flames come out of).
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let stock = nfsmw_data::car::load_customized(&game, CAR, &options, &Selection::default()).unwrap();
    let pipes = exhaust::pipe_frames(&stock.placements, &stock.solids);
    assert!(!pipes.is_empty(), "no tail pipe marker: the nitrous flames have nowhere to start");

    // The car is assembled from its own solids: a body, and its one wheel solid at all four corners.
    let own = |slot: usize| stock.placements.iter().filter(move |p| p.slot == slot).count();
    assert_eq!((own(slot::BODY), own(slot::FRONT_WHEEL), own(slot::REAR_WHEEL)), (1, 2, 2), "{:?}", stock.placements);
    assert_eq!(stock.placements.len(), 5, "nothing is borrowed from a stock car's parts");
    assert!(stock.corners.is_some() && stock.wheel_setup.is_some());
    let wheel = stock.placements.iter().find(|p| p.slot == slot::FRONT_WHEEL).unwrap().solid;
    assert_eq!(wheel, bstring_hash(&format!("{CAR}_KIT00_FRONT_TIRE_A")));
    // The same car is there at the coarser levels the game draws at a distance, and for driving.
    let driving = nfsmw_data::car::load_driving(&game, CAR, &options).unwrap();
    let levels: Vec<usize> = driving.driving_lods.levels.iter().map(|l| l.level).collect();
    assert_eq!(levels, [1, 2, 3], "levels B to D");
    // Body, driver and four wheels: the driver is drawn while the car is driven.
    assert!(driving.driving_lods.levels.iter().all(|l| l.placements.len() == 6));
    println!("type names: {}", tables.parts.type_names().len());

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

    // The host builds the car from its own stats and its own BOUNDS.BIN: no vehicle record is read.
    let physics = PhysicsData::load(&game).unwrap();
    let built = physics.car_with_performance(CAR, PerformancePackage::default()).unwrap();
    assert_eq!(built.spec.mass, stats.mass_kg);
    assert!(built.geometry.is_some() && built.events.is_some(), "the car's own contact shapes and damage");
    assert_eq!(built.spec.dimension, built.bounds.half_dimensions);
    let own_box = built.bounds.half_dimensions;
    assert!(
        (0.7..1.3).contains(&own_box.x) && (0.4..1.0).contains(&own_box.y) && (1.4..3.0).contains(&own_box.z),
        "{own_box}"
    );

    // The physics are built from the car's own stats: the box is the caller's, the rest is the 2012 car's.
    let bounds = CarBounds { half_dimensions: glam::Vec3::new(0.91, 0.55, 2.21), pivot: glam::Vec3::ZERO };
    let stock = physics_from_stats(car_mods::stats(CAR).unwrap(), PerformancePackage::default(), bounds);
    assert_eq!(stock.spec.mass, stats.mass_kg);
    assert!(stock.spec.nos.present(), "{:?}", stock.spec.nos);
    assert_eq!(stock.spec.transmission.gear_ratio[2], 3.91);
    assert!((stock.spec.engine.torque_nm(5600.0) - 440.0 * 1.2).abs() < 10.0);
    let tuned = physics_from_stats(
        car_mods::stats(CAR).unwrap(),
        PerformancePackage { levels: [0, 0, 0, 0, 3, 0, 0], junkman: 0 },
        bounds,
    );
    assert!(tuned.spec.engine.torque_nm(5600.0) > stock.spec.engine.torque_nm(5600.0) * 1.1);
    // The car has engine loops of its own: WAV files in the mod's layer of the install, slowest first.
    let engine = car_mods::engine_sound(CAR).expect("the manifest names engine loops");
    println!("engine loops: {:?}", engine.layers);
    assert!(engine.idle_rpm > 0.0 && engine.redline_rpm > engine.idle_rpm);
    for layer in &engine.layers {
        let wav = game.read(&layer.file).unwrap();
        assert!(&wav[..4] == b"RIFF" || &wav[..4] == b"Gnsu", "{}: a WAV or a Ginsu file", layer.file);
    }
    // The layers mixed by engine speed run slowest first; the ones with curves have no engine speed of their own.
    let by_speed: Vec<f32> = engine.layers.iter().map(|layer| layer.rpm).filter(|rpm| *rpm > 0.0).collect();
    assert!(by_speed.windows(2).all(|pair| pair[0] < pair[1]));
    std::fs::remove_dir_all(&mod_dir).unwrap();
}
