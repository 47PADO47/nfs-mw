//! The maker of the lazy files: converts a car from its 2012 bundle when the game first reads one of its files.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Instant;

use blackbox_modding::sdk::{Error, LazyFiles, ModResult};

use super::{CarRecipe, Key, Part, Recipe, Sound, parse_key};
use crate::config::Config;
use crate::prepare::{self, Settings};
use crate::sounds::{self, CarSounds};
use crate::source::{CarFile, Source};

/// How many cars' results are kept: the files of a car are read one after the other, and a car is read again by the garage and
/// by a race, so the last few are enough.
const KEEP: usize = 3;

/// The files of a car that the maker has made.
struct Files {
    geometry: Vec<u8>,
    textures: Vec<u8>,
}

/// The results the maker keeps, most recent first.
#[derive(Default)]
struct Cache {
    files: Vec<(String, Arc<Files>)>,
    sounds: Vec<(String, Arc<CarSounds>)>,
}

/// Makes the lazy files of the cars of one recipe. The host calls it from any thread, so one lock covers the cache and the
/// work: two threads that want the same car wait for one conversion, and two that want different cars take turns.
pub struct Maker {
    recipe: Recipe,
    /// The 2012 install, opened by the first read.
    source: OnceLock<Result<Source, String>>,
    cache: Mutex<Cache>,
    /// How long each car's files took, in milliseconds, as `(key, milliseconds)`, for the tests and the measuring commands.
    timings: Mutex<Vec<(String, f32)>>,
}

impl Maker {
    pub fn new(recipe: Recipe) -> Self {
        Self { recipe, source: OnceLock::new(), cache: Mutex::new(Cache::default()), timings: Mutex::new(Vec::new()) }
    }

    /// The recipe the files are made from.
    pub fn recipe(&self) -> &Recipe {
        &self.recipe
    }

    /// How long each conversion took so far, as `(what, milliseconds)`: the first read of a car's geometry (the textures come
    /// with it), the car's sounds, and the opening of the install.
    pub fn timings(&self) -> Vec<(String, f32)> {
        self.timings.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn time(&self, what: String, started: Instant) {
        let millis = started.elapsed().as_secs_f32() * 1000.0;
        log::info!("nfsmw-2012-content: {what} made in {millis:.0} ms");
        self.timings.lock().unwrap_or_else(PoisonError::into_inner).push((what, millis));
    }

    /// The 2012 install, opened on first use.
    fn source(&self) -> ModResult<&Source> {
        let opened = self.source.get_or_init(|| {
            let started = Instant::now();
            let source = Source::open(&self.recipe.source).map_err(|error| format!("{error:#}"));
            self.time("opening the 2012 install".to_owned(), started);
            source
        });
        opened.as_ref().map_err(|reason| {
            log::warn!("nfsmw-2012-content: {reason}");
            Error::NotFound
        })
    }

    /// The bundle file of number `number`.
    fn bundle(source: &Source, number: u32) -> Option<&CarFile> {
        source.cars().iter().find(|file| file.number == number)
    }

    /// The geometry and textures of `car`, made now or found among the last few.
    fn files(&self, cache: &mut Cache, car: &CarRecipe) -> ModResult<Arc<Files>> {
        if let Some((_, hit)) = cache.files.iter().find(|(id, _)| *id == car.id) {
            return Ok(Arc::clone(hit));
        }
        let source = self.source()?;
        let started = Instant::now();
        let file = Self::bundle(source, car.file).ok_or(Error::NotFound)?;
        let liveries: Vec<&CarFile> = car.liveries.iter().filter_map(|number| Self::bundle(source, *number)).collect();
        let settings =
            Settings { max_texture_size: self.recipe.max_texture_size, driver_poses: self.recipe.driver_poses };
        let prepared = prepare::prepare(source, file, &car.id, &liveries, &settings).map_err(|error| {
            log::warn!("nfsmw-2012-content: {} cannot be converted again: {error:#}", car.id);
            Error::Host
        })?;
        let made = Arc::new(Files { geometry: prepared.built.geometry, textures: prepared.built.textures });
        remember(&mut cache.files, &car.id, &made);
        self.time(format!("{}: GEOMETRY.BIN and TEXTURES.BIN", car.id), started);
        Ok(made)
    }

    /// The recordings of `car`, made now or found among the last few.
    fn sounds(&self, cache: &mut Cache, car: &CarRecipe) -> ModResult<Arc<CarSounds>> {
        if let Some((_, hit)) = cache.sounds.iter().find(|(id, _)| *id == car.id) {
            return Ok(Arc::clone(hit));
        }
        let source = self.source()?;
        let started = Instant::now();
        let file = Self::bundle(source, car.file).ok_or(Error::NotFound)?;
        let config = Config { own_engine_sound: true, ..Config::default() };
        let made = Arc::new(sounds::of(source, file, &car.id, &config, Some((car.idle_rpm, car.redline_rpm))));
        remember(&mut cache.sounds, &car.id, &made);
        self.time(format!("{}: engine and effect sounds", car.id), started);
        Ok(made)
    }
}

/// Put `made` first among `kept`, which holds at most [`KEEP`] results.
fn remember<T>(kept: &mut Vec<(String, Arc<T>)>, id: &str, made: &Arc<T>) {
    kept.insert(0, (id.to_owned(), Arc::clone(made)));
    kept.truncate(KEEP);
}

/// The `at`th recording of `kind` in `made`.
fn recording(made: &CarSounds, kind: Sound, at: usize) -> Option<&Vec<u8>> {
    let effects = made.effects.as_ref();
    match kind {
        Sound::Engine => made.engine.as_ref()?.layers.get(at).map(|layer| &layer.wav),
        Sound::Shift => effects?.shift.get(at),
        Sound::Whine => effects?.turbo_whine.get(at),
        Sound::Blowoff => effects?.turbo_blowoff.get(at),
        Sound::Backfire => effects?.backfire.get(at),
        Sound::Pops => effects?.pops.get(at),
    }
}

impl LazyFiles for Maker {
    fn read(&self, key: &str) -> ModResult<Vec<u8>> {
        let key = parse_key(key).ok_or(Error::Arg)?;
        let id = match &key {
            Key::Car(id, _) | Key::Sound(id, _, _) => id,
        };
        let car = self.recipe.car(id).ok_or(Error::NotFound)?;
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        match key {
            Key::Car(_, Part::Geometry) => Ok(self.files(&mut cache, car)?.geometry.clone()),
            Key::Car(_, Part::Textures) => Ok(self.files(&mut cache, car)?.textures.clone()),
            Key::Sound(_, kind, at) => {
                let made = self.sounds(&mut cache, car)?;
                recording(&made, kind, at).cloned().ok_or(Error::NotFound)
            }
        }
    }
}
