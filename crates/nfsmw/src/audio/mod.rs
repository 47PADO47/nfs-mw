//! Sound output: `kira` plays what the decoders (`ea-audio`) and the synthesiser (`blackbox-ginsu`) produce.
//!
//! [`Audio`] owns the output device, three mixer groups (effects, music, engine) under a master volume and a
//! cache of the sounds read from the install's banks. Without a usable device it still exists and every
//! call fails with a message, so the game runs silent instead of stopping.

mod aems;
mod car;
pub mod commands;
mod engine;
mod fx;
mod mixer;
mod pcm;
mod plugin;
mod radio;
mod refs;
mod speech;
mod tuning;
mod volume;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use blackbox_attrib::Database;
use ea_audio::abk::Bank;
use game_install::GameDir;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Tween};
use nfsmw_data::sound::{CarSound, EngineLoops, SoundUpgrades};

pub use car::{CarEvent, CarSoundState};
pub use engine::{EngineHandle, EngineMix, EngineVoice, LoopMix};
pub use plugin::AudioPlugin;
#[allow(unused_imports)] // for the HUD, which does not draw the song yet
pub use radio::NowPlaying;
pub use volume::{Group, Volumes};

struct Output {
    manager: AudioManager<DefaultBackend>,
    sfx: TrackHandle,
    music: TrackHandle,
    engine: TrackHandle,
}

pub struct Audio {
    output: Option<Output>,
    dir: GameDir,
    volumes: Volumes,
    /// The bytes of the banks read so far, by path under `SOUND/`.
    banks: HashMap<String, Arc<Vec<u8>>>,
    /// Decoded bank sounds by (bank path, index).
    sounds: HashMap<(String, usize), StaticSoundData>,
    /// The gameplay database, read when a car's sound is first needed.
    database: Option<Arc<Database>>,
    /// The engine the `engine` console command plays, with the car it belongs to.
    pub test_engine: Option<(EngineHandle, CarSound)>,
    /// The engine of the car being driven.
    car: Option<car::CarAudio>,
    /// The collision stitches of `InGameB.bun`, once read.
    stitches: Option<Arc<Vec<nfsmw_data::sound::Stitch>>>,
    /// The radio, loaded when first needed.
    radio: radio::RadioSlot,
    /// Whether the settings let the radio play (the `radio` console command can still start it).
    radio_wanted: bool,
    /// The police dispatch's speech, loaded when first needed.
    speech: speech::SpeechSlot,
    /// Bank sounds that could not be loaded, so each is reported once.
    missing: HashSet<(String, usize)>,
    /// The car whose sound could not be loaded, so the failure is not repeated every frame.
    failed: Option<String>,
    /// The scene's soundtrack that is playing, kept so it can be stopped when the scene goes.
    music: Option<StaticSoundHandle>,
}

/// A car's engine voice and the data that maps its RPM to the loops' frequency.
pub struct CarEngine {
    pub voice: EngineVoice,
    pub sound: CarSound,
}

impl CarEngine {
    /// The lowest frequency of the accelerate loop, in the loop's units.
    pub fn accel_min_frequency(&self) -> f32 {
        self.voice.accel.tables().min_frequency()
    }
}

impl Audio {
    /// Open the default output device. When there is none, or it fails, the returned value is silent and says
    /// why in the log.
    pub fn new(dir: GameDir, volumes: Volumes) -> Self {
        let output = match Output::open() {
            Ok(output) => Some(output),
            Err(e) => {
                log::warn!("no sound: {e}");
                None
            }
        };
        let mut audio = Self {
            output,
            dir,
            volumes,
            banks: HashMap::new(),
            sounds: HashMap::new(),
            database: None,
            test_engine: None,
            car: None,
            stitches: None,
            radio: radio::RadioSlot::default(),
            radio_wanted: true,
            speech: speech::SpeechSlot::default(),
            missing: HashSet::new(),
            failed: None,
            music: None,
        };
        audio.set_volumes(volumes);
        audio
    }

    /// Whether sound comes out.
    pub fn available(&self) -> bool {
        self.output.is_some()
    }

    pub fn volumes(&self) -> Volumes {
        self.volumes
    }

    pub fn set_volumes(&mut self, volumes: Volumes) {
        self.volumes = volumes;
        let Some(out) = self.output.as_mut() else { return };
        let tween = Tween::default();
        out.manager.main_track().set_volume(volume::decibels(volumes.master), tween);
        out.sfx.set_volume(volume::decibels(volumes.sfx), tween);
        out.music.set_volume(volume::decibels(volumes.music), tween);
        out.engine.set_volume(volume::decibels(volumes.engine), tween);
    }

    fn output(&mut self) -> Result<&mut Output, String> {
        self.output.as_mut().ok_or_else(|| "no sound device".to_owned())
    }

    /// The bytes of a bank (`IG_GLOBAL/Stich_Collision_MB.abk`, relative to `SOUND/`), read once.
    fn bank_bytes(&mut self, bank: &str) -> Result<Arc<Vec<u8>>, String> {
        if let Some(bytes) = self.banks.get(bank) {
            return Ok(bytes.clone());
        }
        let bytes = self.dir.read(&format!("SOUND/{bank}")).map_err(|e| format!("{bank}: {e:#}"))?;
        let bytes = Arc::new(bytes);
        self.banks.insert(bank.to_owned(), bytes.clone());
        Ok(bytes)
    }

    /// How many sounds a bank holds.
    pub fn bank_len(&mut self, bank: &str) -> Result<usize, String> {
        let bytes = self.bank_bytes(bank)?;
        let parsed = Bank::parse(&bytes).map_err(|e| format!("{bank}: {e}"))?;
        Ok(parsed.sounds().len())
    }

    /// Sound number `index` of `bank`, decoded (and kept for next time).
    pub fn bank_sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        let key = (bank.to_owned(), index);
        if let Some(data) = self.sounds.get(&key) {
            return Ok(data.clone());
        }
        let bytes = self.bank_bytes(bank)?;
        let parsed = Bank::parse(&bytes).map_err(|e| format!("{bank}: {e}"))?;
        let decoded = parsed.decode(index).map_err(|e| format!("{bank} #{index}: {e}"))?;
        let data = pcm::sound(&decoded);
        self.sounds.insert(key, data.clone());
        Ok(data)
    }

    /// Play a sound in a group.
    pub fn play(&mut self, group: Group, data: StaticSoundData) -> Result<StaticSoundHandle, String> {
        let out = self.output()?;
        let track = match group {
            Group::Sfx => &mut out.sfx,
            Group::Music => &mut out.music,
            Group::Engine => &mut out.engine,
        };
        track.play(data).map_err(|e| e.to_string())
    }

    /// Plays a scene's soundtrack in the music group, in place of the one playing.
    pub fn play_music(&mut self, data: StaticSoundData) -> Result<(), String> {
        self.stop_music();
        let handle = self.play(Group::Music, data)?;
        self.music = Some(handle);
        Ok(())
    }

    /// Stops the soundtrack, if one is playing.
    pub fn stop_music(&mut self) {
        let Some(mut handle) = self.music.take() else { return };
        handle.stop(Tween::default());
    }

    fn database(&mut self) -> Result<Arc<Database>, String> {
        if let Some(db) = &self.database {
            return Ok(db.clone());
        }
        let bytes = self.dir.read("GLOBAL/ATTRIBUTES.BIN").map_err(|e| format!("ATTRIBUTES.BIN: {e:#}"))?;
        let db = Arc::new(Database::open(&bytes).map_err(|e| format!("ATTRIBUTES.BIN: {e}"))?);
        self.database = Some(db.clone());
        Ok(db)
    }

    /// The engine sound of car type `car` (`BMWM3GTR`) at stock levels: its Ginsu loops, decoded.
    pub fn load_car_engine(&mut self, car: &str) -> Result<CarEngine, String> {
        let db = self.database()?;
        let set =
            nfsmw_data::sound::car_sound(&db, car, SoundUpgrades::default()).map_err(|e| format!("{car}: {e:#}"))?;
        let loops = EngineLoops::load(&self.dir, &set.engine).map_err(|e| format!("{car}: {e:#}"))?;
        let accel = loops.accel.ok_or_else(|| format!("{car} has no Ginsu engine loop"))?;
        let voice = EngineVoice { accel, decel: loops.decel, start: EngineMix::default() };
        Ok(CarEngine { voice, sound: set })
    }

    /// Start an engine voice; drop the handle to stop it.
    pub fn start_engine(&mut self, voice: EngineVoice) -> Result<EngineHandle, String> {
        self.output()?.engine.play(voice).map_err(|e| e.to_string())
    }
}

impl Output {
    fn open() -> Result<Self, String> {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).map_err(|e| e.to_string())?;
        let mut track = || manager.add_sub_track(TrackBuilder::new()).map_err(|e| e.to_string());
        let (sfx, music, engine) = (track()?, track()?, track()?);
        Ok(Self { manager, sfx, music, engine })
    }
}
