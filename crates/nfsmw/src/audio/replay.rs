//! Headless playback through the actual Kira mixer, for installed audio regressions.
use super::{Audio, Output, OutputManager, Volumes};
use game_install::GameDir;
use kira::backend::{Backend, Renderer};
use kira::track::TrackBuilder;
use kira::{AudioManager, AudioManagerSettings};

pub(super) struct ReplayBackend {
    renderer: Option<Renderer>,
    pub samples: Vec<f32>,
}

impl Backend for ReplayBackend {
    type Settings = u32;
    type Error = ();
    fn setup(rate: u32, size: usize) -> Result<(Self, u32), ()> {
        Ok((Self { renderer: None, samples: vec![0.0; size * 2] }, rate))
    }
    fn start(&mut self, renderer: Renderer) -> Result<(), ()> {
        self.renderer = Some(renderer);
        Ok(())
    }
}

impl ReplayBackend {
    pub(in crate::audio) fn render(&mut self, frames: usize) -> &[f32] {
        self.samples.resize(frames * 2, 0.0);
        let renderer = self.renderer.as_mut().expect("initialized replay backend");
        renderer.on_start_processing();
        renderer.process(&mut self.samples, 2);
        &self.samples
    }
}

impl Audio {
    pub(crate) fn replay(dir: GameDir, rate: u32) -> Self {
        let mut main = kira::track::MainTrackBuilder::new();
        let wet = main.add_effect(super::wet::Bus::builder());
        let mut manager = AudioManager::<ReplayBackend>::new(AudioManagerSettings {
            backend_settings: rate,
            main_track_builder: main,
            ..Default::default()
        })
        .unwrap();
        let mut music_builder = TrackBuilder::new();
        let music_filter = music_builder.add_effect(super::filter::MusicFilterBuilder);
        let (sfx, music, engine) = (
            manager.add_sub_track(TrackBuilder::new()).unwrap(),
            manager.add_sub_track(music_builder).unwrap(),
            manager.add_sub_track(TrackBuilder::new()).unwrap(),
        );
        Self::with_output(
            dir,
            Volumes::default(),
            Some(Output { manager: OutputManager::Replay(Box::new(manager)), sfx, music, music_filter, engine, wet }),
        )
    }

    pub(crate) fn render_replay(&mut self, frames: usize) -> &[f32] {
        let Some(output) = self.output.as_mut() else { panic!("replay output required") };
        let OutputManager::Replay(manager) = &mut output.manager else { panic!("replay output required") };
        manager.backend_mut().render(frames)
    }
}
