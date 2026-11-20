//! Tyre squeal and skid marks of a bike (`docs/specs/ride6-bike-skid.md`): they come with real tyre slip and with nothing else.
//! The report was a screech and marks on most bikes going fast, straight or in gentle turns.

use blackbox_carsound::{CarInput, CarSoundTuning, EffectsMixer, EngineOutput, LoopId, SoundCommand};
use blackbox_vehicle::FlatGround;
use blackbox_vehicle::bike::BikeSpec;
use nfsmw_data::car::physics::{CarBounds, SurfaceTable};

use super::*;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };
const BASE: f32 = 5.0;
const FRONT: usize = 0;
const REAR: usize = 2;

fn sim_of(spec: Arc<BikeSpec>) -> CarSim {
    CarSim::new(CarPhysics::from_bike(spec, 0.0, BOUNDS), [0.32; 4])
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, BASE), heading: 0.0 }
}

/// What the effects read from a ride: the strongest value of each over the run, and the skid loops the car sound asked for.
#[derive(Debug, Default)]
struct Run {
    skid: [f32; 2],
    smoke: [f32; 2],
    slip: [f32; 2],
    lateral: [f32; 2],
    /// The strongest slide of each tyre past which the tyre model itself says it skids (1 = the force peak).
    slide: [f32; 2],
    loop_volume: f32,
    speed: f32,
    lean: f32,
    /// A step when both tyres had held their peak for half a second and the effects still read a slide.
    heard_while_gripping: bool,
}

/// Ride `seconds` from `speed` m/s; `drive(t, speed)` is the input at time `t` s. The sound is fed as the game feeds it.
fn ride(sim: &mut CarSim, speed: f32, seconds: f32, drive: impl Fn(f32, f32) -> DriveInput) -> Run {
    let ground = FlatGround::new(BASE);
    assert!(sim.place_moving(&ground, spawn(), speed));
    let table = SurfaceTable::default();
    let mut mixer = EffectsMixer::new(&CarSoundTuning::default());
    let mut run = Run::default();
    let mut commands = Vec::new();
    let steps = (seconds / FIXED_STEP) as usize;
    let mut gripping = 0;
    for step in 0..steps {
        let now = step as f32 * FIXED_STEP;
        let input = drive(now, sim.telemetry().speed_mps);
        let _ = sim.step(&input, &ground, None);
        let input = CarInput {
            gear: 4,
            speed: sim.body().linear_velocity.length(),
            wheels: sim.wheel_sounds(&table),
            up_dot: sim.up_dot(),
            two_wheeler: sim.is_bike(),
            ..Default::default()
        };
        commands.clear();
        mixer.update(FIXED_STEP, &input, &EngineOutput::default(), &mut commands);
        // The first second is the bike settling on its springs.
        if now < 1.0 {
            continue;
        }
        let mut heard = 0.0_f32;
        for command in &commands {
            if let SoundCommand::SetLoop { id: LoopId::Skid(_), volume, .. } = command {
                heard = heard.max(*volume);
            }
        }
        run.loop_volume = run.loop_volume.max(heard);
        let slides = [FRONT, REAR].map(|wheel| sim.ride.as_ref().map_or(0.0, |ride| ride.bike.wheel(wheel).slide));
        gripping = match slides.iter().all(|slide| *slide <= 1.0) {
            true => gripping + 1,
            false => 0,
        };
        for (slot, wheel) in [FRONT, REAR].into_iter().enumerate() {
            let state = sim.wheel_state(wheel);
            run.skid[slot] = run.skid[slot].max(state.skid);
            run.smoke[slot] = run.smoke[slot].max(state.smoke);
            run.slip[slot] = run.slip[slot].max(state.slip.abs());
            run.lateral[slot] = run.lateral[slot].max(state.lateral_speed.abs());
            run.slide[slot] = run.slide[slot].max(slides[slot]);
            run.heard_while_gripping |= gripping > 30 && (heard > 0.0 || state.skid > 0.0 || state.smoke > 0.0);
        }
        run.lean = run.lean.max(sim.ride.as_ref().map_or(0.0, |ride| ride.bike.lean().abs()));
    }
    run.speed = sim.telemetry().speed_mps;
    run
}

/// The throttle that holds `target` m/s.
fn hold(target: f32, speed: f32) -> f32 {
    ((target - speed) * 0.2 + 0.1).clamp(0.0, 1.0)
}

/// The bikes to look at: the example bike, and every bike of a folder of imported manifests when `NFSMW_SKID_BIKES` names one (a
/// comma-free list of `.toml` files).
fn bikes() -> Vec<(String, Arc<BikeSpec>)> {
    let mut bikes = vec![("example".to_owned(), Arc::new(BikeSpec::example()))];
    let Ok(list) = std::env::var("NFSMW_SKID_BIKES") else { return bikes };
    for path in list.split(';').filter(|path| !path.is_empty()) {
        let text = std::fs::read_to_string(path).expect("a readable manifest");
        let stem = std::path::Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("bike").to_owned();
        let manifest = blackbox_modding::content::parse_car(&stem, &text).expect("a manifest");
        let bike = manifest.bike.expect("a bike manifest");
        bikes.push((stem, Arc::new(BikeSpec::from(&bike))));
    }
    bikes
}

/// The bikes' tests at speed: the example bike, and the imported ones too when they are at hand.
fn at_speed(label: &str, steer: impl Fn(f32) -> f32 + Copy) {
    for (name, spec) in bikes() {
        for speed in [30.0_f32, 50.0, 80.0] {
            let mut sim = sim_of(spec.clone());
            let run = ride(&mut sim, speed, 6.0, |t, v| DriveInput {
                throttle: hold(speed, v),
                steer: steer(t),
                ..DriveInput::default()
            });
            assert!(run.speed > 25.0 && !sim.rider_off(), "{name} {speed} {label}: {run:?}");
            assert_eq!(run.loop_volume, 0.0, "{name} {speed} {label}: a screech {run:?}");
            assert_eq!(run.skid, [0.0; 2], "{name} {speed} {label}: marks {run:?}");
            assert_eq!(run.smoke, [0.0; 2], "{name} {speed} {label}: smoke {run:?}");
            assert!(!run.heard_while_gripping, "{name} {speed} {label}: {run:?}");
        }
    }
}

#[test]
fn a_bike_riding_straight_at_speed_does_not_screech_or_mark() {
    at_speed("straight", |_| 0.0);
}

#[test]
fn a_bike_in_a_gentle_slalom_at_speed_does_not_screech_or_mark() {
    at_speed("slalom", |t| (t * 1.6).sin() * 0.3);
}

/// What a hard stop and a burnout read, for every bike at hand: a tyre the model itself calls skidding marks and screeches, and one
/// that grips does neither. The example bike has no ABS to speak of and locks its rear wheel; the others differ.
fn skids_exactly_when_the_tyre_skids(label: &str, speed: f32, drive: fn(f32, f32) -> DriveInput) -> Vec<Run> {
    let mut runs = Vec::new();
    for (name, spec) in bikes() {
        let mut sim = sim_of(spec);
        let run = ride(&mut sim, speed, 4.0, drive);
        assert_eq!(run.skid[0], 0.0, "{name} {label}: the front tyre marked {run:?}");
        match run.slide[1] > 1.5 {
            true => assert!(
                run.skid[1] > 0.0 && run.loop_volume > 0.0,
                "{name} {label}: the rear tyre skidded silently {run:?}"
            ),
            false => assert!(
                run.slide[1] > 1.05 || run.skid[1] == 0.0,
                "{name} {label}: the rear tyre gripped and marked {run:?}"
            ),
        }
        runs.push(run);
    }
    runs
}

#[test]
fn a_locked_rear_wheel_under_the_brakes_marks_and_screeches() {
    let runs = skids_exactly_when_the_tyre_skids("both brakes", 35.0, |_, _| DriveInput {
        brake: 1.0,
        ..DriveInput::default()
    });
    let example = &runs[0];
    assert!(example.slide[1] > 2.0 && example.skid[1] == 1.0 && example.smoke[1] == 1.0, "{example:?}");
    // The squeal's volume follows the speed, and the brake help (`docs/specs/ride6-bike-braking.md`) has the bike slower after the first second (the
    // settling second the run skips): 0.18 at 21 m/s where it was above 0.2 at 26 m/s. Still well heard.
    assert!(example.loop_volume > 0.15, "{example:?}");
}

#[test]
fn a_burnout_marks_smokes_and_screeches() {
    let runs = skids_exactly_when_the_tyre_skids("burnout", 0.0, |_, _| DriveInput {
        throttle: 1.0,
        brake: 1.0,
        ..DriveInput::default()
    });
    for run in runs {
        assert!(run.skid[1] == 1.0 && run.smoke[1] == 1.0 && run.loop_volume > 0.2, "{run:?}");
        assert!(run.speed.abs() < 0.5, "the bike stays where it burns: {run:?}");
    }
}

#[test]
fn a_corner_leaned_hard_reads_nothing_while_both_tyres_hold_their_peak() {
    for (name, spec) in bikes() {
        let mut sim = sim_of(spec);
        let run = ride(&mut sim, 30.0, 4.0, |_, v| DriveInput {
            throttle: hold(30.0, v),
            steer: 1.0,
            ..DriveInput::default()
        });
        assert!(run.lean > 0.6, "{name}: the bike leans hard: {run:?}");
        assert!(!run.heard_while_gripping, "{name}: {run:?}");
        let held = run.slide.iter().all(|slide| *slide <= 1.0);
        assert!(!held || (run.skid == [0.0; 2] && run.loop_volume == 0.0), "{name}: {run:?}");
    }
}

/// A car is unchanged: it still screeches and marks in a hard turn at speed with the brake on (its readouts are the vehicle library's).
#[test]
fn a_car_still_screeches_in_a_hard_turn() {
    let mut car = CarSim::new(CarPhysics::from_stats(&Default::default(), BOUNDS), [0.2; 4]);
    let run = ride(&mut car, 35.0, 4.0, |_, _| DriveInput { brake: 0.6, steer: 1.0, ..DriveInput::default() });
    assert!(run.loop_volume > 0.0, "{run:?}");
    assert!(run.skid.iter().any(|skid| *skid > 0.0), "{run:?}");
}

/// A case of the report: its name, the speed it starts at and the driver.
type Case = (&'static str, f32, fn(f32, f32) -> DriveInput);

#[test]
#[ignore = "a report: prints what the effects read of every bike in the hard cases"]
fn report_the_hard_cases() {
    for (name, spec) in bikes() {
        let cases: [Case; 4] = [
            ("rear brake", 35.0, |_, _| DriveInput { handbrake: true, ..DriveInput::default() }),
            ("both brakes", 35.0, |_, _| DriveInput { brake: 1.0, ..DriveInput::default() }),
            ("burnout", 0.0, |_, _| DriveInput { throttle: 1.0, brake: 1.0, ..DriveInput::default() }),
            ("corner", 30.0, |_, v| DriveInput { throttle: hold(30.0, v), steer: 1.0, ..DriveInput::default() }),
        ];
        for (label, speed, drive) in cases {
            let mut sim = sim_of(spec.clone());
            let run = ride(&mut sim, speed, 4.0, drive);
            eprintln!("{name} {label}: {run:?}");
        }
    }
}
