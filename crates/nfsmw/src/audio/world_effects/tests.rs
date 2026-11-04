use super::*;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_world_effect_audio_uses_authored_event_stitches_and_admission() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let effect = blackbox_attrib::vlt_hash("sfx_tollbooth_brk");
    let db = audio.database().unwrap();
    let definition = EffectSound::read(&db, effect, CollisionZone::Default).unwrap();
    assert!(definition.sound.has("EVENT") && definition.sound.has("SMOKABLE"));
    assert_eq!(definition.sound.levels[0].len(), 3);
    assert!(definition.sound.levels[1..].iter().all(Vec::is_empty));
    let request =
        WorldEffectAudio { effect, zone: CollisionZone::Default, position: Vec3::ZERO, magnitude: 1.0, moment: None };
    let view = AudioView::default();
    for control in [
        WorldEffectAudio { effect: 0, ..request },
        WorldEffectAudio { magnitude: 0.0, ..request },
        WorldEffectAudio { position: Vec3::splat(definition.cull_distance + 1.0), ..request },
        WorldEffectAudio { magnitude: f32::NAN, ..request },
    ] {
        audio.update_world_effect_audio(vec![control], view, 1.0 / 60.0);
        assert_eq!(audio.world_effects.playback.active_count(), 0);
    }
    audio.update_world_effect_audio(vec![request], view, 1.0 / 60.0);
    assert!(audio.world_effects.playback.active_count() > 0);
    let mut peak = 0.0_f32;
    for _ in 0..180 {
        audio.update_world_effect_audio(Vec::new(), view, 1.0 / 60.0);
        for sample in audio.render_replay(800) {
            peak = peak.max(sample.abs());
        }
    }
    assert!(peak > 0.001, "the authored tollbooth event must produce PCM");
    assert_eq!(audio.world_effects.playback.active_count(), 0, "authored modules release after finishing");
}

/// The booth's break sound is an `EVENT` impact whose `StreamSweetner` names the positioned `tollbooth` moment: the
/// break streams that recording with its stitch, except while world data still streams in.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_tollbooth_break_streams_its_authored_moment_sweetener() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let effect = blackbox_attrib::vlt_hash("sfx_tollbooth_brk");
    let moment = blackbox_attrib::vlt_hash("tollbooth");
    let db = audio.database().unwrap();
    let definition = EffectSound::read(&db, effect, CollisionZone::Default).unwrap();
    assert_eq!(definition.sound.sweeteners, [StreamSweetener { moment, threshold: 0 }]);
    let authored = nfsmw_data::sound::MomentSound::by_key(&db, moment).expect("installed tollbooth moment");
    assert!(authored.positioned);
    drop(db);
    let request = WorldEffectAudio {
        effect,
        zone: CollisionZone::Default,
        position: Vec3::X * 4.0,
        magnitude: 1.0,
        moment: None,
    };
    let view = AudioView::default();
    let loading = AudioView { weather_streaming: true, ..view };
    audio.update_world_effect_audio(vec![request], loading, 1.0 / 60.0);
    assert!(audio.world_effects.playback.active_count() > 0, "the stitch still plays while the world loads");
    assert_eq!(audio.world_effects.sweetener.playing(), None, "the original skips the stream while loading");
    audio.stop_world_effect_audio();
    audio.update_world_effect_audio(vec![request], view, 1.0 / 60.0);
    assert_eq!(audio.world_effects.sweetener.playing(), Some(moment));
    let started = audio.world_effects.sweetener.plays();
    audio.update_world_effect_audio(vec![request], view, 1.0 / 60.0);
    assert_eq!(audio.world_effects.sweetener.plays(), started + 1, "the next booth restarts the booth crash");
    audio.stop_world_effect_audio();
    let mut energy = 0.0_f64;
    audio.update_world_effect_audio(vec![request], view, 1.0 / 60.0);
    // Past the stitch: what is left is the moment recording.
    for frame in 0..240 {
        audio.update_world_effect_audio(Vec::new(), view, 1.0 / 60.0);
        let stitch_done = audio.world_effects.playback.active_count() == 0;
        let samples = audio.render_replay(800);
        if frame >= 60 && stitch_done {
            energy += samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
        }
    }
    assert!(energy > 0.001, "the tollbooth moment reaches replay PCM after the stitch ends");
    audio.stop_world_effect_audio();
    assert_eq!(audio.world_effects.sweetener.playing(), None, "scene teardown stops the stream");
}

/// A car-to-car hit the player's car takes part in streams the `car2car` moment its `TWO_CAR` impact names, as the
/// original drive capture shows on every hard car crash; impacts without `TWO_CAR` or `EVENT` stream nothing.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_two_car_impacts_stream_the_car2car_moment() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let car2car = blackbox_attrib::vlt_hash("car2car");
    let db = audio.database().unwrap();
    let impacts = nfsmw_data::sound::all_impacts(&db);
    drop(db);
    let streaming: Vec<_> = impacts.iter().filter(|s| !s.sweeteners.is_empty()).collect();
    let two_car: Vec<_> = streaming.iter().filter(|s| s.has("TWO_CAR")).collect();
    assert!(!two_car.is_empty(), "installed car-to-car impacts carry sweeteners");
    assert!(two_car.iter().all(|s| s.sweeteners.iter().any(|w| w.moment == car2car)), "{two_car:#?}");
    eprintln!("{:#?}", two_car.iter().map(|s| (&s.name, &s.description, &s.sweeteners)).collect::<Vec<_>>());
    // The capture's crashes streamed it at intensity 115 (magnitude 0.9).
    let crash = **two_car.iter().min_by_key(|s| s.sweeteners.iter().map(|w| w.threshold).min()).unwrap();
    let view = AudioView::default();
    let point = Vec3::X * 4.0;
    audio.play_impact_sweetener(crash, 0.0, point, view);
    let threshold = crash.sweeteners.iter().map(|s| s.threshold).min().unwrap();
    assert_eq!(audio.world_effects.sweetener.playing().is_some(), threshold == 0, "below every threshold");
    audio.play_impact_sweetener(crash, 0.9, point, view);
    assert_eq!(audio.world_effects.sweetener.playing(), Some(car2car));
    audio.stop_world_effect_audio();
    if let Some(wall) = streaming.iter().find(|s| !s.has("TWO_CAR") && !s.has("EVENT")) {
        audio.play_impact_sweetener(wall, 1.0, point, view);
        assert_eq!(audio.world_effects.sweetener.playing(), None, "{:?} has no admitting description", wall.name);
    }
}
