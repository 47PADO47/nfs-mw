//! From a car type to its sound set: `pvehicle` -> `engineaudio`, `shiftpattern`, `turbosfx`, `acceltrans`.
//! Rules: `docs/specs/engine-sound.md` §2.

use anyhow::{Context, Result};
use blackbox_attrib::{CollectionRef, Database, Value};

use super::fields::{Fields, upgrade_spec};
use super::{
    AccelTransition, CollisionSounds, ENGINE_DIR, EngineSound, GlobalBanks, SHIFTING_DIR, ShiftSound, TURBO_DIR,
    TurboSound,
};

/// The player's installed upgrade levels (`engine_current`, ... in the original: save-game values).
/// 0 is stock; the cap of each class is its `*_upgrades` count in `pvehicle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoundUpgrades {
    pub engine: i32,
    pub transmission: i32,
    pub induction: i32,
}

/// Everything a car's engine sound needs from the database.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSound {
    /// The `pvehicle` collection (`bmwm3gtr`).
    pub car: String,
    pub engine: EngineSound,
    pub shift: ShiftSound,
    /// `None` for cars without a turbo or supercharger sound.
    pub turbo: Option<TurboSound>,
    pub accel_transition: AccelTransition,
    /// Which entry of `pvehicle.engineaudio` was chosen (see [`audio_engine_level`]).
    pub engine_level: usize,
    pub banks: GlobalBanks,
    /// The collision and scrape sounds the car's `pvehicle` links.
    pub collision: CollisionSounds,
}

impl CarSound {
    /// Install-relative paths of every file the car's engine sound loads, `.gin` loops first.
    pub fn files(&self) -> Vec<String> {
        let engine = &self.engine;
        let loops = [&engine.accel_loop, &engine.decel_loop];
        let engine_files = loops
            .into_iter()
            .chain([&engine.bank_main])
            .chain(&engine.banks_aux)
            .chain(&engine.sweet_banks)
            .filter(|name| !name.is_empty())
            .map(|name| format!("{ENGINE_DIR}/{name}"));
        let shift = (!self.shift.bank.is_empty()).then(|| format!("{SHIFTING_DIR}/{}", self.shift.bank));
        let turbo = self.turbo.as_ref().map(|t| format!("{TURBO_DIR}/{}", t.bank));
        let mut files: Vec<String> = engine_files.collect();
        files.extend(shift);
        files.extend(turbo);
        files.extend(self.banks.files());
        files
    }
}

/// The sound set of car type `type_name` (`BMWM3GTR`, any case) at the given upgrade levels, from `db`
/// (`attributes.bin`).
pub fn car_sound(db: &Database, type_name: &str, upgrades: SoundUpgrades) -> Result<CarSound> {
    let name = type_name.to_ascii_lowercase();
    let car = db.collection("pvehicle", &name).with_context(|| format!("no vehicle record named {name}"))?;
    let pvehicle = Fields(car);

    let engine_sets = linked(db, &car, "engineaudio");
    if engine_sets.is_empty() {
        anyhow::bail!("{name}: no engineaudio link");
    }
    let engine_level = audio_engine_level(pvehicle.i32("engine_upgrades"), upgrades.engine, engine_sets.len());
    let engine = EngineSound::read(Fields(engine_sets[engine_level]));
    let accel_transition = car_accel(&engine_sets[engine_level]);

    let shift_sets = upgrade_sets(db, &car, "ShiftSND");
    let shift_at = upgrade_entry(&levels(&shift_sets), pvehicle.i32("transmission_upgrades"), upgrades.transmission);
    let shift = shift_sets
        .get(shift_at)
        .map(|(c, _)| ShiftSound::read(Fields(*c)))
        .with_context(|| format!("{name}: no ShiftSND"))?;

    let turbo_sets = upgrade_sets(db, &car, "TurboSND");
    let turbo_at = upgrade_entry(&levels(&turbo_sets), pvehicle.i32("induction_upgrades"), upgrades.induction);
    let turbo = turbo_sets.get(turbo_at).and_then(|(c, _)| TurboSound::read(Fields(*c)));

    Ok(CarSound {
        car: name,
        engine,
        shift,
        turbo,
        accel_transition,
        engine_level,
        banks: GlobalBanks::read(db),
        collision: CollisionSounds::read(db, type_name),
    })
}

/// Which entry of a car's `engineaudio` array plays, from the engine upgrades the car has (`num_upgrades`,
/// 0 to 4) and the level installed (`current`, 0 to 4): the original's `UpgradeIntervals`.
pub fn audio_engine_level(num_upgrades: i32, current: i32, entries: usize) -> usize {
    let base = (4 - num_upgrades).clamp(0, 4);
    let offset = current.clamp(0, 4);
    let level = match base {
        0 => match base + offset {
            3.. => 2,
            1..=2 => 1,
            _ => 0,
        },
        1 | 2 => i32::from(base + offset >= 3),
        _ => 0,
    };
    (level as usize).min(entries.saturating_sub(1))
}

/// Which entry of a `ShiftSND` or `TurboSND` array applies: the last whose level is not above
/// `4 - num_upgrades + current`, or 0.
pub fn upgrade_entry(entry_levels: &[u8], num_upgrades: i32, current: i32) -> usize {
    let installed = 4 - num_upgrades + current;
    let mut chosen = 0;
    for (n, &level) in entry_levels.iter().enumerate() {
        if i32::from(level) > installed {
            break;
        }
        chosen = n;
    }
    chosen
}

fn levels(sets: &[(CollectionRef<'_>, u8)]) -> Vec<u8> {
    sets.iter().map(|&(_, level)| level).collect()
}

/// The collections an array of RefSpecs links to.
fn linked<'a>(db: &'a Database, car: &CollectionRef<'a>, field: &str) -> Vec<CollectionRef<'a>> {
    let Some(Value::Array(items)) = car.get(field) else { return Vec::new() };
    items.iter().filter_map(Value::as_ref_spec).filter_map(|r| db.resolve(r)).collect()
}

/// The collections and levels of an `UpgradeSpecs` array.
fn upgrade_sets<'a>(db: &'a Database, car: &CollectionRef<'a>, field: &str) -> Vec<(CollectionRef<'a>, u8)> {
    Fields(*car)
        .raw_items(field)
        .into_iter()
        .filter_map(upgrade_spec)
        .filter_map(|(r, level)| db.resolve(r).map(|c| (c, level)))
        .collect()
}

fn car_accel(engine: &CollectionRef<'_>) -> AccelTransition {
    engine.follow("acceltrans").map(|c| AccelTransition::read(Fields(c))).unwrap_or_default()
}
