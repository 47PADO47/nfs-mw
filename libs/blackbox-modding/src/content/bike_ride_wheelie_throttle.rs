//! The wheelie from the throttle alone (`docs/specs/ride6-wheelie-throttle.md`), on the imported bikes of a folder, with the game's
//! assist off (the reference) and on, the weight lever never touched:
//!
//! ```text
//! RIDE6_CARS=<folder of manifests> [RIDE6_FILTER=a,b] cargo test -p blackbox-modding --features host --lib bike_wheelie_throttle -- --ignored --nocapture
//! ```
//!
//! Per bike and per setting: a launch from a standstill with the throttle flat out (the time to 10 degrees of pitch and the highest pitch in
//! 8 s), the throttle held flat out for 10 s from 20 m/s (the highest pitch: a bike that loops out falls), a throttle tapped for 0.4 s and
//! released for 0.4 s from 12 m/s (the highest pitch and whether the front wheel comes down between the taps) and a bang-bang throttle
//! (on below 30 degrees of pitch, off above) from a standstill: whether the pitch stays between 8 and 46 degrees from 2 s to 9 s (the assist
//! only works in the first three gears, so the run starts in first). Each also says whether the bike fell, and by how many degrees the
//! assist's wheelie went over the cap of its gear (`excess`, 1.5 s after a gear change left out). The totals count the bikes that lift in 3.5 s,
//! hold, fall, and go over a cap by more than 6 degrees.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec, ThrottleWheelie};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::bike_ride_sweep::specs;

/// The game's settings, with the lever (`RIDE6_TW_LEVER`, -1 to 0) and the throttle that asks (`RIDE6_TW_THROTTLE`) tried at other values.
fn game() -> ThrottleWheelie {
    let number = |name: &str, default: f32| std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default);
    let base = ThrottleWheelie::GAME;
    ThrottleWheelie {
        lever: number("RIDE6_TW_LEVER", base.lever),
        throttle: number("RIDE6_TW_THROTTLE", base.throttle),
        ..base
    }
}

/// What one run saw: the highest pitch (degrees), the time the pitch first passed 10 degrees, the pitch range of the last 5 s, and whether
/// the bike fell.
#[derive(Clone, Copy, Debug, Default)]
struct Run {
    high: f32,
    ten: Option<f32>,
    last: (f32, f32),
    /// The most (degrees) the pitch stood over the cap of the gear while the throttle held the lever, 1.5 s after a gear change left out.
    excess: f32,
    fell: bool,
    /// The speed at the end (m/s), and the gear.
    speed: f32,
}

fn ride(spec: &BikeSpec, config: ThrottleWheelie, start: f32, seconds: f32, throttle: &dyn Fn(f32, f32) -> f32) -> Run {
    steered(spec, config, (start, seconds), throttle, &|_| 0.0)
}

/// [`ride`] with a stick as well: `stick(t)` is the handlebar input (-1 left to 1 right) at time `t`.
fn steered(
    spec: &BikeSpec,
    config: ThrottleWheelie,
    (start, seconds): (f32, f32),
    throttle: &dyn Fn(f32, f32) -> f32,
    stick: &dyn Fn(f32) -> f32,
) -> Run {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    bike.set_throttle_wheelie(config);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    let mut run = Run { last: (f32::MAX, f32::MIN), excess: f32::MIN, ..Run::default() };
    let (mut gear, mut changed) = (0, f32::MIN);
    let steps = (seconds / FIXED_STEP) as usize;
    for step in 0..steps {
        let t = step as f32 * FIXED_STEP;
        let input =
            BikeInput { throttle: throttle(t, bike.pitch().to_degrees()), steer: stick(t), ..Default::default() };
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            run.fell = true;
            return run;
        }
        let pitch = bike.pitch().to_degrees();
        run.high = run.high.max(pitch);
        if pitch > 10.0 && run.ten.is_none() {
            run.ten = Some(t);
        }
        run.speed = bike.forward_speed();
        if bike.gear() != gear {
            (gear, changed) = (bike.gear(), t);
        }
        if let (true, Some(cap)) = (bike.throttle_lever() && t - changed > 1.5, config.cap(bike.gear())) {
            run.excess = run.excess.max(pitch - cap.to_degrees());
        }
        if t >= 2.0 {
            run.last = (run.last.0.min(pitch), run.last.1.max(pitch));
        }
    }
    run
}

/// The runs of one bike with one setting.
struct Bike4 {
    launch: Run,
    flat: Run,
    taps: Run,
    spam: Run,
    blips: Run,
    turn: Run,
}

fn runs(spec: &BikeSpec, config: ThrottleWheelie) -> Bike4 {
    Bike4 {
        launch: ride(spec, config, 0.0, 8.0, &|_, _| 1.0),
        flat: ride(spec, config, 20.0, 10.0, &|_, _| 1.0),
        taps: ride(spec, config, 12.0, 10.0, &|t, _| f32::from((t % 0.8) < 0.4)),
        spam: ride(spec, config, 8.0, 12.0, &|t, _| f32::from((t % 1.1) < 0.7)),
        blips: ride(spec, config, 0.0, 9.0, &|t, pitch| f32::from(t < 0.6 || pitch < 30.0)),
        // Flat out at 15 m/s with the stick over to a third for 3 s, back to centre, then the other way: a wheelie that is steered.
        turn: steered(spec, config, (15.0, 10.0), &|_, _| 1.0, &|t| match t {
            t if t < 2.0 => 0.0,
            t if t < 5.0 => 0.35,
            t if t < 8.0 => -0.35,
            _ => 0.0,
        }),
    }
}

impl Bike4 {
    fn lifts(&self) -> bool {
        self.launch.ten.is_some_and(|t| t < 3.5)
    }

    fn holds(&self) -> bool {
        self.blips.last.0 > 8.0 && self.blips.last.1 < 46.0
    }

    /// The most the assist's wheelie went over the cap of its gear in any of the runs (degrees; very negative when it never asked).
    fn excess(&self) -> f32 {
        [self.launch, self.flat, self.taps, self.spam, self.blips, self.turn]
            .iter()
            .map(|run| run.excess)
            .fold(f32::MIN, f32::max)
    }

    fn fell(&self) -> bool {
        [self.launch, self.flat, self.taps, self.spam, self.blips, self.turn].iter().any(|run| run.fell)
    }

    fn line(&self) -> String {
        let ten = self.launch.ten.map_or("-".to_owned(), |t| format!("{t:.1}s"));
        format!(
            "launch 10deg {ten} max {:.0} then {:.0} km/h at 8 s; flat max {:.0}; taps max {:.0}; spam {:.0}..{:.0} (max {:.0}); turn max {:.0}; blips {:.0}..{:.0} {} excess {:.0}{}",
            self.launch.high,
            self.launch.speed * 3.6,
            self.flat.high,
            self.taps.high,
            self.spam.last.0,
            self.spam.last.1,
            self.spam.high,
            self.turn.high,
            self.blips.last.0,
            self.blips.last.1,
            if self.holds() { "HELD" } else { "lost" },
            self.excess(),
            if self.fell() { " FELL" } else { "" }
        )
    }
}

#[test]
#[ignore = "needs RIDE6_CARS"]
fn bike_wheelie_throttle() {
    let (mut total, mut lifts, mut holds, mut falls, mut over) = (0, [0; 2], [0; 2], [0; 2], 0);
    let (mut flat_loops, mut worst) = ([0; 2], [0.0_f32; 2]);
    for (name, spec) in specs() {
        total += 1;
        let both = [runs(&spec, ThrottleWheelie::REFERENCE), runs(&spec, game())];
        for (n, bike) in both.iter().enumerate() {
            lifts[n] += usize::from(bike.lifts());
            holds[n] += usize::from(bike.holds());
            falls[n] += usize::from(bike.fell());
            flat_loops[n] += usize::from(bike.flat.high > 60.0);
            over += usize::from(n == 1 && bike.excess() > 6.0);
            worst[n] = worst[n].max(bike.flat.high).max(bike.launch.high).max(bike.taps.high);
        }
        println!("{name}:\n  reference {}\n  throttle  {}", both[0].line(), both[1].line());
    }
    println!(
        "{total} bikes. Reference: lift in 3.5 s {}, hold {}, fell {}, flat out past 60 degrees {}, highest pitch {:.0}. \
         Throttle assist: lift {}, hold {}, fell {}, flat out past 60 degrees {}, highest pitch {:.0}, over a gear's cap by 6 degrees {over}",
        lifts[0], holds[0], falls[0], flat_loops[0], worst[0], lifts[1], holds[1], falls[1], flat_loops[1], worst[1]
    );
}
