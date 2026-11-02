//! A wheelie ridden as a `--drive-script` rides it (`docs/specs/ride6-wheelie-steer.md`, section 9), from a standstill, on the imported
//! bikes of a folder (`RIDE6_CARS=<folder> RIDE6_FILTER=ktm450`): `bike_wheelie_script` prints, every `RIDE6_EVERY` seconds, the speed,
//! pitch, lean and its limit, the heading and the steering, and says when and how the rider let go.
//! `RIDE6_SEGMENTS="seconds,throttle,stick,lever;..."` (default: 2.5 s and 2 s straight, 2 s at +1, 2 s at -1, throttle 1, lever -1).

use blackbox_vehicle::bike::{Bike, BikeInput};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::specs;

/// One held input of a script: how long, the throttle, the stick and the weight lever.
fn segments() -> Vec<[f32; 4]> {
    let text = std::env::var("RIDE6_SEGMENTS").unwrap_or_else(|_| "2.5,1,0,-1;2,1,0,-1;2,1,1,-1;2,1,-1,-1".to_string());
    text.split(';')
        .filter_map(|part| {
            let v: Vec<f32> = part.split(',').filter_map(|n| n.trim().parse().ok()).collect();
            <[f32; 4]>::try_from(v).ok()
        })
        .collect()
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_wheelie_script() {
    let every: f32 = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
    for (name, spec) in specs() {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
        println!("{name}");
        let (mut time, mut next) = (0.0_f32, 0.0_f32);
        for [seconds, throttle, stick, lever] in segments() {
            let end = time + seconds;
            while time < end {
                bike.set_weight(lever);
                bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
                time += FIXED_STEP;
                if time >= next {
                    next += every;
                    let forward = bike.orientation() * glam::Vec3::Z;
                    println!(
                        "  t{time:.2} stick{stick:+.0} v{:.1} pitch{:.0} lean{:.2}/{:.2} head{:.0} steer{:.2} yawrate{:.2} front{}",
                        bike.forward_speed(),
                        bike.pitch().to_degrees(),
                        bike.lean(),
                        bike.lean_limit(),
                        forward.x.atan2(forward.z).to_degrees(),
                        bike.steer_angle(),
                        bike.body().angular_velocity.y,
                        if bike.wheel(0).on_ground { "down" } else { "UP" }
                    );
                }
                if bike.fallen() {
                    println!("  FELL at t{time:.2}: {:?}", bike.fall_kind());
                    break;
                }
            }
            if bike.fallen() {
                break;
            }
        }
    }
}
