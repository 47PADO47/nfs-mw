use super::*;
use glam::Vec3;
use nfsmw_data::sound::{CollisionZone, EffectSound, EventKind};

#[test]
#[ignore = "needs installed audio banks and mix map (set NFSMW_GAME_DIR)"]
fn installed_impact_retains_first_mix_azimuth_while_scrape_follows_the_view() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let db = audio.database().unwrap();
    let definition = EffectSound::read(&db, blackbox_attrib::vlt_hash("sfx_tollbooth_brk"), CollisionZone::Default)
        .expect("installed sequenced tollbooth event");
    let stitches = audio.stitches();
    let mut stitch = stitches[definition.sound.levels[0][0] as usize].clone();
    for piece in &mut stitch.pieces {
        piece.offset = 25_000;
    }
    let event = CarEvent {
        sound_profile: 0,
        vehicle: None,
        audio_zone: None,
        kind: EventKind::Detached,
        surface: 0,
        magnitude: 1.0,
        front: false,
        smackable: true,
        position: Vec3::new(5.0, 12.0, 0.0),
    };
    let map = crate::audio::mixer::mix_map(&audio.dir).unwrap();
    let mut playback = ImpactPlayback::default();
    playback.collision_with_mixer(
        &mut audio,
        &stitch,
        &definition.sound,
        event,
        Some(CollisionMixer::new(map.clone())),
        32767,
    );
    let input = CarInput::default();
    let first_view = AudioView::default();
    playback.update(&mut audio, 1.0 / 60.0, 0.0, &input, &first_view, (1.0, 0));
    let Source::Collision { initial_azimuth: Some(first), .. } = &playback.active[0].source else {
        panic!("production impact update must capture its initial azimuth");
    };
    let first = *first;
    let turned = AudioView { camera_forward: Vec3::NEG_X, car_forward: Vec3::NEG_X, ..first_view };
    playback.update(&mut audio, 1.0 / 60.0, 0.0, &input, &turned, (1.0, 0));
    let Source::Collision { initial_azimuth, .. } = &playback.active[0].source else { unreachable!() };
    assert_eq!(*initial_azimuth, Some(first));

    let mut scrape = CollisionMixer::new(map);
    let before = scrape
        .update(
            1.0 / 60.0,
            CollisionFrame {
                sound: &definition.sound,
                magnitude: 1.0,
                point: event.position,
                view: &first_view,
                input: &input,
                scrape: true,
            },
        )
        .azimuth;
    let after = scrape
        .update(
            1.0 / 60.0,
            CollisionFrame {
                sound: &definition.sound,
                magnitude: 1.0,
                point: event.position,
                view: &turned,
                input: &input,
                scrape: true,
            },
        )
        .azimuth;
    assert_ne!(before, after, "scrape azimuth remains live after the camera turns");
}
