//! The effects controllers, driven the way the game drives them: the engine mix first, then the effects.

use super::*;
use crate::tests::common::{input, tuning};
use crate::{CarInput, CollisionTuning, EngineMixer, TICK_SECONDS, TurboTuning, WheelInput};

struct Rig {
    engine: EngineMixer,
    fx: EffectsMixer,
    landings: Vec<Landing>,
}

impl Rig {
    fn new(tuning: CarSoundTuning) -> Self {
        Self { engine: EngineMixer::new(&tuning), fx: EffectsMixer::new(&tuning), landings: Vec::new() }
    }

    /// `seconds` of the same telemetry; every command issued.
    fn run(&mut self, seconds: f32, input: CarInput) -> Vec<SoundCommand> {
        let mut commands = Vec::new();
        for _ in 0..(seconds / TICK_SECONDS).round() as usize {
            let engine = self.engine.update(TICK_SECONDS, &input);
            self.landings.extend(self.fx.update(TICK_SECONDS, &input, &engine, &mut commands));
        }
        commands
    }
}

fn turbo_tuning() -> CarSoundTuning {
    let turbo = TurboTuning {
        spool_volume: 10000,
        charge_time: 15.0,
        leak_rate: 0.5,
        blowoff_volume: [20000, 24000],
        blowoff_seconds: 1.0,
    };
    CarSoundTuning { turbo: Some(turbo), ..tuning() }
}

fn plays(commands: &[SoundCommand], want: impl Fn(SoundRef) -> bool) -> Vec<f32> {
    commands
        .iter()
        .filter_map(|c| match c {
            SoundCommand::Play { sound, volume, .. } if want(*sound) => Some(*volume),
            _ => None,
        })
        .collect()
}

fn loop_volumes(commands: &[SoundCommand], id: LoopId) -> Vec<f32> {
    commands
        .iter()
        .filter_map(|c| match c {
            SoundCommand::SetLoop { id: i, volume, .. } if *i == id => Some(*volume),
            _ => None,
        })
        .collect()
}

fn stops(commands: &[SoundCommand], id: LoopId) -> usize {
    commands.iter().filter(|c| matches!(c, SoundCommand::StopLoop(i) if *i == id)).count()
}

fn wheel(slip: f32, skid: f32, load: f32) -> WheelInput {
    WheelInput { slip, skid, load, tolerated_slip: 0.5, road_noise_loop: 5, ..WheelInput::default() }
}

fn driving(speed: f32) -> CarInput {
    CarInput { speed, wheels: [wheel(0.0, 0.0, 6000.0); 4], ..input(0.5, 0.3, 4) }
}

#[test]
fn an_up_shift_clunks_and_a_high_rpm_one_adds_the_sweeteners() {
    let mut rig = Rig::new(tuning());
    rig.run(1.5, input(0.9, 1.0, 2));
    let commands = rig.run(1.0, input(0.6, 1.0, 3));
    let clunks = plays(&commands, |s| s == SoundRef::GearClunk { up: true });
    assert_eq!(clunks.len(), 1, "one clunk");
    assert!(clunks[0] > 0.0 && clunks[0] <= 1.0);
    assert!(!plays(&commands, |s| matches!(s, SoundRef::Sweetener(_))).is_empty(), "a sweetener at 9000 RPM");
}

#[test]
fn the_clunk_is_louder_at_high_revs() {
    let clunk = |rpm_pct: f32| {
        let mut rig = Rig::new(tuning());
        rig.run(1.5, input(rpm_pct, 1.0, 2));
        let commands = rig.run(1.0, input(rpm_pct * 0.7, 1.0, 3));
        plays(&commands, |s| matches!(s, SoundRef::GearClunk { .. })).first().copied().unwrap_or(0.0)
    };
    assert!(clunk(0.9) > clunk(0.45) * 1.5, "{} vs {}", clunk(0.9), clunk(0.45));
}

#[test]
fn reverse_plays_a_whine_that_follows_the_revs_and_stops_after() {
    let mut rig = Rig::new(tuning());
    let commands = rig.run(1.0, input(0.5, 1.0, crate::GEAR_REVERSE));
    let volumes = loop_volumes(&commands, LoopId::Reverse);
    assert!(!volumes.is_empty() && volumes.last().unwrap() > &0.1, "{volumes:?}");
    let commands = rig.run(0.5, input(0.3, 0.0, crate::GEAR_FIRST));
    assert_eq!(stops(&commands, LoopId::Reverse), 1);
}

#[test]
fn the_brake_mash_plays_once_when_the_pedal_goes_to_the_floor_at_speed() {
    let mut rig = Rig::new(tuning());
    let rolling = CarInput { speed: 30.0, ..input(0.5, 0.0, 4) };
    rig.run(0.5, rolling);
    let commands = rig.run(0.5, CarInput { brake: 1.0, ..rolling });
    assert_eq!(plays(&commands, |s| s == SoundRef::BrakeMash).len(), 1);
    let again = rig.run(0.5, CarInput { brake: 1.0, ..rolling });
    assert!(plays(&again, |s| s == SoundRef::BrakeMash).is_empty(), "held, not pressed again");
    let slow = Rig::new(tuning()).run(0.5, CarInput { brake: 1.0, speed: 1.0, ..input(0.2, 0.0, 2) });
    assert!(plays(&slow, |s| s == SoundRef::BrakeMash).is_empty(), "not when crawling");
}

#[test]
fn a_turbo_spools_with_the_throttle_and_blows_off_when_it_lifts() {
    let mut rig = Rig::new(turbo_tuning());
    let spooling = rig.run(2.0, input(0.8, 1.0, 3));
    let volumes = loop_volumes(&spooling, LoopId::Turbo);
    assert!(volumes.last().unwrap() > volumes.first().unwrap(), "the spool grows");
    let lifted = rig.run(1.0, input(0.8, 0.0, 3));
    let blowoffs = plays(&lifted, |s| matches!(s, SoundRef::TurboBlowoff(_)));
    assert_eq!(blowoffs.len(), 1, "one blow-off");
    assert!(blowoffs[0] > 0.1);
    let later = rig.run(2.0, input(0.5, 0.0, 3));
    assert!(plays(&later, |s| matches!(s, SoundRef::TurboBlowoff(_))).is_empty());
    assert_eq!(stops(&lifted, LoopId::Turbo) + stops(&later, LoopId::Turbo), 1, "the loop ends with the blow-off");
}

#[test]
fn a_car_without_forced_induction_has_no_turbo_sound() {
    let mut rig = Rig::new(tuning());
    let commands = rig.run(2.0, input(0.8, 1.0, 3));
    assert!(commands.iter().all(|c| !matches!(c, SoundCommand::SetLoop { id: LoopId::Turbo, .. })));
}

#[test]
fn nitrous_loops_while_it_burns_and_purges_when_the_tank_runs_dry() {
    let mut rig = Rig::new(tuning());
    let burning = rig.run(1.0, CarInput { nos_active: true, ..input(0.7, 1.0, 3) });
    let pitches: Vec<f32> = burning
        .iter()
        .filter_map(|c| match c {
            SoundCommand::SetLoop { id: LoopId::Nitrous, pitch, .. } => Some(*pitch),
            _ => None,
        })
        .collect();
    assert!(pitches.last().unwrap() > pitches.first().unwrap(), "the pitch rises with the boost");
    let off = rig.run(0.2, CarInput { nos_active: false, nos_empty: true, ..input(0.7, 1.0, 3) });
    assert_eq!(stops(&off, LoopId::Nitrous), 1);
    assert_eq!(plays(&off, |s| s == SoundRef::Purge).len(), 1);
    let still = rig.run(0.5, CarInput { nos_empty: true, ..input(0.7, 1.0, 3) });
    assert!(plays(&still, |s| s == SoundRef::Purge).is_empty(), "one purge per emptying");
}

#[test]
fn sliding_tires_squeal_and_gripping_ones_do_not() {
    let mut rig = Rig::new(tuning());
    let grip = rig.run(1.0, driving(25.0));
    assert!(loop_volumes(&grip, LoopId::Skid(0)).is_empty(), "no skid while gripping");
    let sliding = CarInput { wheels: [wheel(0.0, -9.0, 9000.0); 4], ..driving(25.0) };
    let commands = rig.run(1.5, sliding);
    let front = loop_volumes(&commands, LoopId::Skid(0));
    assert!(front.last().unwrap() > &0.2, "{front:?}");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, SoundCommand::SetLoop { sound: SoundRef::Skid { sideways: true, .. }, .. }))
    );
    let after = rig.run(2.0, driving(25.0));
    assert_eq!(stops(&after, LoopId::Skid(0)), 1);
    let burnout = CarInput { wheels: [wheel(8.0, 0.0, 9000.0); 4], ..driving(5.0) };
    let commands = rig.run(1.0, burnout);
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, SoundCommand::SetLoop { sound: SoundRef::Skid { sideways: false, .. }, .. }))
    );
}

#[test]
fn the_skid_surface_is_the_highest_of_the_wheels_on_the_ground() {
    let mut rig = Rig::new(tuning());
    let mut wheels = [wheel(0.0, -9.0, 9000.0); 4];
    wheels[0].skid_surface = 1;
    let commands = rig.run(1.0, CarInput { wheels, ..driving(25.0) });
    assert!(commands.iter().any(|c| matches!(
        c,
        SoundCommand::SetLoop { id: LoopId::Skid(0), sound: SoundRef::Skid { surface: 1, .. }, .. }
    )));
}

#[test]
fn road_noise_follows_the_surface_and_speed_and_stops_in_the_air() {
    let mut rig = Rig::new(tuning());
    let slow = rig.run(0.5, driving(5.0));
    let fast = rig.run(0.5, driving(35.0));
    let (slow, fast) = (loop_volumes(&slow, LoopId::Road(0)), loop_volumes(&fast, LoopId::Road(0)));
    assert!(fast.last().unwrap() > slow.last().unwrap());
    assert!(rig.run(0.2, driving(35.0)).iter().all(
        |c| !matches!(c, SoundCommand::SetLoop { sound, .. } if matches!(sound, SoundRef::RoadNoise(n) if *n != 5))
    ));
    let airborne = CarInput { wheels: [WheelInput { on_ground: false, ..wheel(0.0, 0.0, 0.0) }; 4], ..driving(35.0) };
    let commands = rig.run(0.2, airborne);
    assert_eq!((stops(&commands, LoopId::Road(0)), stops(&commands, LoopId::Road(1))), (1, 1));
    let grass = CarInput {
        wheels: [WheelInput { road_noise_loop: crate::NO_ROAD_NOISE, ..wheel(0.0, 0.0, 6000.0) }; 4],
        ..driving(35.0)
    };
    assert!(rig.run(0.2, grass).iter().all(|c| !matches!(c, SoundCommand::SetLoop { id: LoopId::Road(_), .. })));
}

#[test]
fn wind_grows_with_speed_and_is_silent_when_slow() {
    let mut rig = Rig::new(tuning());
    let slow = rig.run(0.3, driving(10.0));
    let fast = rig.run(0.3, driving(40.0));
    assert!(loop_volumes(&fast, LoopId::Wind).last() > loop_volumes(&slow, LoopId::Wind).last());
    let stopped = rig.run(0.3, driving(0.5));
    assert_eq!(stops(&stopped, LoopId::Wind), 1);
}

#[test]
fn a_landing_after_a_jump_reports_how_hard() {
    let mut rig = Rig::new(tuning());
    rig.run(0.3, driving(30.0));
    let air = CarInput { wheels: [WheelInput { on_ground: false, ..wheel(0.0, 0.0, 0.0) }; 4], ..driving(30.0) };
    rig.run(0.6, air);
    assert!(rig.landings.is_empty());
    let down = CarInput { wheels: [WheelInput { compression: 0.15, ..wheel(0.0, 0.0, 9000.0) }; 4], ..driving(30.0) };
    rig.run(0.2, down);
    assert_eq!(rig.landings.len(), 1);
    assert!(rig.landings[0].magnitude > 0.3 && !rig.landings[0].hard);
    // A short hop does not count.
    let mut rig = Rig::new(tuning());
    rig.run(0.05, air);
    rig.run(0.2, down);
    assert!(rig.landings.is_empty());
}

fn tune(lengths: [u32; 4]) -> CollisionTuning {
    CollisionTuning { level_lengths: lengths, volumes: [10000, 15000, 20000, 25000], stream_thresholds: vec![] }
}

#[test]
fn an_impact_picks_a_level_by_magnitude_and_walks_the_samples() {
    let mut fx = EffectsMixer::new(&tuning());
    let t = tune([4, 4, 4, 4]);
    let hit = |m: f32| ImpactRequest { magnitude: m, light_wall: false, tuning: &t };
    let soft = fx.impact(&hit(0.0)).unwrap();
    let hard = fx.impact(&hit(1.0)).unwrap();
    assert_eq!((soft.level, hard.level), (0, 3));
    assert!(hard.volume > soft.volume);
    assert_eq!((soft.index, hard.index), (0, 1), "the counter moves on");
    let mid = fx.impact(&hit(0.5)).unwrap();
    assert!((1..=2).contains(&mid.level));
    // Two lists in use: the loudest hit uses the second.
    let two = tune([3, 3, 0, 0]);
    let loud = fx.impact(&ImpactRequest { magnitude: 1.0, light_wall: false, tuning: &two }).unwrap();
    assert_eq!(loud.level, 1);
}

#[test]
fn light_hits_on_smackables_and_empty_collections_are_dropped() {
    let mut fx = EffectsMixer::new(&tuning());
    let t = tune([2, 2, 2, 2]);
    assert!(fx.impact(&ImpactRequest { magnitude: 0.05, light_wall: true, tuning: &t }).is_none());
    assert!(fx.impact(&ImpactRequest { magnitude: 0.5, light_wall: true, tuning: &t }).is_some());
    assert!(fx.impact(&ImpactRequest { magnitude: 0.5, light_wall: false, tuning: &tune([0; 4]) }).is_none());
    assert!(fx.impact(&ImpactRequest { magnitude: f32::NAN, light_wall: false, tuning: &t }).is_none());
}

#[test]
fn a_scrape_loops_while_it_lasts_and_fades_out_after() {
    let mut rig = Rig::new(tuning());
    let mut commands = Vec::new();
    let engine = rig.engine.update(TICK_SECONDS, &driving(20.0));
    for _ in 0..30 {
        rig.fx.scrape(Some((ScrapeKind::Wall, 0.8)));
        rig.fx.update(TICK_SECONDS, &driving(20.0), &engine, &mut commands);
    }
    let volumes = loop_volumes(&commands, LoopId::Scrape);
    assert!(volumes.last().unwrap() > &0.7);
    let mut after = Vec::new();
    for _ in 0..30 {
        rig.fx.scrape(None);
        rig.fx.update(TICK_SECONDS, &driving(20.0), &engine, &mut after);
    }
    let fade = loop_volumes(&after, LoopId::Scrape);
    assert!(fade.windows(2).all(|w| w[1] <= w[0]), "fading");
    assert_eq!(stops(&after, LoopId::Scrape), 1);
}

#[test]
fn bad_telemetry_never_makes_a_bad_number() {
    let mut rig = Rig::new(turbo_tuning());
    let nan = f32::NAN;
    let wheel = WheelInput {
        slip: nan,
        skid: f32::INFINITY,
        load: nan,
        compression: nan,
        traction_usage: nan,
        ..WheelInput::default()
    };
    let bad = CarInput { speed: nan, wheels: [wheel; 4], up_dot: nan, ..input(0.5, 1.0, 3) };
    let commands = rig.run(1.0, bad);
    for c in commands {
        let (volume, pitch) = match c {
            SoundCommand::Play { volume, pitch, .. } | SoundCommand::SetLoop { volume, pitch, .. } => (volume, pitch),
            SoundCommand::StopLoop(_) => continue,
        };
        assert!(volume.is_finite() && pitch.is_finite(), "{c:?}");
    }
}
