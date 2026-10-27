//! Converts one car from real installs, the way the mod does at start-up (in the background, then promoted), and reads the
//! result back with the game's own readers. Set `MW2012_GAME_DIR` (the 2012 install) (the 2005
//! install) and run with `--ignored`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nfsmw_2012_content::job::{self, Finished, Job, Start};

fn mod_folder(source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = ['POR911CARR12']\n")).unwrap();
    dir
}

fn wait(job: &Job) -> Finished {
    let started = Instant::now();
    loop {
        if let Some(finished) = job.poll() {
            return finished;
        }
        assert!(started.elapsed() < Duration::from_secs(300), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn a_start_converts_in_the_background_and_the_next_start_does_not() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let source = PathBuf::from(source);
    let dir = mod_folder(&source);

    let Start::Started { job: running, live } = job::start(&dir, None).unwrap() else {
        panic!("the first start must convert")
    };
    assert!(!live, "nothing is live on a first start");
    assert!(!dir.join("files").exists(), "nothing is written where the game reads until the run is promoted");
    let Finished::Done(report) = wait(&running) else { panic!("the conversion failed") };
    assert_eq!(report.imported.len(), 1, "skipped: {:?}", report.skipped);
    assert!(job::promote(&dir).unwrap());
    let car = &report.imported[0];

    let folder = dir.join("files/CARS").join(&car.id);
    let geometry = std::fs::read(folder.join("GEOMETRY.BIN")).unwrap();
    let (solids, errors) = blackbox_solid::read_solids_lossy(&geometry).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    for stem in ["KIT00_BODY", "KIT00_FRONT_TIRE"] {
        for letter in ['A', 'B', 'C', 'D'] {
            let name = format!("{}_{stem}_{letter}", car.id);
            assert!(solids.iter().any(|solid| solid.name == name), "no solid named {name}");
        }
    }
    assert!(solids.iter().all(|solid| !solid.groups.is_empty()));

    let textures = std::fs::read(folder.join("TEXTURES.BIN")).unwrap();
    let packs = blackbox_tpk::read_texture_packs(&textures).unwrap();
    assert!(packs.iter().all(|pack| pack.failed.is_empty()));
    assert!(packs.iter().map(|pack| pack.textures.len()).sum::<usize>() > 0);

    let manifest =
        std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
    assert!(!manifest.contains("clone"), "a mod car is not a copy of any car");
    assert!(manifest.contains("[stats]"), "the car has no stats of its own");
    assert!(manifest.contains("[customization]"), "the car has no paints of its own");

    // The hinged parts: the 911 has seven, each its own solid at the two finest levels, cut out of the body.
    let parsed_car = blackbox_modding::content::parse_car(&car.id, &manifest).expect("the game's parser takes it");
    assert_eq!(parsed_car.damage_parts.len(), 8, "bonnet, bumpers, mirrors, front doors and the active spoiler");
    let spoiler = parsed_car.damage_parts.iter().find(|part| part.active.is_some()).expect("the 911's spoiler moves");
    let active = spoiler.active.unwrap();
    assert_eq!((active.above_mph, active.below_mph), (70.0, 40.0));
    assert!(active.offset[2] > 0.0 && active.offset[0] < 0.0, "it goes up and back: {:?}", active.offset);
    let (axis, degrees) = active.rotation.expect("and it turns, as the animation's rotation key says");
    assert!(axis[1].abs() > 0.999 && (degrees.abs() - 30.46).abs() < 0.1, "{axis:?} {degrees}");
    let triangles = |name: &str| {
        let solid = solids.iter().find(|solid| solid.name == name).unwrap_or_else(|| panic!("no solid named {name}"));
        solid.groups.iter().map(|group| group.num_indices as usize / 3).sum::<usize>()
    };
    for letter in ['A', 'B'] {
        let body = triangles(&format!("{}_KIT00_BODY_{letter}", car.id));
        let parts: usize =
            parsed_car.damage_parts.iter().map(|part| triangles(&format!("{}_{letter}", part.model))).sum();
        assert!(
            parts > 1000 && parts < body,
            "level {letter}: {parts} triangles in the parts, {body} left in the body"
        );
    }
    assert_eq!(
        triangles(&format!("{}_KIT00_BODY_A", car.id))
            + parsed_car.damage_parts.iter().map(|p| triangles(&format!("{}_A", p.model))).sum::<usize>(),
        report.imported[0].triangles,
        "cutting the parts out loses no triangle of the finest level"
    );

    // The car's own recordings: the manifest names every file of its engine and its other sounds, and they are all there.
    let parsed: toml::Table = toml::from_str(&manifest).unwrap();
    let mut named = Vec::new();
    for (table, keys) in [("engine_sound", &["layers"][..]), ("car_sound", &["shift", "backfire", "pops"][..])] {
        let table = parsed[table].as_table().unwrap_or_else(|| panic!("the Porsche has a [{table}] table"));
        for key in keys {
            let entries = table[*key].as_array().unwrap_or_else(|| panic!("{key} is a list"));
            assert!(!entries.is_empty(), "{key} names no file");
            named.extend(
                entries.iter().map(|entry| entry.as_str().or_else(|| entry["file"].as_str()).unwrap().to_owned()),
            );
        }
    }
    for file in named {
        let wav = std::fs::read(dir.join("files").join(&file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(&wav[..4] == b"RIFF" || &wav[..4] == b"Gnsu", "{file} is a WAV or a Ginsu file");
    }

    assert!(matches!(job::start(&dir, None).unwrap(), Start::UpToDate { cars: 1 }));
    std::fs::write(dir.join("config.toml"), "source = 'nowhere'\n").unwrap();
    let Start::Started { job: second, live } = job::start(&dir, None).unwrap() else {
        panic!("a changed setting converts")
    };
    assert!(live, "content from the first run is live now, so the new run waits for the next start");
    assert!(matches!(wait(&second), Finished::Failed(_)), "a missing install is a failure, not a crash");
    assert!(dir.join("files").is_dir(), "a failed run leaves the content as it was");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR); prints the time"]
fn every_car_converts_and_how_long_it_takes() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-all-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = nfsmw_2012_content::config::Config::default();
    let mut last = None;
    for threads in [1, 3, nfsmw_2012_content::import::default_threads()] {
        let started = Instant::now();
        let report = nfsmw_2012_content::import::run(&config, Path::new(&source), &dir, threads).unwrap();
        println!("{} cars in {:.2} s on {threads} threads", report.imported.len(), started.elapsed().as_secs_f32());
        last = Some(report);
    }
    let report = last.unwrap();
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    assert!(report.imported.len() > 70, "93 bundles, 20 of them liveries folded into their cars");
    // Every manifest passes the game's checks and carries the car's own paints, and nothing else for its looks.
    let mut paints = 0;
    for entry in std::fs::read_dir(dir.join("cars")).unwrap().flatten() {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let manifest = blackbox_modding::content::parse_car(&stem, &std::fs::read_to_string(&path).unwrap()).unwrap();
        let own = manifest.customization.unwrap_or_else(|| panic!("{stem} has no paints"));
        assert!(own.rims.is_empty() && own.spoilers.is_empty() && own.vinyls.is_empty() && own.tints.is_empty());
        paints += own.paints.len();
        // Every car names the shared sounds, and the files they name are written once.
        let shared = manifest.car_sound.map(|sound| sound.shared).unwrap_or_default();
        assert_eq!((shared.impact_big.len(), shared.scrape.len()), (12, 3), "{stem}");
        let named = [&shared.impact_small, &shared.impact_medium, &shared.impact_big, &shared.scrape, &shared.pops];
        for path in named.into_iter().flatten() {
            assert!(dir.join("files").join(path).is_file(), "{stem}: {path} is named and not written");
        }
    }
    println!("{paints} paints in {} cars", report.imported.len());
    std::fs::remove_dir_all(&dir).unwrap();
}
