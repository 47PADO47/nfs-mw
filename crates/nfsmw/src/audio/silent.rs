//! A silent output: the whole mixer runs at real time and its samples are discarded. No sound device is opened, so
//! hidden runs can record the audio trace (`NFSMW_AUDIO_OUTPUT=silent` or `--silent-audio`, `docs/testing.md`).
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use kira::backend::{Backend, Renderer};

const RATE: u32 = 48_000;

static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Asks for the silent output for the audio created after this call (the `--silent-audio` switch).
pub fn request_silent_output() {
    REQUESTED.store(true, Ordering::Relaxed);
}

/// Whether the run asked for the silent output, by the switch or by `NFSMW_AUDIO_OUTPUT=silent`.
pub(super) fn wanted() -> bool {
    REQUESTED.load(Ordering::Relaxed)
        || std::env::var("NFSMW_AUDIO_OUTPUT").is_ok_and(|value| value.eq_ignore_ascii_case("silent"))
}

/// Consumes the renderer on a worker thread at the real sample rate, so sounds start, run and finish on time.
pub(super) struct SilentBackend {
    block: usize,
    stop: Arc<AtomicBool>,
    rendered: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
}

impl SilentBackend {
    /// Frames the mixer has rendered (and thrown away) so far.
    #[cfg(test)]
    pub(super) fn rendered_frames(&self) -> u64 {
        self.rendered.load(Ordering::Relaxed)
    }
}

impl Backend for SilentBackend {
    type Settings = ();
    type Error = String;

    fn setup(_settings: (), internal_buffer_size: usize) -> Result<(Self, u32), String> {
        let backend = Self {
            block: internal_buffer_size.max(1),
            stop: Arc::new(AtomicBool::new(false)),
            rendered: Arc::new(AtomicU64::new(0)),
            worker: None,
        };
        Ok((backend, RATE))
    }

    fn start(&mut self, mut renderer: Renderer) -> Result<(), String> {
        let (block, stop, rendered) = (self.block, self.stop.clone(), self.rendered.clone());
        let worker = std::thread::Builder::new()
            .name("nfsmw-silent-audio".into())
            .spawn(move || {
                let mut samples = vec![0.0f32; block * 2];
                let started = Instant::now();
                let mut done: u64 = 0;
                while !stop.load(Ordering::Relaxed) {
                    let due = (started.elapsed().as_secs_f64() * f64::from(RATE)) as u64;
                    while done + block as u64 <= due {
                        renderer.on_start_processing();
                        renderer.process(&mut samples, 2);
                        done += block as u64;
                    }
                    rendered.store(done, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        Ok(())
    }
}

impl Drop for SilentBackend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
    use kira::{AudioManager, AudioManagerSettings, Frame};

    #[test]
    fn silent_output_runs_the_mixer_without_a_device() {
        let mut manager = AudioManager::<SilentBackend>::new(AudioManagerSettings::default()).unwrap();
        let frames: Arc<[Frame]> = vec![Frame::from_mono(0.5); RATE as usize / 20].into();
        let sound =
            StaticSoundData { sample_rate: RATE, frames, settings: StaticSoundSettings::default(), slice: None };
        let handle = manager.play(sound).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while handle.state() != kira::sound::PlaybackState::Stopped && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(handle.state(), kira::sound::PlaybackState::Stopped, "the 50 ms sound ran to its end");
        assert!(manager.backend_mut().rendered_frames() >= u64::from(RATE) / 20);
    }
}
