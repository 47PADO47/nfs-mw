//! A converted car in the garage, with the real installs: it can be painted and given a vinyl of its clone, it has the
//! mounts for spoilers and the pipes the nitrous flames come out of, and it is built from its own `[stats]`. Set
//! `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use game_install::GameDir;
use nfsmw_2012_content::job::{self, Finished, Start};
use nfsmw_data::car::customization::{Category, Inventory, Selection};
use nfsmw_data::car::mods::{self as car_mods, CarSpec, CarStats, EngineLayer, EngineSound};
use nfsmw_data::car::physics::{CarBounds, PerformancePackage, physics_from_stats};
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
        clone: manifest["clone"].as_str().unwrap().to_owned(),
        name: manifest["name"].as_str().map(str::to_owned),
        manufacturer: manifest.get("manufacturer").and_then(|brand| brand.as_str()).map(str::to_owned),
        player: true,
        cop_heat: None,
        geometry_in_mod: true,
        stats: stats.clone(),
        engine_sound: manifest.get("engine_sound").map(engine_sound_of),
    };
    let (cars, rejected) = car_mods::register(&tables.types, &[spec]);
    assert!(rejected.is_empty(), "{rejected:?}");
    car_mods::publish(cars);
    stats
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_converted_car_takes_paint_vinyls_spoilers_flames_and_tuning() {
    let (Some(source), Some(root)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let mod_dir = converted(Path::new(&source), Path::new(&root));
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
    let clone_folder = car_mods::clone_folder(CAR).unwrap();
    println!("{CAR} clones {} (folder {clone_folder})", car_type.type_name);

    // The mounts and pipes of the clone are on the body.
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let stock = nfsmw_data::car::load_customized(&game, CAR, &options, &Selection::default()).unwrap();
    let pipes = exhaust::pipe_frames(&stock.placements, &stock.solids);
    assert!(!pipes.is_empty(), "no tail pipe marker: the nitrous flames have nowhere to start");

    // What the garage offers the car is what it offers the clone, vinyls included.
    let inventory = Inventory::load(&game).unwrap();
    let own = inventory.catalog(&game, CAR).unwrap();
    let clone = inventory.catalog(&game, clone_folder).unwrap();
    for category in Category::ALL {
        println!(
            "{:<24} car {:>3}  clone {:>3}",
            category.label(),
            own.choices[category as usize].len(),
            clone.choices[category as usize].len()
        );
    }
    assert!(!own.choices[Category::Paint as usize].is_empty(), "no paint finish");
    assert!(!own.choices[Category::Spoiler as usize].is_empty(), "no spoiler: the body has no SPOILER marker");
    assert_eq!(own.choices[Category::Vinyl as usize].len(), clone.choices[Category::Vinyl as usize].len());

    // A vinyl changes the paint texture the body is drawn with.
    let skin = bstring_hash(&format!("{}_SKIN1", car_type.base_model_name.to_ascii_uppercase()));
    let Some(vinyl) = own.choices[Category::Vinyl as usize].first() else {
        println!("the clone has no vinyls, so none is checked");
        return;
    };
    let mut selection = Selection::default();
    selection.parts[Category::Vinyl as usize] = Some(vinyl.hash);
    let painted = nfsmw_data::car::load_customized(&game, CAR, &options, &selection).unwrap();
    assert_ne!(stock.textures[&skin].data, painted.textures[&skin].data, "the vinyl did not reach the body texture");

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
        assert_eq!(&wav[..4], b"RIFF", "{}", layer.file);
    }
    assert!(engine.layers.windows(2).all(|pair| pair[0].rpm < pair[1].rpm));
    std::fs::remove_dir_all(&mod_dir).unwrap();
}
