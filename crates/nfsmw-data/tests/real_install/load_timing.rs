//! Load-time benchmark for the data loads the game repeats. It prints timings and asserts nothing, so a later
//! in-memory cache can be measured before and after with the same command.

use std::fmt::Display;
use std::time::{Duration, Instant};

use game_install::GameDir;
use nfsmw_data::car::physics::PhysicsData;
use nfsmw_data::car::{self, CarTables, LoadOptions};
use nfsmw_data::{gameplay, read_unwrapped, world};

use crate::install;

/// Cars loaded one after another, the first ones in `car::list` order that assemble.
const CARS: usize = 5;

fn print_ms(what: &str, elapsed: Duration) {
    eprintln!("load-timing {what}: {:.1} ms", elapsed.as_secs_f64() * 1e3);
}

/// Run `load` once, print how long it took under `what`, and hand back its result.
fn timed<T, E: Display>(what: &str, load: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let start = Instant::now();
    let result = load();
    print_ms(what, start.elapsed());
    result
}

/// Run `load` `n` times under `label`, panicking with `label` if it fails.
fn repeat<T, E: Display>(label: &str, n: usize, mut load: impl FnMut() -> Result<T, E>) {
    for i in 1..=n {
        timed(&format!("{label} #{i}"), &mut load).unwrap_or_else(|e| panic!("{label}: {e:#}"));
    }
}

/// Assemble the first [`CARS`] cars of the install with the options the game uses for traffic and the player.
fn load_cars(dir: &GameDir) {
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut spent = Duration::ZERO;
    let mut loaded = 0;
    for folder in car::list(dir) {
        if loaded == CARS {
            break;
        }
        let start = Instant::now();
        let result = car::load(dir, &folder, &options);
        let elapsed = start.elapsed();
        if let Err(e) = &result {
            eprintln!("load-timing car {folder}: skipped ({e:#})");
            continue;
        }
        print_ms(&format!("car {folder}"), elapsed);
        spent += elapsed;
        loaded += 1;
    }
    print_ms(&format!("cars total ({loaded} cars)"), spent);
}

#[test]
#[ignore = "needs the game; load-time benchmark"]
fn load_timing() {
    let Some(dir) = install() else { return };
    repeat("read_unwrapped GLOBAL/GLOBALB.LZC", 3, || read_unwrapped(&dir, "GLOBAL/GLOBALB.LZC"));
    repeat("CarTables::load", 3, || CarTables::load(&dir));
    load_cars(&dir);
    repeat("PhysicsData::load", 2, || PhysicsData::load(&dir));
    repeat("gameplay::load", 2, || gameplay::load(&dir));
    timed("load_global_textures", || world::load_global_textures(&dir)).unwrap_or_else(|e| panic!("globals: {e:#}"));
}
