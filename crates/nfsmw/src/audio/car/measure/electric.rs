//! The electric car (the converted Tesla Roadster) through the real headless mixer: its sound is the motor's loops, whose
//! pitch rises with the car's speed, with no gear to clunk into and no combustion engine, so it is not judged by the revs
//! the way the other cars are (`regression`).
//!
//! Ignored without the installs: `NFSMW_GAME_DIR` (the 2005 install) and `NFSMW_MEASURE_MOD` (the converted cars).

use std::path::PathBuf;

use blackbox_carsound::GEAR_FIRST;

use super::dsp;
use super::report::{self, STEPS, steady_key};
use super::rig::{Frame, Heard, RATE, Rig};

/// The speeds the motor is heard at, in mph.
const SPEEDS: [f32; 5] = [15.0, 40.0, 70.0, 100.0, 120.0];

/// The rig and the folder of the Tesla among the converted cars.
fn open() -> Option<(Rig, String)> {
    let game = PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?);
    let mod_dir = PathBuf::from(std::env::var_os("NFSMW_MEASURE_MOD")?);
    let rig = Rig::open(&game, Some(&mod_dir));
    let car = nfsmw_data::car::mods::published()
        .iter()
        .find(|car| car.name.as_deref().is_some_and(|name| name.contains("Tesla")))?
        .folder
        .clone();
    Some((rig, car))
}

/// The last half second of the engine of `car` held at `mph` and `throttle`.
fn held(rig: &Rig, car: &str, mph: f32, throttle: f32) -> Vec<f32> {
    let frames: Vec<Frame> = (0..96).map(|_| Frame::drive(0.5, throttle, GEAR_FIRST + 1, mph)).collect();
    let pcm = rig.render(car, &frames, Heard::Engine);
    pcm[pcm.len() - RATE as usize / 2..].to_vec()
}

#[test]
#[ignore = "renders the converted Tesla offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD)"]
fn the_motor_rises_with_the_speed_at_a_steady_level() {
    let Some((rig, car)) = open() else { return };
    let (mut centroids, mut levels) = (Vec::new(), Vec::new());
    for mph in SPEEDS {
        let tail = held(&rig, &car, mph, 1.0);
        centroids.push(dsp::centroid_hz(&tail, RATE));
        levels.push(dsp::rms_db(&tail));
    }
    println!("{car}: centroid {centroids:.0?} Hz, level {levels:.1?} dB at {SPEEDS:?} mph");
    let (low, high) = levels.iter().fold((f32::MAX, f32::MIN), |(low, high), db| (low.min(*db), high.max(*db)));
    assert!(low >= -26.0 && high <= -12.0, "the motor is as loud as the other cars: {low:.1} to {high:.1} dB");
    assert!(high - low <= 8.0, "the motor keeps its level over the speeds: {:.1} dB apart", high - low);
    assert!(
        centroids.windows(2).all(|pair| pair[1] >= pair[0] * 0.97),
        "the motor never drops in pitch as the car speeds up: {centroids:.0?}"
    );
    assert!(centroids[4] > centroids[0] * 1.3, "the motor rises with the speed: {centroids:.0?}");
}

#[test]
#[ignore = "renders the converted Tesla offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD)"]
fn the_motor_does_not_follow_the_engine_speed_of_a_gear() {
    let Some((rig, car)) = open() else { return };
    // The same speed in two gears (two engine speeds) is the same sound: the motor is driven by the car's speed.
    let render = |rpm: f32| {
        let frames: Vec<Frame> = (0..96).map(|_| Frame::drive(rpm, 1.0, GEAR_FIRST + 1, 70.0)).collect();
        let pcm = rig.render(&car, &frames, Heard::Engine);
        dsp::centroid_hz(&pcm[pcm.len() - RATE as usize / 2..], RATE)
    };
    let (low, high) = (render(0.2), render(0.9));
    assert!((high / low - 1.0).abs() < 0.15, "centroid {low:.0} Hz at 20 percent revs, {high:.0} Hz at 90");
}

#[test]
#[ignore = "renders the converted Tesla offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD)"]
fn the_electric_car_makes_no_shift_pop_or_backfire_sounds() {
    let Some((rig, car)) = open() else { return };
    let metrics = report::measure(&rig, &car);
    assert_eq!(metrics.get("start.sfx.bursts"), 0.0, "a standing start through six shifts has no shift sound");
    let steady: Vec<f32> = STEPS.iter().skip(1).map(|step| metrics.get(&steady_key(1.0, *step, "db"))).collect();
    println!("{car}: steady loudness over the revs {steady:.1?} dB; sfx peak {:.1} dB", metrics.get("start.sfx.peak"));
}
