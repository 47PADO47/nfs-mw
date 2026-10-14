use super::*;
use blackbox_carsound::NO_ROAD_NOISE;

fn input(id: u8) -> CarInput {
    let mut input = CarInput { speed: 30.0, ..Default::default() };
    for wheel in &mut input.wheels {
        wheel.on_ground = true;
        wheel.road_noise_loop = id;
    }
    input
}

fn commands(id: u8) -> Vec<SoundCommand> {
    (0..2)
        .map(|side| SoundCommand::SetLoop {
            id: LoopId::Road(side),
            sound: SoundRef::RoadNoise(id),
            volume: 0.7,
            pitch: 0.9,
        })
        .collect()
}

#[test]
fn weather_matches_four_original_instruction_vectors() {
    for (mph, volume) in [(0.0, 32767), (35.0, 20479), (70.0, 8191), (100.0, 8191)] {
        assert_eq!(wind::weather(mph / 2.23699, 32767), [volume, 0, 4000, 0]);
    }
    assert_eq!(wind::weather(f32::NAN, -1), [0, 0, 4000, 0]);
}

#[test]
fn transitions_use_pc_gains_and_pitch_endpoints() {
    let levels = AmbientLevels { road_transition: [32767; 3], ..Default::default() };
    for (id, base) in [(0, 12000), (1, 12000), (2, 24500), (3, 32767)] {
        let low = road::transition(id, 0.0, 0, &levels);
        assert_eq!(low[1], road::multiply(road::multiply(base, 32767), 32767));
        assert_eq!(low[2], 3600);
        assert_eq!(low[4], 1);
        assert_eq!(road::transition(id, 100.0, 1, &levels)[2], 4400);
    }
}

#[test]
fn terrain_transition_uses_last_changed_wheel_and_rejects_dontplay() {
    let mut history = road::History::default();
    let mut context = RoadContext::default();
    let initial = input(0);
    assert_eq!(history.observe(&initial, Some(&context)), [None; 2]);
    for (w, on) in [(0, 0), (3, 1)] {
        context.terrain[w] = RoadTerrain { key: 10 + w as u32, transition_on: on, ..Default::default() };
    }
    let changed = input(1);
    assert_eq!(history.observe(&changed, Some(&context)), [Some(1), None]);
    context.terrain[3].key += 1;
    context.terrain[3].transition_on = 4;
    assert_eq!(history.observe(&input(2), Some(&context)), [None; 2]);
}

#[test]
fn transition_falls_back_to_off_and_requires_ground() {
    let mut history = road::History::default();
    let mut old = RoadContext::default();
    old.terrain[0].transition_off = 0;
    history.observe(&input(0), Some(&old));
    let mut new = old;
    new.terrain[0].key = 1;
    assert_eq!(history.observe(&input(1), Some(&new)), [Some(0), None]);
    new.terrain[0].key = 2;
    let mut airborne = input(2);
    airborne.wheels[0].on_ground = false;
    airborne.wheels[3].on_ground = false;
    assert_eq!(history.observe(&airborne, Some(&new)), [None; 2]);
}

#[test]
fn damage_edges_take_precedence_without_retriggering() {
    let mut history = road::History::default();
    let mut context = RoadContext::default();
    history.observe(&input(0), Some(&context));
    context.damage[0] = 1;
    assert_eq!(history.observe(&input(0), Some(&context)), [Some(2), None]);
    assert_eq!(history.observe(&input(0), Some(&context)), [None; 2]);
    context.damage[3] = 2;
    assert_eq!(history.observe(&input(0), Some(&context)), [Some(3), None]);
}

#[test]
fn stitch_timer_is_signed_milliseconds_and_picks_last_free_slot() {
    let mut timer = 16;
    assert_eq!(stitch::repeat_slot(&mut timer, 100, 0.0169, [true; 2]), None);
    assert_eq!(timer, 0);
    assert_eq!(stitch::repeat_slot(&mut timer, 100, 0.001, [true; 2]), Some(1));
    assert_eq!(timer, 100);
    timer = 0;
    assert_eq!(stitch::repeat_slot(&mut timer, 100, 0.001, [true, false]), Some(0));
    timer = 0;
    assert_eq!(stitch::repeat_slot(&mut timer, 100, 0.001, [false; 2]), None);
    assert_eq!(timer, 100);
}

#[test]
fn stitch_repeat_cadence_retains_debt_at_all_render_rates_and_stalls() {
    fn expiries(deltas: impl IntoIterator<Item = f32>) -> (usize, usize) {
        let mut clock = stitch::RepeatClock::default();
        let mut timer = 100;
        let mut ticks = 0;
        let mut repeats = 0;
        for delta in deltas {
            for _ in 0..clock.advance(delta) {
                ticks += 1;
                repeats += usize::from(stitch::repeat_slot(&mut timer, 100, 1.0 / 60.0, [true; 2]).is_some());
            }
        }
        (ticks, repeats)
    }
    let reference = expiries(std::iter::repeat_n(1.0 / 60.0, 180));
    assert_eq!(reference, (180, 25));
    for fps in [5, 30, 60, 120, 144, 240, 360] {
        assert_eq!(expiries(std::iter::repeat_n(1.0 / fps as f32, 3 * fps)), reference, "{fps} fps");
    }
    assert_eq!(expiries([0.1, 0.7, 0.02, 0.98, 1.2, 0.0]), reference);
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_road_graph_replaces_releases_and_keeps_unchanged_instance() {
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let mut playback = AmbientPlayback::default();
    playback.update(&mut audio, &input(0), &commands(0), Default::default(), 1.0 / 60.0);
    assert!(playback.available());
    assert!(playback.owns_loop(LoopId::Wind));
    playback.select(&mut audio, 0, Some(0));
    assert!(playback.road[0].retired.is_empty());
    playback.select(&mut audio, 0, Some(1));
    assert_eq!(playback.road[0].retired.len(), 1);
    assert!(playback.road[0].module.is_some());
    assert_eq!(playback.road[0].params, road::params(1, 0, 4096, 0, false, 0));
    playback.select(&mut audio, 0, None);
    assert_eq!(playback.road[0].retired.len(), 2);
    assert_eq!(playback.road[0].retired[1].1, road::params(1, 0, 4096, 0, false, 0));
    assert!(playback.road[0].retired[1].0.finished(), "unticked replacement has no host resources to release");
    assert!(playback.road[0].module.is_none());
    let mut silent = input(NO_ROAD_NOISE);
    silent.speed = 0.0;
    for _ in 0..180 {
        playback.update(&mut audio, &silent, &Vec::new(), Default::default(), 1.0 / 60.0);
        audio.render_replay(800);
    }
    assert!(
        playback.road[0].retired.is_empty(),
        "authored release residue: {:?}",
        playback.road[0]
            .retired
            .iter()
            .map(|(module, params)| (params.first(), module.voice_counts()))
            .collect::<Vec<_>>()
    );
    assert!(audio.missing.is_empty());
    playback.wind.module = None;
    playback.wind.failed = true;
    playback.road[1].module = None;
    playback.road[1].failed = true;
    playback.ids[1] = Some(1);
    let fallback = vec![
        SoundCommand::SetLoop { id: LoopId::Wind, sound: SoundRef::Wind, volume: 0.5, pitch: 1.0 },
        SoundCommand::SetLoop { id: LoopId::Road(1), sound: SoundRef::RoadNoise(1), volume: 0.5, pitch: 1.0 },
    ];
    playback.update(&mut audio, &input(1), &fallback, Default::default(), 1.0 / 60.0);
    assert_eq!(fallback.len(), 2, "actual failures preserve generic loop commands");
    assert!(!playback.owns_loop(LoopId::Wind));
    assert!(!playback.owns_loop(LoopId::Road(1)));
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_transition_weather_and_stitch_profiles_are_usable() {
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let mut playback = AmbientPlayback::default();
    let levels = AmbientLevels {
        wind: [0, 0, 0, 0, 0, 4096],
        weather: 32767,
        road_transition: [32767; 3],
        road_stitch: 32767,
        ..Default::default()
    };
    let mut context = RoadContext::default();
    playback.update_with_context(&mut audio, &input(0), &commands(0), levels, 1.0 / 60.0, Some(&context));
    context.terrain[0] = RoadTerrain { key: 1, transition_on: 0, ..Default::default() };
    playback.update_with_context(&mut audio, &input(1), &commands(1), levels, 1.0 / 60.0, Some(&context));
    assert!(!playback.transition[0].failed);
    assert_eq!(playback.transition[0].params[0], 0);
    assert!(!playback.weather.failed && playback.weather.module.is_some());
    assert_eq!(playback.weather.params, wind::weather(30.0, 32767));
    assert!(audio.missing.is_empty());
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let mut playback = AmbientPlayback::default();
    let levels = AmbientLevels { wind: [0, 0, 0, 0, 0, 4096], road: [0; 8], road_stitch: 32767, ..Default::default() };
    playback.select(&mut audio, 0, Some(8));
    assert!(playback.stitches[0].has_profile(), "common authored road-stitch profile must resolve");
    assert!(playback.owns_loop(LoopId::Road(0)), "resolved profile must also load its piece modules");
    let mut recorded = Vec::new();
    for _ in 0..120 {
        playback.update(&mut audio, &input(8), &commands(8), levels, 1.0 / 60.0);
        recorded.extend_from_slice(audio.render_replay(800));
    }
    assert!(playback.owns_loop(LoopId::Road(0)), "piece updates must remain usable");
    assert!(
        crate::audio::capture::levels(&recorded).rms > 0.0001,
        "stitch alone must produce PCM with wind/weather/wet/ordinary gains zero"
    );
    assert!(audio.missing.is_empty());
}

#[test]
#[ignore = "requires installed audio attributes; set NFSMW_GAME_DIR"]
fn installed_road_surface_selectors_decode_enum_transitions_and_secondary_keys() {
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let database = audio.database().unwrap();
    let surfaces = nfsmw_data::car::physics::SurfaceTable::from_database(&database);
    for name in ["dirt", "gravel"] {
        let surface = surfaces.audio(nfsmw_data::car::physics::SurfaceTable::hash_of(name));
        assert!(surface.secondary, "{name} exact-key secondary producer");
        assert_ne!(surface.road_loop, NO_ROAD_NOISE, "{name} has an authored loop");
    }
    let mut actual_transitions = 0;
    for collection in database.collections_of("simsurface") {
        let decoded = surfaces.audio(collection.key());
        assert_eq!(decoded.transition_on, collection.get_u32("Aud_RoadNoise_TransON").map_or(-1, |v| v as i32));
        assert_eq!(decoded.transition_off, collection.get_u32("Aud_RoadNoise_TransOFF").map_or(-1, |v| v as i32));
        actual_transitions += usize::from(decoded.transition_on >= 0 || decoded.transition_off >= 0);
    }
    assert!(actual_transitions > 0, "installed transition enums must not all silently default to NONE");
}
