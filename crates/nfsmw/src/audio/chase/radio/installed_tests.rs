//! The chase radio against the user's own install: every event the machine can ask for exists, and a scripted chase
//! timeline driven through the machine into the real recordings gives requests that all assemble to audio.
//!
//! `NFSMW_GAME_DIR="D:/..." cargo test --release -p nfsmw chase::radio::installed -- --ignored --test-threads=1`
//!
//! This proves the requests are well formed (a sentence version exists for the situation, the takes decode); it says
//! nothing about how the result sounds next to the original.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use blackbox_attrib::Database;
use blackbox_pursuit::speech::{Line, Output};
use blackbox_pursuit::{EndReason, Infraction, InfractionLog};
use ea_audio::Pcm;
use game_install::GameDir;
use nfsmw_data::speech_chase::{chase_tune, level_speech};

use super::params::event_name;
use super::{ChaseRadio, LineIds, speech_params};
use crate::audio::chase::AirSupport;
use crate::audio::speech::params::{lib_params, values};
use crate::audio::speech::random::Rng;
use crate::audio::speech::{Context, Files, Lines, Sink, Speech};
use crate::audio::{CopFeed, Formation, PursuitFeed, RoadblockFeed, Vehicle};

/// A sound output that sounds for the length of each line, advanced by the test's clock.
struct Timed(Rc<Cell<f32>>);

impl Sink for Timed {
    fn busy(&self) -> bool {
        self.0.get() > 0.0
    }

    fn play(&mut self, pcm: &Pcm) -> Result<(), String> {
        self.0.set(pcm.frames() as f32 / pcm.sample_rate.max(1) as f32);
        Ok(())
    }

    fn stop(&mut self) {
        self.0.set(0.0);
    }

    fn set_volume(&mut self, _amplitude: f32) {}
}

fn open() -> Option<(GameDir, Database)> {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?)).ok()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    Some((dir, db))
}

fn files(dir: &GameDir, seed: u64) -> Files {
    let idx = dir.read("SOUND/SPEECH/copspeech.idx").unwrap();
    let evt = dir.read("SOUND/SPEECH/copspeech.evt").unwrap();
    let big = dir.resolve("SOUND/SPEECH/copspeech.big").unwrap();
    Files::open(&idx, &evt, big, Rng::new(seed)).unwrap()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_line_the_machine_asks_for_is_an_event_of_the_install() {
    let Some((_, db)) = open() else { return };
    let dispatcher = crate::audio::speech::Dispatcher::new(
        nfsmw_data::speech::events(&db),
        nfsmw_data::speech::tune(&db),
        Rng::new(1),
    );
    let ids = LineIds::new(&dispatcher);
    let missing: Vec<&str> = ids.missing().into_iter().map(event_name).collect();
    assert!(missing.is_empty(), "events missing from the install: {missing:?}");
    for &line in Line::ALL {
        let event = dispatcher.event(ids.id(line).unwrap()).unwrap();
        println!("{line:?} -> {} ({})", event.name, event.id);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_installed_speech_tuning_is_what_the_machine_assumes() {
    let Some((_, db)) = open() else { return };
    assert_eq!(chase_tune(&db), nfsmw_data::speech_chase::ChaseTune::default());
    let hot = level_speech(&db, 5);
    println!("heat 5: {hot:?}");
    assert!(hot.roadblocks, "a heat 5 chase has roadblocks");
}

/// A feed of the scripted chase at `secs`.
fn feed(secs: f32, edit: impl FnOnce(&mut PursuitFeed)) -> PursuitFeed {
    let mut f =
        PursuitFeed { heat: 1, intensity: 1.0, secs, secs_to_evade: 60.0, player_mph: 85.0, ..Default::default() };
    f.speech.player.position = [0.0, 0.0, 0.0];
    f.speech.player.velocity_dir = [0.0, 0.0, 1.0];
    f.speech.player.forward = [0.0, 0.0, 1.0];
    f.speech.player.road = Some(5);
    f.speech.player.car_type = values::car_type::PORSCHE;
    f.speech.player.car_colour = values::car_color::RED;
    f.speech.status.formation = Formation::BoxIn;
    f.speech.status.infractions = {
        let mut log = InfractionLog::default();
        log.record(Infraction::Speeding, &blackbox_pursuit::Fines::new([0; 8]));
        log
    };
    let cops = [(1, 30.0, Vehicle::Patrol, 12), (2, 55.0, Vehicle::Suv, 40), (3, 80.0, Vehicle::Cross, 70)];
    for (id, distance, vehicle, road) in cops {
        f.speech.push_cop(CopFeed {
            id,
            active: true,
            health: 1.0,
            distance,
            speed_mph: 80.0,
            position: [distance, 0.0, 0.0],
            los: true,
            in_view: true,
            in_formation: true,
            in_position: id == 1,
            offset: [3.0, 2.0],
            vehicle,
            road,
            ..CopFeed::default()
        });
    }
    edit(&mut f);
    f
}

fn lose(f: &mut PursuitFeed, secs_to_evade: f32) {
    f.intensity = 0.4;
    f.secs_to_evade = secs_to_evade;
    f.speech.status.cooling_down = true;
    for i in 0..usize::from(f.speech.cop_count) {
        f.speech.cops[i].los = false;
        f.speech.cops[i].in_view = false;
    }
}

/// What a scripted chase gave the radio.
struct Played {
    asked: BTreeSet<Line>,
    checked: usize,
    failures: Vec<String>,
    /// Requests the install has no recording for, by the data's own rules.
    unheard: Vec<&'static str>,
    said: u32,
    status: String,
}

/// Runs `at` (the feed and the end reason at a time) for `secs` through the machine into the real recordings; every
/// request must assemble to audio.
fn play(dir: &GameDir, db: &Database, secs: f32, at: &dyn Fn(f32) -> (Option<PursuitFeed>, EndReason)) -> Played {
    let events = nfsmw_data::speech::events(db);
    let tune = nfsmw_data::speech::tune(db);
    let sounding = Rc::new(Cell::new(0.0));
    let mut speech =
        Speech::new(events.clone(), tune, Box::new(files(dir, 3)), Box::new(Timed(sounding.clone())), Rng::new(5));
    // The recordings the requests are checked against: their own repeat memory, apart from the one that speaks.
    let mut checker = files(dir, 4);
    let ids = {
        let d = crate::audio::speech::Dispatcher::new(events, tune, Rng::new(1));
        LineIds::new(&d)
    };
    let mut radio = ChaseRadio::new(11);
    radio.set_tune(&chase_tune(db));
    let air = AirSupport::default();
    let dt = 0.1;
    let mut played = Played {
        asked: BTreeSet::new(),
        checked: 0,
        failures: Vec::new(),
        unheard: Vec::new(),
        said: 0,
        status: String::new(),
    };
    for step in 0..(secs / dt) as u32 {
        let (chase, ended) = at(step as f32 * dt);
        let level = level_speech(db, chase.as_ref().map_or(1, |c| c.heat));
        let heat = chase.as_ref().map_or(0, |c| c.heat as i32);
        speech.set_context(Context { heat, player_speed: chase.as_ref().map_or(0.0, |c| c.player_mph) });
        speech.dispatcher_mut().set_pursuit_secs(chase.as_ref().map_or(0.0, |c| c.secs));
        let out: Output = radio.run(dt, chase.as_ref(), ended, &level, Some(speech.dispatcher_mut()));
        for request in &out.requests {
            played.asked.insert(request.line);
            let name = event_name(request.line);
            let Some(id) = ids.id(request.line) else { continue };
            let sp = speech_params(request.line, &request.params);
            let numbered = lib_params(name, u16::from(request.speaker), &sp);
            match checker.take(id, u16::from(request.speaker), &numbered) {
                Ok(pcm) if pcm.frames() > 0 => played.checked += 1,
                // Dispatch's "no" to a backup request has a recording only for the type "no units" (event 184); a denied
                // request names Rhino or super pursuit units, so the original has nothing to say for it either.
                Err(_) if request.line == Line::DispBackupReply && request.params.yes == Some(false) => {
                    played.unheard.push(name);
                }
                Ok(_) => played.failures.push(format!("{name}: silent")),
                Err(e) => played.failures.push(format!("{name} {:?} by {}: {e}", request.params, request.speaker)),
            }
        }
        radio.apply(out, &mut speech, &air);
        speech.update(dt);
        sounding.set((sounding.get() - dt).max(0.0));
    }
    played.said = speech.said();
    played.status = radio.status();
    played
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_scripted_chase_timeline_is_said_with_the_real_recordings() {
    let Some((dir, db)) = open() else { return };
    let state = Script;
    let played = play(&dir, &db, 200.0, &|t| state.at(t));
    println!("{} requests assembled; lines asked for: {:?}", played.checked, played.asked);
    println!("{}", played.status);
    let failures = &played.failures;
    assert!(
        failures.is_empty(),
        "{} requests did not assemble:
{}",
        failures.len(),
        failures.join(
            "
"
        )
    );
    let expected = [
        Line::AttemptVehicleStop,
        Line::GoAhead,
        Line::VehicleReport,
        Line::InitiateStrategy,
        Line::BullhornArrest,
        Line::CallForRoadblock,
        Line::DispRoadblockReply,
        Line::DispBreakAway,
        Line::QuadrantForming,
        Line::ReinitiatePursuit,
        Line::LocationReport,
    ];
    for line in expected {
        assert!(played.asked.contains(&line), "{line:?} was never asked for; asked {:?}", played.asked);
    }
    assert!(played.said > 8, "the dispatcher said only {} lines", played.said);
}

/// The scripted chase: sight lost twice, a hot stretch with a bullhorn, a roadblock, and an escape.
#[derive(Default)]
struct Script;

impl Script {
    /// The feed and the end reason at time `t`.
    fn at(&self, t: f32) -> (Option<PursuitFeed>, EndReason) {
        if t >= 150.0 {
            return (None, EndReason::Evaded);
        }
        let chase = feed(t, |f| {
            if (22.0..30.0).contains(&t) {
                f.speech.status.formation = Formation::Pit;
                f.speech.status.finisher_active = true;
            }
            if t >= 30.0 {
                f.heat = 3;
            }
            if (45.0..50.0).contains(&t) {
                f.speech.status.busted = 0.9;
            }
            if (55.0..85.0).contains(&t) {
                lose(f, 60.0 - (t - 55.0));
            }
            if (95.0..115.0).contains(&t) {
                f.roadblock_active = true;
                f.speech.roadblock = Some(RoadblockFeed { id: 1, visual: t > 100.0, spike_strips: 1, distance: 150.0 });
            }
            if (120.0..150.0).contains(&t) {
                lose(f, (150.0 - t).max(0.0));
            }
        });
        (Some(chase), EndReason::None)
    }
}

/// The original's off-road moments read a parameter-map layer named "Speech" under the car (the SPAM id, integer
/// field 1 of the selected set). The installed track has it: 26 sets whose ids are 25 distinct values from 0 to 27 and
/// the default -1. What the id means (which of the 24 named places it stands for) is the original's `GetSPAMLocation`
/// switch, which is not available to this port, so the layer is read here but not yet used.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_installed_track_has_the_speech_parameter_map_with_its_place_ids() {
    let Some((dir, _)) = open() else { return };
    let metadata = nfsmw_data::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
    let layers = blackbox_paramap::Layer::read_all(&metadata).unwrap();
    let layer = layers.iter().find(|l| l.name_hash == blackbox_hash::bstring_hash("Speech")).expect("the Speech layer");
    assert_eq!(layer.set_count(), 26);
    let mut ids: Vec<i32> = (0..layer.set_count()).map(|set| layer.integer(set, 1).expect("an integer id")).collect();
    assert!((0..layer.set_count()).all(|set| layer.integer(set, 0) == Some(0)));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 26, "every set has its own id: {ids:?}");
    assert_eq!((ids[0], ids[1], *ids.last().unwrap()), (-1, 0, 27));
    // The map answers somewhere on the track.
    assert!(layer.set_at(0.0, 0.0).is_some());
}

/// The world's events at the times of a scripted chase, with the feed edits that go with them.
struct WorldScript {
    log: std::cell::RefCell<(crate::audio::EventLog, usize)>,
}

impl WorldScript {
    fn schedule() -> Vec<(f32, crate::audio::WorldEvent)> {
        use crate::audio::{Crash, WorldEvent};
        use blackbox_pursuit::speech::{Backup, Impact};
        const MPH: f32 = 0.447_04;
        let crash = |delta_mph: f32, closing: f32, before: f32, after: f32| Crash {
            delta_v: delta_mph * MPH,
            closing,
            speed_before: before,
            speed_after: after,
        };
        let mut events: Vec<(f32, WorldEvent)> = vec![
            (20.0, WorldEvent::PlayerHitWorld(crash(24.0, 25.0, 35.0, 5.0))),
            (25.0, WorldEvent::Airborne { altitude: 2.0, secs: 1.6 }),
            (32.5, WorldEvent::BlewByCop { cop: 1 }),
            (
                35.0,
                WorldEvent::PlayerHitCop {
                    cop: 1,
                    crash: crash(30.0, 15.0, 30.0, 25.0),
                    impact: Impact::TBone,
                    deliberate: true,
                },
            ),
            (109.5, WorldEvent::CopHitTraffic { cop: 2, closing: 14.0 }),
            (118.5, WorldEvent::CopHitWorld { cop: 1, delta_v: 28.0 * MPH, closing: 14.0 }),
            (44.0, WorldEvent::PlayerHitTraffic { crash: crash(25.0, 20.0, 30.0, 6.0), semi: true }),
            (52.0, WorldEvent::Tunnel { inside: true }),
            (99.0, WorldEvent::GasStation { by_player: true, at: [20.0, 0.0, 0.0] }),
            (97.0, WorldEvent::RoadblockBuilt { spike_offset: Some(2) }),
            (103.0, WorldEvent::TireBlown),
            (108.0, WorldEvent::RoadblockDodged),
            (117.0, WorldEvent::BackupDenied(Backup::Rhino)),
        ];
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        events
    }

    fn at(&self, t: f32) -> (Option<PursuitFeed>, EndReason) {
        let schedule = Self::schedule();
        let mut state = self.log.borrow_mut();
        while state.1 < schedule.len() && schedule[state.1].0 <= t {
            let next = state.1;
            state.0.push(schedule[next].1);
            state.1 += 1;
        }
        let log = state.0;
        let chase = Script.at(t).0.map(|mut f| {
            f.speech.events = log;
            if (32.0..34.0).contains(&t) {
                f.speech.cops[0].speed_mph = 20.0;
                f.speech.cops[1].los = false;
                f.speech.cops[2].los = false;
            }
            if (50.0..58.0).contains(&t) {
                f.speech.heli =
                    Some(crate::audio::HeliFeed { id: 99, active: true, los: true, distance: 80.0, fuel_secs: 90.0 });
            }
            if (104.0..107.0).contains(&t) {
                f.speech.player.up_y = -0.8;
            }
            f.speech.player.road_forward = Some([0.0, 0.0, 1.0]);
            if (100.0..100.6).contains(&t) {
                f.player_mph = 100.0;
                f.speech.player.brake = 1.0;
            }
            if (100.6..102.0).contains(&t) {
                f.player_mph = 20.0;
                f.speech.player.brake = 1.0;
            }
            if t >= 112.0 {
                f.speech.cops[2].destroyed = true;
                f.speech.cops[2].active = false;
                f.cops_destroyed = 1;
            }
            if t >= 116.0 {
                f.backup_eta = 45.0;
            }
            f
        });
        (chase, Script.at(t).1)
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_worlds_events_in_a_scripted_chase_all_assemble_to_audio() {
    let Some((dir, db)) = open() else { return };
    let script = WorldScript { log: Default::default() };
    let played = play(&dir, &db, 150.0, &|t| script.at(t));
    println!("{} requests assembled; lines asked for: {:?}", played.checked, played.asked);
    println!("{}", played.status);
    let failures = &played.failures;
    assert!(failures.is_empty(), "{} requests did not assemble:\n{}", failures.len(), failures.join("\n"));
    let expected = [
        Line::CollisionWorld,
        Line::CollWorldAir,
        Line::Spotted,
        Line::Bailout,
        Line::CollWorldFlip,
        Line::SuspectBrake,
        Line::UnitDisabled,
        Line::CallForEv,
        Line::HeliHazardAlert,
    ];
    for line in expected {
        assert!(played.asked.contains(&line), "{line:?} was never asked for; asked {:?}", played.asked);
    }
}
