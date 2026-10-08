//! Against the install (needs `NFSMW_GAME_DIR`; passes without checking otherwise): the four real maps parse and
//! evaluate, and the levels a drive reads are sane.

use std::sync::Arc;

use blackbox_carsound::{CarInput, EffectSignals, EngineOutput};
use blackbox_mixmap::{MixMap, Mixer};
use game_install::GameDir;

use super::*;
use crate::audio::mixer::levels::MAKEUP;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).unwrap())
}

#[test]
fn the_four_real_maps_parse_and_evaluate_without_nonsense() {
    let Some(dir) = install() else { return };
    for name in ["MAPOUTPUT", "MAPOUTPUTDRG", "MAPOUTPUT2CR", "MAPOUTPUT2DR"] {
        let bytes = dir.read(&format!("SOUND/MIXMAPS/{name}.mxb")).unwrap();
        let map = MixMap::parse(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(map.states.len(), 13, "{name}");
        assert_eq!(map.state(2).unwrap().masters.len(), 38, "{name}: player car channels");
        assert!(map.state(9).is_some_and(|s| s.controls.is_empty()), "{name}: the plane state is empty");
        // Every state once, every object attached: a hundred frames of everything at once.
        let mut mixer = Mixer::new(Arc::new(map), &[1; 13]);
        let counts = mixer.element_counts();
        assert!(counts.iter().all(|&c| c > 0), "{name}: {counts:?}");
        for frame in 0..100 {
            mixer.process(1.0 / 60.0);
            assert!(mixer.volume(ids::player_object(object::ENGINE_DUAL), 2).is_some(), "{name} frame {frame}");
        }
    }
}

fn frame_at<'a>(input: &'a CarInput, engine: &'a EngineOutput) -> Frame<'a> {
    Frame { input, engine, effects: EffectSignals::default(), master_volume: 25_500 }
}

/// Levels of the car at a steady speed on asphalt after a second of it.
fn cruise(mixer: &mut CarMixer, speed: f32) -> Levels {
    let input = CarInput { speed, throttle: 0.5, ..CarInput::default() };
    let engine = EngineOutput { physics_rpm: 4000.0, accelerating: true, ..EngineOutput::default() };
    let mut levels = Levels::unmixed();
    for _ in 0..60 {
        levels = mixer.update(1.0 / 60.0, &frame_at(&input, &engine));
    }
    levels
}

#[test]
fn cruising_levels_follow_the_map() {
    let Some(dir) = install() else { return };
    let mut mixer = CarMixer::load(&dir, true).expect("the map loads");
    eprintln!("speed m/s | engine road(asphalt,gravel) wind | clunk sweet turbo nitrous skid(f,s)");
    let mut rows = Vec::new();
    for speed in [0.0f32, 13.4, 26.8, 44.7, 67.0] {
        let l = cruise(&mut mixer, speed);
        eprintln!(
            "{speed:>5.1}     | {:.3} ({:.3} / {:.3}) ({:.3} / {:.3}) | {:.3} {:.3} {:.3} {:.3} {:.3} {:.3}",
            l.engine_volume,
            l.road[5],
            l.road[0],
            l.wind,
            l.wind_pitch,
            l.clunk_up,
            l.sweet_disengage,
            l.turbo_spool,
            l.nitrous,
            l.skid_side,
            l.skid_forward
        );
        rows.push((speed, l));
    }
    for (speed, l) in &rows {
        let all = [
            l.engine_volume,
            l.engine_pitch,
            l.clunk_up,
            l.clunk_down,
            l.sweet_engage,
            l.whine,
            l.turbo_spool,
            l.nitrous,
            l.skid_side,
            l.wind,
            l.landing,
        ];
        assert!(all.iter().all(|v| v.is_finite() && (0.0..=MAKEUP + 0.5).contains(v)), "{speed} m/s: {l:?}");
        assert!(l.road.iter().all(|v| v.is_finite() && *v >= 0.0), "{speed} m/s: {:?}", l.road);
    }
    // The engine reads the same near its centre of range at any speed: the map ducks it, never silences it.
    assert!(rows.iter().all(|(_, l)| l.engine_volume > 0.3 && l.engine_volume < 2.0), "{rows:?}");
    // The wind grows with speed.
    assert!(rows[4].1.wind > rows[1].1.wind, "{} {}", rows[4].1.wind, rows[1].1.wind);
    // The unmixed levels are the ones used when the map cannot be read.
    assert_eq!(Levels::unmixed().road[5], 0.35);
}

#[test]
fn the_pitch_of_the_engine_is_unity_at_rest() {
    let Some(dir) = install() else { return };
    let mut mixer = CarMixer::load(&dir, true).unwrap();
    let l = cruise(&mut mixer, 20.0);
    assert!((l.engine_pitch - 1.0).abs() < 0.1, "{}", l.engine_pitch);
}

#[test]
fn master_vol_scales_the_engine_level() {
    let Some(dir) = install() else { return };
    let level = |master_volume: u32| {
        let mut mixer = CarMixer::load(&dir, true).unwrap();
        let input = CarInput { speed: 20.0, ..CarInput::default() };
        let engine = EngineOutput { physics_rpm: 4000.0, ..EngineOutput::default() };
        let frame = Frame { input: &input, engine: &engine, effects: EffectSignals::default(), master_volume };
        (0..60).map(|_| mixer.update(1.0 / 60.0, &frame)).last().unwrap().engine_volume
    };
    let (quiet, loud) = (level(22_300), level(32_000));
    assert!(loud > quiet * 1.05, "Master_Vol 32000 gives {loud}, 22300 gives {quiet}");
}
