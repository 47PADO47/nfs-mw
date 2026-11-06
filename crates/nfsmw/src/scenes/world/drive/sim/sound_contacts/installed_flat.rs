//! Installed race cars on a flat road: once settled, no body (underbody scrape) contact may reach audio.
use super::*;
use crate::audio::ContactSource;
use crate::scenes::world::{drive::DriveInput, road::Spawn};
use blackbox_vehicle::FlatGround;
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::car::physics::PhysicsData;

/// Race-car records the career and the reported scrapes involve, plus a traffic sedan for comparison.
const CARS: [&str; 8] = ["carreragt", "cts", "bmwm3gtre46", "gti", "supra", "911turbo", "gallardo", "traf4dseda"];

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): installed race-car physics on flat ground"]
fn installed_race_cars_never_scrape_a_flat_road() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut failures = Vec::new();
    for name in CARS {
        for (ai, start, throttle) in [(true, 0.0, 0.0), (true, 25.0, 1.0), (true, 60.0, 1.0), (false, 40.0, 0.6)] {
            let physics = data.car(name).unwrap();
            let mut sim = CarSim::new(physics, [0.2; 4]);
            if ai {
                sim.configure_ai();
                sim.collision_racer = name != "traf4dseda";
            }
            assert!(sim.place_moving(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }, start), "{name} placed");
            let input = DriveInput { throttle, ..Default::default() };
            let (mut scrapes, mut lowest, mut wheels_min) = (0usize, f32::INFINITY, 4usize);
            for step in 0..360 {
                let impact = sim.step(&input, &ground, None);
                let mut sounds = impact.sounds;
                sounds.extend(sim.take_sound_contacts());
                let settled = step >= 60;
                if !settled {
                    continue;
                }
                scrapes += sounds.iter().filter(|c| c.source == ContactSource::Ground).count();
                let body = sim.body();
                let bottom = body.position.y - body.dimension().y;
                lowest = lowest.min(bottom);
                wheels_min = wheels_min.min(sim.telemetry().wheels_on_ground);
            }
            let speed = sim.body().linear_velocity.length();
            eprintln!(
                "{name} ai={ai} start={start} throttle={throttle}: body contacts {scrapes}, lowest box bottom {lowest:.3} m, \
                 min wheels {wheels_min}, end speed {speed:.1} m/s"
            );
            if scrapes > 0 || wheels_min < 4 {
                failures
                    .push(format!("{name} ai={ai} start={start}: {scrapes} ground body contacts, {wheels_min} wheels"));
            }
        }
    }
    assert!(failures.is_empty(), "flat-road scrapes: {failures:#?}");
}
