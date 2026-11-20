//! The lazy files, against the real 2012 install: a run that writes stubs and a run that writes every file make the same
//! car, byte for byte, once the maker has converted it again. Set `MW2012_GAME_DIR` (the 2012 install) and run with
//! `--ignored --nocapture`; the sizes and the times are printed.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nfsmw_2012_content::job::{self, Finished, Job, Start};
use nfsmw_2012_content::lazy::{self, Key, Maker, Part, Recipe, Sound};

fn folder(name: &str, source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-lazy-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = ['POR911CARR12', 'ALF4C12']\n"))
        .unwrap();
    dir
}

fn wait(job: &Job) {
    let started = Instant::now();
    loop {
        match job.poll() {
            Some(Finished::Done(_)) => return,
            Some(Finished::Failed(reason)) => panic!("the conversion failed: {reason}"),
            None => {}
        }
        assert!(started.elapsed() < Duration::from_secs(300), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Run one conversion in `dir`, lazy or not, and promote it. Returns how long it took.
fn convert(dir: &Path, lazy: bool) -> Duration {
    let started = Instant::now();
    let Start::Started { job: running, .. } = job::start_with(dir, None, |_| lazy).unwrap() else {
        panic!("the first start must convert")
    };
    wait(&running);
    assert!(job::promote(dir).unwrap());
    started.elapsed()
}

fn size_of(dir: &Path) -> u64 {
    let mut total = 0;
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        total += if path.is_dir() { size_of(&path) } else { std::fs::metadata(&path).unwrap().len() };
    }
    total
}

/// Every key of the car `id` that the eager folder has a file for: the two files, and the recordings its manifest names.
fn keys_of(eager: &Path, id: &str) -> Vec<(String, PathBuf)> {
    let cars = eager.join("files/CARS").join(id);
    let mut keys = vec![
        (lazy::car_key(id, Part::Geometry), cars.join("GEOMETRY.BIN")),
        (lazy::car_key(id, Part::Textures), cars.join("TEXTURES.BIN")),
    ];
    let text = std::fs::read_to_string(eager.join("cars").join(format!("{}.toml", id.to_ascii_lowercase()))).unwrap();
    let manifest = blackbox_modding::content::parse_car(id, &text).unwrap();
    if let Some(sound) = manifest.car_sound {
        let lists = [
            (Sound::Shift, sound.shift),
            (Sound::Whine, sound.turbo_whine),
            (Sound::Blowoff, sound.turbo_blowoff),
            (Sound::Backfire, sound.backfire),
            (Sound::Pops, sound.pops),
        ];
        for (kind, files) in lists {
            for (at, file) in files.iter().enumerate() {
                keys.push((lazy::sound_key(id, kind, at), eager.join("files").join(file)));
            }
        }
    }
    for (at, layer) in manifest.engine_sound.map(|engine| engine.layers).unwrap_or_default().iter().enumerate() {
        keys.push((lazy::sound_key(id, Sound::Engine, at), eager.join("files").join(&layer.file)));
    }
    keys
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR); prints sizes and times"]
fn the_maker_makes_the_files_a_run_that_writes_everything_writes() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = PathBuf::from(source);
    let (eager, streamed) = (folder("eager", &source), folder("streamed", &source));
    let eager_time = convert(&eager, false);
    let streamed_time = convert(&streamed, true);
    println!(
        "2 cars: every file {:.2} s and {} bytes; stubs {:.2} s and {} bytes",
        eager_time.as_secs_f32(),
        size_of(&eager),
        streamed_time.as_secs_f32(),
        size_of(&streamed)
    );
    let recipe = Recipe::read(&streamed).expect("a run that writes stubs writes its recipe");
    assert_eq!(recipe.car.len(), 2);
    assert!(Recipe::read(&eager).is_none(), "a run that writes every file has no recipe");
    let maker = Maker::new(recipe);
    use blackbox_modding::sdk::LazyFiles;
    for car in ["POR911CARR12", "ALF4C12"] {
        let stub = std::fs::read(streamed.join("files/CARS").join(car).join("GEOMETRY.BIN")).unwrap();
        assert!(stub.len() < 128, "{car}: the geometry on disk is a stub of {} bytes", stub.len());
        for (key, path) in keys_of(&eager, car) {
            let started = Instant::now();
            let made = maker.read(&key).unwrap_or_else(|error| panic!("{key}: {error:?}"));
            println!("{key}: {} bytes in {:.0} ms", made.len(), started.elapsed().as_secs_f32() * 1000.0);
            let written = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(made == written, "{key} differs from the file the other run wrote");
            assert!(matches!(lazy::parse_key(&key), Some(Key::Car(..) | Key::Sound(..))));
        }
    }
    for (what, millis) in maker.timings() {
        println!("{what}: {millis:.0} ms");
    }
    std::fs::remove_dir_all(&eager).unwrap();
    std::fs::remove_dir_all(&streamed).unwrap();
}
