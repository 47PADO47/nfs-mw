//! The measurement on the installs. Set `NFSMW_GAME_DIR` (the 2005 install) and `NFSMW_MEASURE_MOD` (a mod folder of
//! converted 2012 cars, written by `nfsmw-2012-content/tests/sound_convert.rs`); `NFSMW_MEASURE_CARS` is a comma list of
//! car names (default: BMWM3GTR and every converted car) and `NFSMW_MEASURE_OUT` a file the numbers are appended to.

use std::io::Write;
use std::path::PathBuf;

use super::report::{self, Metrics};
use super::rig::Rig;

fn rig() -> Option<Rig> {
    let game = PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?);
    let mod_dir = std::env::var_os("NFSMW_MEASURE_MOD").map(PathBuf::from);
    Some(Rig::open(&game, mod_dir.as_deref()))
}

fn cars() -> Vec<String> {
    if let Ok(list) = std::env::var("NFSMW_MEASURE_CARS") {
        return list.split(',').map(|name| name.trim().to_owned()).filter(|name| !name.is_empty()).collect();
    }
    let mut cars = vec!["BMWM3GTR".to_owned()];
    cars.extend(nfsmw_data::car::mods::published().iter().map(|car| car.folder.clone()));
    cars
}

fn emit(car: &str, metrics: &Metrics) {
    let lines = metrics.lines(car);
    print!("{lines}");
    if let Some(path) = std::env::var_os("NFSMW_MEASURE_OUT") {
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
        file.write_all(lines.as_bytes()).unwrap();
    }
}

#[test]
#[ignore = "renders cars offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD, NFSMW_MEASURE_CARS, NFSMW_MEASURE_OUT)"]
fn measure_cars() {
    let Some(rig) = rig() else { return };
    for car in cars() {
        let started = std::time::Instant::now();
        let metrics = report::measure(&rig, &car);
        emit(&car, &metrics);
        eprintln!("{car}: measured in {:.1} s", started.elapsed().as_secs_f32());
    }
}

/// The level and brightness of each layer of one car alone, at three engine speeds (`NFSMW_MEASURE_SOLO` names the car).
#[test]
#[ignore = "renders one car layer by layer (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD, NFSMW_MEASURE_SOLO)"]
fn solo_layers() {
    use std::sync::atomic::Ordering;

    use super::super::own_engine::SOLO_LAYER;
    use super::{dsp, rig::RATE, scenario};

    let (Some(rig), Ok(car)) = (rig(), std::env::var("NFSMW_MEASURE_SOLO")) else { return };
    let layers = nfsmw_data::car::mods::engine_sound(&car).expect("a converted car").layers.len();
    for only in (0..layers).chain([usize::MAX]) {
        SOLO_LAYER.store(only, Ordering::Relaxed);
        let mut line = format!("{car} layer {}:", if only == usize::MAX { "all".into() } else { only.to_string() });
        for rpm in [0.0, 0.5, 1.0] {
            let pcm = rig.render(&car, &scenario::steady(rpm, 1.0, 1.6), super::rig::Heard::Engine);
            let tail = &pcm[pcm.len() - RATE as usize / 2..];
            line +=
                &format!("  r{:.0}: {:6.1} dB {:5.0} Hz", rpm * 100.0, dsp::rms_db(tail), dsp::centroid_hz(tail, RATE));
        }
        println!("{line}");
    }
    SOLO_LAYER.store(usize::MAX, Ordering::Relaxed);
}
