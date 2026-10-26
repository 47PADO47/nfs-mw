//! Drives a car through the real headless mixer (`Audio::replay`) from scripted inputs and returns what it renders.
//! Nothing is played: the replay backend only fills a buffer.

use std::path::Path;

use blackbox_carsound::{CarInput, GEAR_FIRST, ScrapeKind};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::sound::EventKind;

use super::super::{
    CarAudioTelemetry, CarContact, CarEvent, CarSoundState, CarSoundTick, ContactSource, SoundUpgrades,
};
use crate::audio::{Audio, AudioView, Volumes};
use crate::modding::content::{ModDir, rescan_in};

pub const RATE: u32 = 48_000;
/// The scene's tick: the sound is driven 60 times a second.
pub const FPS: u32 = 60;
/// Top gear id given to every car (the scene's numbering: first is 2).
pub const TOP_GEAR: i32 = 7;
/// Metres per second in one mile per hour.
const MPH_TO_MPS: f32 = 0.447_04;

/// One frame of a scripted drive.
#[derive(Clone)]
pub struct Frame {
    pub input: CarInput,
    pub events: Vec<CarEvent>,
    pub scrape: Option<(ScrapeKind, f32)>,
    /// What the car touches this frame.
    pub contacts: Vec<CarContact>,
}

impl Frame {
    /// A frame at `rpm` (0 idle to 1 redline) with `throttle`, in `gear`, at `mph`.
    pub fn drive(rpm: f32, throttle: f32, gear: i32, mph: f32) -> Self {
        let input = CarInput {
            rpm_pct: rpm,
            throttle,
            gear,
            top_gear: Some(TOP_GEAR),
            speed: mph * MPH_TO_MPS,
            ..CarInput::default()
        };
        Self { input, events: Vec::new(), scrape: None, contacts: Vec::new() }
    }

    pub fn idle() -> Self {
        Self::drive(0.0, 0.0, GEAR_FIRST, 0.0)
    }
}

/// A hit on a wall of `magnitude` (0 to 1) at the front of the car.
pub fn hit(magnitude: f32) -> CarEvent {
    CarEvent {
        sound_profile: 0,
        vehicle: None,
        audio_zone: None,
        kind: EventKind::HitWorld,
        surface: 0,
        magnitude,
        front: true,
        smackable: false,
        position: Vec3::ZERO,
    }
}

/// A wall the car slides along at 12 m/s, front on.
pub fn wall_contact() -> CarContact {
    CarContact {
        sound_profile: 0,
        vehicle: None,
        audio_zone: None,
        detached: false,
        source: ContactSource::World,
        identity: 1,
        surface: nfsmw_data::car::physics::SurfaceTable::hash_of("default"),
        point: Vec3::new(0.0, 0.0, 2.0),
        normal: Vec3::NEG_Z,
        velocity: Vec3::new(12.0, 0.0, 3.0),
        delta_v: 0.1,
        front: true,
        bottom: false,
    }
}

/// Which of the sound groups are heard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Heard {
    /// The engine and the engine-like loops (the car's whole engine, the whine).
    Engine,
    /// Everything else a car makes: shifts, pops, tyres, impacts, scrapes, nitrous.
    Sfx,
    Both,
}

impl Heard {
    fn volumes(self) -> Volumes {
        let (engine, sfx) = match self {
            Self::Engine => (1.0, 0.0),
            Self::Sfx => (0.0, 1.0),
            Self::Both => (1.0, 1.0),
        };
        Volumes { master: 1.0, music: 0.0, sfx, engine, hud: 0.0 }
    }
}

/// A headless audio engine on one install, with the converted 2012 cars registered when a mod folder is given.
pub struct Rig {
    dir: GameDir,
}

impl Rig {
    pub fn open(game: &Path, mod_dir: Option<&Path>) -> Self {
        let dir = GameDir::open(game.to_path_buf()).unwrap();
        if let Some(mod_dir) = mod_dir {
            let id = "nfsmw-2012-content";
            let mods = [ModDir { id: id.into(), dir: mod_dir.to_path_buf(), priority: 0 }];
            rescan_in(&dir, &mods, id).unwrap();
        }
        Self { dir }
    }

    /// Drive `car` through `frames` and return the mono render. A fresh engine is used for each call, so the runs do
    /// not leak into each other.
    pub fn render(&self, car: &str, frames: &[Frame], heard: Heard) -> Vec<f32> {
        let mut audio = Audio::replay(self.dir.clone(), RATE);
        audio.set_volumes(heard.volumes());
        let mut stereo = Vec::with_capacity(frames.len() * (RATE / FPS) as usize * 2);
        let dt = 1.0 / FPS as f32;
        for (index, frame) in frames.iter().enumerate() {
            let from = u64::from(index as u32) * u64::from(RATE) / u64::from(FPS);
            let to = u64::from(index as u32 + 1) * u64::from(RATE) / u64::from(FPS);
            let tick = CarSoundTick {
                telemetry: CarAudioTelemetry { forward_speed: frame.input.speed, ..Default::default() },
                input: frame.input,
                contacts: frame.contacts.clone(),
                surface: 0,
            };
            let state = CarSoundState {
                nitrous_fx: None,
                telemetry: tick.telemetry,
                car: car.into(),
                upgrades: SoundUpgrades::default(),
                input: frame.input,
                events: frame.events.clone(),
                scrape: frame.scrape,
                surface: 0,
                ticks: vec![tick],
                view: AudioView::default(),
            };
            audio.drive_car(Some(&state), dt);
            stereo.extend_from_slice(audio.render_replay((to - from) as usize));
        }
        super::dsp::mono(&stereo)
    }
}
