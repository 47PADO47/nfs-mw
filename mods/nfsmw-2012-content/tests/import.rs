//! Converts one car from real installs, the way the mod does at start-up (in the background, then promoted), and reads the
//! result back with the game's own readers. Set `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005
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
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_start_converts_in_the_background_and_the_next_start_does_not() {
    let (Some(source), Some(game)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let (source, game) = (PathBuf::from(source), PathBuf::from(game));
    let dir = mod_folder(&source);

    let Start::Started { job: running, live } = job::start(&dir, &game).unwrap() else {
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
            let suffix = format!("_{stem}_{letter}");
            assert!(solids.iter().any(|solid| solid.name.ends_with(&suffix)), "no solid ending {suffix}");
        }
    }
    assert!(solids.iter().all(|solid| !solid.groups.is_empty()));

    let textures = std::fs::read(folder.join("TEXTURES.BIN")).unwrap();
    let packs = blackbox_tpk::read_texture_packs(&textures).unwrap();
    assert!(packs.iter().all(|pack| pack.failed.is_empty()));
    assert!(packs.iter().map(|pack| pack.textures.len()).sum::<usize>() > 0);

    let manifest =
        std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
    assert!(manifest.contains(&format!("clone = \"{}\"", car.clone)));

    assert!(matches!(job::start(&dir, &game).unwrap(), Start::UpToDate { cars: 1 }));
    std::fs::write(dir.join("config.toml"), "source = 'nowhere'\n").unwrap();
    let Start::Started { job: second, live } = job::start(&dir, &game).unwrap() else {
        panic!("a changed setting converts")
    };
    assert!(live, "content from the first run is live now, so the new run waits for the next start");
    assert!(matches!(wait(&second), Finished::Failed(_)), "a missing install is a failure, not a crash");
    assert!(dir.join("files").is_dir(), "a failed run leaves the content as it was");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR); prints the time"]
fn every_car_converts_and_how_long_it_takes() {
    let (Some(source), Some(game)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-all-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = nfsmw_2012_content::config::Config::default();
    let mut last = None;
    for threads in [1, 3, nfsmw_2012_content::import::default_threads()] {
        let started = Instant::now();
        let report =
            nfsmw_2012_content::import::run(&config, Path::new(&source), Path::new(&game), &dir, threads).unwrap();
        println!("{} cars in {:.2} s on {threads} threads", report.imported.len(), started.elapsed().as_secs_f32());
        last = Some(report);
    }
    let report = last.unwrap();
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    assert!(report.imported.len() > 90);
    std::fs::remove_dir_all(&dir).unwrap();
}
