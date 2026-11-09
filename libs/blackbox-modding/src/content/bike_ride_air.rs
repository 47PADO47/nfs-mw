//! The audit of the arcade help on jumps and on rough ground (`docs/specs/ride6-bike-jumps.md`, `docs/specs/ride6-bike-rough-ground.md`), ridden
//! on the imported bikes of a folder (`RIDE6_CARS=<folder> [RIDE6_FILTER=a,b] [RIDE6_EVERY=n]`) as ignored tests, a diagnostic and not a
//! regression test:
//!
//! - `bike_jump_sweep`: every bike at 60, 80, 100 and 120 km/h over a step down of 1, 2, 3 and 5 m and off a ramp onto a flat road and onto a
//!   15 degree slope, 1 to 5 m below the lip; the falls by class (road, motard, offroad) and the first fall of each bike.
//! - `bike_rough_sweep`: five offroad bikes and three road bikes over a rough strip (a washboard of 2 m and of 12 m) and in a dirt corner, at 40
//!   and 80 km/h: the falls, the speed held and the lateral g.
//!
//! `RIDE6_ARCADE=air_before` rides this game's arcade steering without the help of the two specs (what the game was before them), `reference`
//! the reference game's, `air=0` and `rough=0` (with the other `RIDE6_ARCADE` keys) switch one help off, anything else is the game's own.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec, TerrainClass};
use blackbox_vehicle::ground::{Ground, ProfileGround, SurfaceGrip};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_steer_arcade::arcade;
use super::bike_ride_sweep::{held_throttle, specs};

/// Dirt as the host's surface table gives it, near enough: well under the road's grip.
const DIRT: SurfaceGrip = SurfaceGrip { lateral: 0.65, drive: 0.7, rolling: 1.5, tag: 3 };
/// Where the road ends and the jump begins (m from the start).
const LIP: f32 = 40.0;

fn bikes() -> Vec<(String, BikeSpec)> {
    let every: usize = std::env::var("RIDE6_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    let arcade = arcade();
    specs()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % every == 0)
        .map(|(_, (name, mut spec))| {
            spec.steering.arcade = arcade;
            (name, spec)
        })
        .collect()
}

/// A landing: a step down, a ramp then a flat road `drop` m below its lip, or a ramp then a slope of 15 degrees down to that road.
#[derive(Clone, Copy, Debug)]
enum Landing {
    Step,
    RampFlat,
    RampSlope,
}

fn ground_of(landing: Landing, drop: f32) -> ProfileGround {
    let road = SurfaceGrip::DEFAULT;
    match landing {
        Landing::Step => ProfileGround::step_down(LIP, drop, road),
        Landing::RampFlat => ProfileGround::new(vec![(LIP - 4.0, 0.0), (LIP, 1.0), (LIP + 0.05, -drop)], road),
        Landing::RampSlope => {
            let run = (drop + 1.0) / 0.27;
            ProfileGround::new(vec![(LIP - 4.0, 0.0), (LIP, 1.0), (LIP + 0.5, 1.0), (LIP + 0.5 + run, -drop)], road)
        }
    }
}

/// What a ride over a jump came to: the time and the kind of the fall, if any.
fn jump(spec: &BikeSpec, ground: &ProfileGround, speed: f32) -> Option<(f32, String)> {
    let mut bike = Bike::new(spec.clone());
    let flat = FlatGround::new(0.0);
    assert!(bike.place_moving(&flat, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = ((LIP + 40.0) / speed / FIXED_STEP) as usize + 240;
    for step in 0..steps {
        let throttle = held_throttle(speed, bike.velocity().length());
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, ground);
        if bike.fallen() {
            return Some((step as f32 * FIXED_STEP, format!("{:?}", bike.fall_kind())));
        }
    }
    None
}

fn name_of(class: TerrainClass) -> &'static str {
    match class {
        TerrainClass::Road => "road",
        TerrainClass::Motard => "motard",
        TerrainClass::Offroad => "offroad",
    }
}

#[test]
#[ignore = "an audit over the imported bikes (RIDE6_CARS)"]
fn bike_jump_sweep() {
    let bikes = bikes();
    let mut table = [[0_usize; 2]; 3];
    for (name, spec) in &bikes {
        let class = spec.terrain_class();
        let mut first: Option<String> = None;
        for landing in [Landing::Step, Landing::RampFlat, Landing::RampSlope] {
            for drop in [1.0, 2.0, 3.0, 5.0] {
                for kmh in [60.0_f32, 80.0, 100.0, 120.0] {
                    let result = jump(spec, &ground_of(landing, drop), kmh / 3.6);
                    table[class.index()][0] += 1;
                    let Some((t, kind)) = result else { continue };
                    table[class.index()][1] += 1;
                    first.get_or_insert(format!("{landing:?} {drop} m {kmh} km/h {kind} at {t:.1} s"));
                }
            }
        }
        println!("{name} ({}): {}", name_of(class), first.as_deref().unwrap_or("no fall"));
    }
    for class in [TerrainClass::Road, TerrainClass::Motard, TerrainClass::Offroad] {
        let [runs, falls] = table[class.index()];
        println!("{:8} {falls} falls in {runs} jumps", name_of(class));
    }
}

/// What a ride on rough ground came to.
struct Ride {
    fell: bool,
    speed: f32,
    lateral_g: f32,
}

fn ride(spec: &BikeSpec, ground: &dyn Ground, speed: f32, stick: f32, seconds: f32) -> Ride {
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(ground, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = (seconds / FIXED_STEP) as usize;
    let (mut sum, mut count) = (0.0_f32, 0.0_f32);
    for step in 0..steps {
        let throttle = held_throttle(speed, bike.velocity().length());
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, ground);
        if step + (2.0 / FIXED_STEP) as usize >= steps {
            sum += bike.velocity().length() * bike.body().angular_velocity.y.abs() / 9.81;
            count += 1.0;
        }
    }
    Ride { fell: bike.fallen(), speed: bike.velocity().length(), lateral_g: sum / count.max(1.0) }
}

#[test]
#[ignore = "an audit over the imported bikes (RIDE6_CARS)"]
fn bike_rough_sweep() {
    let all = bikes();
    let of = |class: TerrainClass, n: usize| -> Vec<&(String, BikeSpec)> {
        all.iter().filter(|(_, spec)| spec.terrain_class() == class).take(n).collect()
    };
    let (offroad, road) = (of(TerrainClass::Offroad, 5), of(TerrainClass::Road, 3));
    println!("{} offroad and {} road bikes", offroad.len(), road.len());
    for (label, group) in [("offroad", &offroad), ("road", &road)] {
        for kmh in [40.0_f32, 80.0] {
            let speed = kmh / 3.6;
            let (mut strip2, mut strip12, mut corner) = ((0, 0.0), (0, 0.0), (0, 0.0, 0.0));
            for (_, spec) in group {
                let short = ride(spec, &ProfileGround::washboard(10.0, 0.5, 4, 0.05, DIRT), speed, 0.0, 5.0);
                let long = ride(spec, &ProfileGround::washboard(10.0, 0.5, 24, 0.05, DIRT), speed, 0.0, 6.0);
                let bend = ride(spec, &FlatGround { height: 0.0, surface: DIRT }, speed, 0.7, 6.0);
                strip2 = (strip2.0 + usize::from(short.fell), strip2.1 + short.speed / speed);
                strip12 = (strip12.0 + usize::from(long.fell), strip12.1 + long.speed / speed);
                corner = (corner.0 + usize::from(bend.fell), corner.1 + bend.speed / speed, corner.2 + bend.lateral_g);
            }
            let n = group.len().max(1) as f32;
            println!(
                "{label:8} {kmh:3.0} km/h: 2 m strip {} falls, speed kept {:.2}; 12 m strip {} falls, {:.2}; dirt corner {} falls, speed kept {:.2}, {:.2} g",
                strip2.0,
                strip2.1 / n,
                strip12.0,
                strip12.1 / n,
                corner.0,
                corner.1 / n,
                corner.2 / n
            );
        }
    }
}
