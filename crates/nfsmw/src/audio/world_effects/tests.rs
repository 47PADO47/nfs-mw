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
    let request = WorldEffectAudio { effect, zone: CollisionZone::Default, position: Vec3::ZERO, magnitude: 1.0 };
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
