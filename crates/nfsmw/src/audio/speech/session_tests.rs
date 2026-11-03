//! The scheduled scene cleanup, with synthetic speech and a sink that records active playback.

use std::sync::{Arc, Mutex};

use bevy_ecs::system::RunSystemOnce;
use ea_audio::{Pcm, speech::Params};
use game_install::GameDir;

use super::*;
use crate::app::Host;
use crate::audio::speech::{Lines, Outcome, Sink};
use crate::audio::{MusicInput, Volumes};
use crate::frontend::MenuScene;
use crate::settings::{Partial, Settings};

#[derive(Default)]
struct Playback {
    busy: bool,
    stops: usize,
    plays: usize,
}
struct Output(Arc<Mutex<Playback>>);
impl Sink for Output {
    fn busy(&self) -> bool {
        self.0.lock().unwrap().busy
    }
    fn stop(&mut self) {
        let mut p = self.0.lock().unwrap();
        p.busy = false;
        p.stops += 1;
    }
    fn set_volume(&mut self, _: f32) {}
    fn play(&mut self, _: &Pcm) -> Result<(), String> {
        let mut p = self.0.lock().unwrap();
        p.busy = true;
        p.plays += 1;
        Ok(())
    }
}
struct Recording;
impl Lines for Recording {
    fn has(&self, _: u32) -> bool {
        true
    }
    fn take(&mut self, _: u32, _: u16, _: &Params) -> Result<Pcm, String> {
        self.bank_take(0, 0)
    }
    fn bank_take(&self, _: usize, _: usize) -> Result<Pcm, String> {
        Ok(Pcm { sample_rate: 1000, channels: 1, samples: vec![1; 1000], loop_range: None })
    }
    fn summary(&self) -> String {
        "synthetic".into()
    }
}

#[test]
#[ignore = "requires installation identity only; set NFSMW_GAME_DIR; no audio device or game recordings used"]
fn batch12_scene_cleanup_stops_active_police_and_discards_queued_speech() {
    let dir =
        GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("installation"))).unwrap();
    let playback = Arc::new(Mutex::new(Playback::default()));
    let events = vec![nfsmw_data::speech::SpeechEvent { id: 1, name: "synthetic".into(), ..Default::default() }];
    let mut speech =
        Speech::new(events, Default::default(), Box::new(Recording), Box::new(Output(playback.clone())), Rng::new(3));
    let mut audio = Audio::with_output(dir, Volumes::default(), None);
    speech.play_now(1, 0, &SpeechParams::new()).unwrap();
    assert_eq!(speech.request(1, 0), Outcome::Queued);
    assert_eq!(speech.dispatcher().waiting(), 1);
    audio.speech.state = State::Ready(Box::new(speech));
    let mut world = bevy_ecs::world::World::new();
    let mut host = Host::new(Box::new(MenuScene::default()), &Settings::from(Partial::default()), None);
    host.scene_changes = 1;
    let mut music = MusicInput::default();
    music.start_pursuit(3, 0.8);
    world.insert_non_send(host);
    world.insert_non_send(audio);
    world.insert_resource(music);
    world.insert_resource(bevy_time::Time::<()>::default());
    world.run_system_once(crate::audio::plugin::drive_car).unwrap();
    let mut audio = world.remove_non_send::<Audio>().unwrap();
    assert!(!playback.lock().unwrap().busy, "scene cleanup left the current line sounding");
    assert_eq!(playback.lock().unwrap().stops, 2, "one play_now cut and one scene cleanup stop");
    let speech = audio.speech.loaded_mut().unwrap();
    assert_eq!(speech.dispatcher().waiting(), 0, "the old world's queued line survived");
    for _ in 0..20 {
        audio.update_speech(0.1);
    }
    assert_eq!(playback.lock().unwrap().plays, 1, "an old request restarted speech in the menu");
    assert_eq!(world.resource::<MusicInput>().state(), Default::default());
    let speech = audio.speech.loaded_mut().unwrap();
    assert_eq!(speech.request(1, 0), Outcome::Queued, "the next session is not muted");
}
