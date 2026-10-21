use super::*;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_tollbooth_detachment_sequence_reaches_the_world_audio_queue_and_pcm() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let placed = fixtures(&dir, &index, &data)
        .into_iter()
        .find(|p| p.spawner.unwrap().attributes == vlt_hash("tollbooth"))
        .unwrap();
    let pose = fixture_pose(&placed, &data);
    let (mut world, _, _) = strike(&placed, &data, pose, 60.0, true);
    let mut requests = Vec::new();
    for _ in 0..12 {
        world.update_scenery(&data);
        requests.extend(world.take_world_effect_audio());
    }
    let toll: Vec<_> = requests.iter().filter(|request| request.effect == vlt_hash("sfx_tollbooth_brk")).collect();
    assert_eq!(toll.len(), 1, "the actual detach sequence emits its authored breaking sound exactly once");
    assert!(world.take_world_effect_audio().is_empty(), "requests drain once");
    let point = toll[0].position;
    let original = placed.transform.w_axis.truncate();
    assert!(point.distance(original) < 15.0, "the sound uses the booth's event marker");
    let view = crate::audio::AudioView { camera_position: point, car_position: point, ..Default::default() };
    let mut audio = crate::audio::Audio::replay(dir, 48000);
    audio.update_world_effect_audio(requests, view, 1.0 / 60.0);
    let mut energy = 0.0_f64;
    for _ in 0..180 {
        audio.update_world_effect_audio(Vec::new(), view, 1.0 / 60.0);
        for sample in audio.render_replay(800) {
            energy += f64::from(*sample).powi(2);
        }
    }
    assert!(energy > 0.001, "real tollbooth contact -> sequence -> world audio -> authored PCM");
}
