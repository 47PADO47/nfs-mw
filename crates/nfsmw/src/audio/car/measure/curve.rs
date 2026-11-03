//! The level of a mod car's engine at every engine speed from idle to redline, to find holes in its layer set.
//! `NFSMW_GAME_DIR` (the 2005 install), `NFSMW_MEASURE_MOD` (a mod folder), `NFSMW_MEASURE_CARS` (comma list of cars),
//! `NFSMW_MEASURE_STEP` (rpm between points, default 250), `NFSMW_MEASURE_THROTTLE` (default 1) and `NFSMW_MEASURE_OUT`.

use std::io::Write;
use std::path::PathBuf;

use super::rig::{Heard, RATE, Rig};
use super::{dsp, scenario};

fn env_number(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|text| text.parse().ok()).unwrap_or(default)
}

#[test]
#[ignore = "renders cars offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD, NFSMW_MEASURE_CARS, NFSMW_MEASURE_STEP)"]
fn level_curve() {
    let Some(game) = std::env::var_os("NFSMW_GAME_DIR").map(PathBuf::from) else { return };
    let mod_dir = std::env::var_os("NFSMW_MEASURE_MOD").map(PathBuf::from);
    let rig = Rig::open(&game, mod_dir.as_deref());
    let step = env_number("NFSMW_MEASURE_STEP", 250.0);
    let throttle = env_number("NFSMW_MEASURE_THROTTLE", 1.0);
    let cars = std::env::var("NFSMW_MEASURE_CARS").unwrap_or_default();
    let mut out = String::new();
    for car in cars.split(',').map(str::trim).filter(|car| !car.is_empty()) {
        let Some(sound) = nfsmw_data::car::mods::engine_sound(car) else {
            out += &format!("{car}: no engine sound\n");
            continue;
        };
        let (idle, redline) = (sound.idle_rpm, sound.redline_rpm);
        let mut rpm = idle;
        while rpm <= redline + 0.5 {
            let fraction = (rpm - idle) / (redline - idle);
            let pcm = rig.render(car, &scenario::steady(fraction, throttle, 1.2), Heard::Engine);
            let tail = &pcm[pcm.len() - RATE as usize / 2..];
            out += &format!("{car} {rpm:.0} {:.1}\n", dsp::rms_db(tail));
            rpm += step;
        }
    }
    print!("{out}");
    if let Some(path) = std::env::var_os("NFSMW_MEASURE_OUT") {
        std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap().write_all(out.as_bytes()).unwrap();
    }
}

/// Repeated pulls and lifts (full throttle for 2.5 s at 0.8 of the way to the redline, then the throttle shut for 1.2 s), to
/// see which lifts sound a backfire or a pop: for each lift the peak of the effects alone in the second after it, and the
/// engine's own peak for scale. `NFSMW_MEASURE_LIFTS` is the number of lifts (default 20).
#[test]
#[ignore = "renders cars offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD, NFSMW_MEASURE_CARS, NFSMW_MEASURE_LIFTS)"]
fn lift_offs() {
    use blackbox_carsound::GEAR_FIRST;

    use super::rig::{FPS, Frame};

    let Some(game) = std::env::var_os("NFSMW_GAME_DIR").map(PathBuf::from) else { return };
    let mod_dir = std::env::var_os("NFSMW_MEASURE_MOD").map(PathBuf::from);
    let rig = Rig::open(&game, mod_dir.as_deref());
    let lifts = env_number("NFSMW_MEASURE_LIFTS", 20.0) as usize;
    let cars = std::env::var("NFSMW_MEASURE_CARS").unwrap_or_default();
    let gear = GEAR_FIRST + 2;
    let mut out = String::new();
    for car in cars.split(',').map(str::trim).filter(|car| !car.is_empty()) {
        let mut frames = Vec::new();
        let mut marks = Vec::new();
        let seconds = |s: f32| (s * FPS as f32) as usize;
        frames.extend((0..seconds(1.0)).map(|_| Frame::idle()));
        for _ in 0..lifts {
            frames.extend((0..seconds(2.5)).map(|_| Frame::drive(0.8, 1.0, gear, scenario::mph_at(0.8, gear))));
            marks.push(frames.len());
            frames.extend((0..seconds(1.2)).map(|n| {
                let rpm = 0.8 - 0.3 * n as f32 / seconds(1.2) as f32;
                Frame::drive(rpm, 0.0, gear, scenario::mph_at(rpm, gear))
            }));
        }
        let sfx = rig.render(car, &frames, Heard::Sfx);
        let engine = rig.render(car, &frames, Heard::Engine);
        let per_frame = (RATE / FPS) as usize;
        let engine_peak = dsp::peak_db(&engine);
        let (mut heard, mut loudest) = (0, f32::MIN);
        let mut peaks = Vec::new();
        for mark in &marks {
            let window = &sfx[mark * per_frame..(mark + seconds(1.0)) * per_frame];
            let peak = dsp::peak_db(window);
            peaks.push(format!("{peak:.0}"));
            if peak > -30.0 {
                heard += 1;
                loudest = loudest.max(peak);
            }
        }
        out += &format!(
            "{car}: {heard} of {lifts} lifts make a sound; loudest {loudest:.1} dBFS; engine peak {engine_peak:.1} dBFS
               peak after each lift: {}
",
            peaks.join(" ")
        );
    }
    print!("{out}");
}

/// A pull with the gear number changing every second (up three times, then down twice), the throttle held: the peak of the
/// effects alone in the 0.4 s after each change, against the 0.4 s before it. The shifts of a bike are played by the change.
#[test]
#[ignore = "renders cars offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD, NFSMW_MEASURE_CARS)"]
fn gear_changes() {
    use blackbox_carsound::GEAR_FIRST;

    use super::rig::{FPS, Frame};

    let Some(game) = std::env::var_os("NFSMW_GAME_DIR").map(PathBuf::from) else { return };
    let mod_dir = std::env::var_os("NFSMW_MEASURE_MOD").map(PathBuf::from);
    let rig = Rig::open(&game, mod_dir.as_deref());
    let cars = std::env::var("NFSMW_MEASURE_CARS").unwrap_or_default();
    let gears = [2, 3, 4, 5, 4, 3];
    let mut out = String::new();
    for car in cars.split(',').map(str::trim).filter(|car| !car.is_empty()) {
        let frames: Vec<Frame> = gears
            .iter()
            .flat_map(|&n| {
                (0..FPS)
                    .map(move |_| Frame::drive(0.6, 1.0, GEAR_FIRST + n - 2, scenario::mph_at(0.6, GEAR_FIRST + n - 2)))
            })
            .collect();
        let sfx = rig.render(car, &frames, Heard::Sfx);
        let per_frame = (RATE / FPS) as usize;
        let window = RATE as usize * 4 / 10;
        let mut line = format!("{car}: peak of the effects after / before each gear change (dBFS):");
        for change in 1..gears.len() {
            let at = change * FPS as usize * per_frame;
            let (before, after) = (dsp::peak_db(&sfx[at - window..at]), dsp::peak_db(&sfx[at..at + window]));
            line += &format!("  {} -> {}: {after:.0} / {before:.0}", gears[change - 1], gears[change]);
        }
        out += &line;
        out += "\n";
    }
    print!("{out}");
}
