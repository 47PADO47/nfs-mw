//! Context selection and the real shared Kira music bus, with a synthetic sustained tone.
use super::*;
use crate::settings::{Partial, Percent};

#[test]
fn batch18_music_context_selects_menu_game_pause_and_preview_independently() {
    let mut settings = Settings::from(Partial::default());
    settings.menu_music_volume = Percent(20);
    settings.game_music_volume = Percent(80);
    let menus = volumes_in_context(&settings, false, false);
    let game = volumes_in_context(&settings, true, false);
    let preview = volumes_in_context(&settings, true, true);
    assert_eq!(menus.music, 0.2);
    assert_eq!(game.music, 0.8, "driving and paused gameplay share the game context");
    assert_eq!(preview.music, 0.2, "explicit menu preview uses menu volume");
    assert_eq!(menus.sfx, game.sfx);
    assert_eq!(menus.master, game.master);
    settings.menu_music_volume = Percent(0);
    assert_eq!(volumes_in_context(&settings, false, false).music, 0.0);
    assert_eq!(volumes_in_context(&settings, true, false).music, 0.8);
}

#[test]
#[ignore = "requires installation identity only; no game recordings or audio device used"]
fn batch18_music_live_gain_changes_the_existing_music_voice_without_restarting_it() {
    use crate::audio::{capture, pcm};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut audio = Audio::replay(dir, 1000);
    let mut settings = Settings::from(Partial::default());
    settings.master_volume = Percent(100);
    settings.menu_music_volume = Percent(20);
    settings.game_music_volume = Percent(80);
    let data =
        pcm::sound(&ea_audio::Pcm { sample_rate: 1000, channels: 1, samples: vec![8000; 100_000], loop_range: None });
    let voice = audio.play(crate::audio::Group::Music, data).unwrap();
    let mut heard = Vec::new();
    for (driving, preview) in [(false, false), (true, false), (true, true), (false, false)] {
        audio.set_volumes(volumes_in_context(&settings, driving, preview));
        audio.render_replay(200);
        let samples = audio.render_replay(200);
        heard.push(capture::levels(samples).rms);
    }
    assert!(heard[0] > 0.001, "synthetic voice is audible");
    assert!((heard[1] / heard[0] - 4.0).abs() < 0.05, "context ratio follows independent gains: {heard:?}");
    assert!((heard[2] - heard[0]).abs() < 0.001, "preview selects menu gain");
    assert!((heard[3] - heard[0]).abs() < 0.001, "closing preview restores current context");
    assert!(voice.position() > 1.0, "gain changes preserve this voice's advancing playhead");
}

#[test]
#[ignore = "requires installation identity only; synthetic scene and no sound output"]
fn batch18_music_scheduled_context_retains_game_gain_while_paused_and_updates_other_buses() {
    use crate::viewer::Scene;
    use bevy_ecs::system::RunSystemOnce;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct WorldScene(Arc<AtomicBool>);
    impl Scene for WorldScene {
        fn title(&self) -> String {
            "music context test".into()
        }
        fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
        fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
            unreachable!()
        }
        fn paused(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut settings = Settings::from(Partial::default());
    settings.menu_music_volume = Percent(20);
    settings.game_music_volume = Percent(80);
    let paused = Arc::new(AtomicBool::new(false));
    let host = Host::new(Box::new(WorldScene(paused.clone())), &settings, None);
    let mut audio = Audio::with_output(dir, Volumes::default(), None);
    audio.set_radio_wanted(false);
    let mut world = bevy_ecs::world::World::new();
    world.insert_resource(settings);
    world.insert_resource(bevy_time::Time::<()>::default());
    world.insert_resource(MusicInput::default());
    world.insert_non_send(host);
    world.insert_non_send(audio);
    world.run_system_once(drive_car).unwrap();
    assert_eq!(world.non_send::<Audio>().volumes().music, 0.2);
    paused.store(true, Ordering::Relaxed);
    world.run_system_once(drive_car).unwrap();
    assert_eq!(world.non_send::<Audio>().volumes().music, 0.8, "paused gameplay remains in game music context");
    {
        let mut settings = world.resource_mut::<Settings>();
        settings.master_volume = Percent(50);
        settings.sfx_volume = Percent(30);
        settings.menu_music_volume = Percent(0);
        settings.game_music_volume = Percent(40);
    }
    world.run_system_once(drive_car).unwrap();
    let gain = world.non_send::<Audio>().volumes();
    assert_eq!((gain.master, gain.sfx, gain.music), (0.5, 0.3, 0.4));
    paused.store(false, Ordering::Relaxed);
    world.run_system_once(drive_car).unwrap();
    assert_eq!(world.non_send::<Audio>().volumes().music, 0.0, "returning to menus restores the independent mute");
}
