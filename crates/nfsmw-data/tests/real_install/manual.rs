//! Driving real cars with the gears changed by hand (docs/specs/vehicle-manual-shifting.md).

use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle};
use nfsmw_data::car::physics::PhysicsData;

use crate::install;

const CARS: [&str; 5] = ["BMWM3GTR", "PORSCHE911", "CORVETTE", "CAMARO", "MUSTANGGT"];

fn manual(data: &PhysicsData, name: &str) -> Option<Vehicle> {
    let mut v = Vehicle::new(data.car(name).ok()?.spec);
    v.config.automatic = false;
    assert!(v.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 5.0, 0.0));
    Some(v)
}

fn step(v: &mut Vehicle, input: InputState) {
    v.step(FIXED_STEP, &input, &FlatGround::new(0.0));
}

/// Full throttle with an upshift at 92% of the way to the red line, for `seconds`. Returns the gears used.
fn pull(v: &mut Vehicle, seconds: f32) -> Vec<usize> {
    let red = v.spec().engine.red_line;
    let top = v.powertrain().top_gear();
    let mut gears = vec![v.gear()];
    for _ in 0..(seconds / FIXED_STEP) as usize {
        let up = v.rpm() > 0.92 * red && v.gear() < top && !v.powertrain().shifting();
        step(v, InputState { throttle: 1.0, shift_up: up, ..Default::default() });
        assert!(v.rpm() <= red + 1.0, "rpm {} over the red line {red}", v.rpm());
        if gears.last() != Some(&v.gear()) {
            gears.push(v.gear());
        }
    }
    gears
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_manual_pull_climbs_the_gears_one_at_a_time() {
    let Some(dir) = install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    for name in CARS {
        let Some(mut v) = manual(&data, name) else { continue };
        let gears = pull(&mut v, 25.0);
        assert!(gears.windows(2).all(|w| w[1] == w[0] + 1), "{name}: gears {gears:?}");
        assert!(gears.len() >= 5, "{name}: only reached {gears:?}");
        assert!(v.forward_speed() * 3.6 > 160.0, "{name}: {:.0} km/h after 25 s", v.forward_speed() * 3.6);
        assert!(v.position().x.abs() < 1.0, "{name}: veered {} m", v.position().x);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_manual_box_stays_in_the_gear_it_is_given() {
    let Some(dir) = install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    for name in CARS {
        let Some(mut v) = manual(&data, name) else { continue };
        let first = v.gear();
        let red = v.spec().engine.red_line;
        for _ in 0..(10.0 / FIXED_STEP) as usize {
            step(&mut v, InputState { throttle: 1.0, ..Default::default() });
            assert_eq!(v.gear(), first, "{name} shifted by itself");
            assert!(v.rpm() <= red + 1.0);
        }
        assert!(v.rpm() > red - 400.0, "{name}: {} rpm held on the limiter in first", v.rpm());
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn downshifting_far_into_the_limiter_slows_the_car_without_blowing_up() {
    let Some(dir) = install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    for name in CARS {
        let Some(mut v) = manual(&data, name) else { continue };
        let red = v.spec().engine.red_line;
        pull(&mut v, 14.0);
        step(&mut v, InputState::default());
        let (gear, start) = (v.gear(), v.forward_speed());
        assert!(start > 30.0, "{name}: set-up speed {start}");
        // Straight down to first.
        for _ in 0..gear {
            step(&mut v, InputState { shift_down: true, ..Default::default() });
        }
        for i in 0..(4.0 / FIXED_STEP) as usize {
            step(&mut v, InputState::default());
            let s = v.forward_speed();
            assert!(s.is_finite() && v.rpm().is_finite() && v.rpm() <= red + 1.0, "{name} at step {i}: {s} m/s");
            assert!(v.position().x.abs() < 3.0, "{name}: veered {} m", v.position().x);
        }
        assert!(v.forward_speed() < start - 3.0, "{name}: {start} -> {} m/s", v.forward_speed());
        assert!(v.forward_speed() > 0.0, "{name}: stopped dead or reversed");
    }
}
