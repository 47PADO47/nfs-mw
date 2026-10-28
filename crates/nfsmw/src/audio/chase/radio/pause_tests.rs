use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use bevy_ecs::system::RunSystemOnce;
use game_install::GameDir;

use super::*;
use crate::app::Host;
use crate::audio::{Audio, CopFeed, MusicInput, Volumes};
use crate::settings::{Partial, Settings};
use crate::viewer::Scene;

struct PausedWorld {
    paused: Arc<AtomicBool>,
    roam_reads: Arc<AtomicUsize>,
    calls: Arc<Mutex<Vec<f32>>>,
    feed: PursuitFeed,
}
impl Scene for PausedWorld {
    fn title(&self) -> String {
        "synthetic pursuit".into()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
    fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    fn pursuit_feed(&self) -> Option<PursuitFeed> {
        Some(self.feed)
    }
    fn roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        self.roam_reads.fetch_add(1, Ordering::Relaxed);
        None
    }
    fn start_911_call(&mut self, secs: f32) {
        self.calls.lock().unwrap().push(secs);
    }
}

fn audio(dir: GameDir) -> Audio {
    let mut audio = Audio::with_output(dir, Volumes::default(), None);
    audio.set_speech_volume(0.0);
    audio.set_radio_wanted(false);
    audio.chase_radio = ChaseRadio::new(11);
    audio
}

#[test]
#[ignore = "requires installed tuning tables; set NFSMW_GAME_DIR; no sound device or speech recordings used"]
fn batch12_pause_freezes_police_notices_and_911_delivery_but_keeps_music_context() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("installation")).unwrap();
    let mut feed = PursuitFeed { heat: 3, intensity: 1.0, secs: 10.0, player_mph: 90.0, ..Default::default() };
    feed.speech.push_cop(CopFeed { id: 4, active: true, los: true, ..Default::default() });
    let paused = Arc::new(AtomicBool::new(false));
    let reads = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let scene = PausedWorld { paused: paused.clone(), roam_reads: reads.clone(), calls: calls.clone(), feed };
    let host = Host::new(Box::new(scene), &Settings::from(Partial::default()), None);
    let mut world = bevy_ecs::world::World::new();
    let mut time = bevy_time::Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(100));
    world.insert_resource(time);
    world.insert_resource(MusicInput::default());
    world.insert_non_send(host);
    world.insert_non_send(audio(dir.clone()));
    world.run_system_once(crate::audio::plugin::drive_car).unwrap();
    let before = world.non_send::<Audio>().chase_flow_status();
    let music = world.resource::<MusicInput>().state();
    assert!(music.pursuit.is_some());
    world.non_send_mut::<Audio>().chase_radio.call_911 = Some(12.0);
    paused.store(true, Ordering::Relaxed);
    for _ in 0..300 {
        world.run_system_once(crate::audio::plugin::drive_car).unwrap();
    }
    assert_eq!(world.non_send::<Audio>().chase_flow_status(), before.replace("911 call none", "911 call 12 s"));
    assert_eq!(world.resource::<MusicInput>().state(), music);
    assert_eq!(reads.load(Ordering::Relaxed), 1, "pause must not drain the world's roam events");
    assert!(calls.lock().unwrap().is_empty(), "a pending 911 call must wait for simulation resume");
    paused.store(false, Ordering::Relaxed);
    world.run_system_once(crate::audio::plugin::drive_car).unwrap();
    assert_eq!(*calls.lock().unwrap(), [12.0]);
    let mut control = audio(dir);
    let mut control_music = MusicInput::default();
    for _ in 0..2 {
        control.follow_chase(&mut control_music, Some(&feed), EndReason::None, 0.1);
    }
    assert_eq!(
        world.non_send::<Audio>().chase_flow_status(),
        control.chase_flow_status(),
        "resume advances one simulation step"
    );
    assert_eq!(world.resource::<MusicInput>().state(), control_music.state());
    let stable = control.chase_flow_status();
    let mut noisy = feed;
    noisy.speech.events.push(crate::audio::WorldEvent::TireBlown);
    for _ in 0..10 {
        control.follow_chase(&mut control_music, Some(&noisy), EndReason::None, 0.0);
    }
    assert_eq!(control.chase_flow_status(), stable, "zero-dt updates must not consume new world notices");
}
