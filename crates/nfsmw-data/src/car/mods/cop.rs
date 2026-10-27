//! The police side of a mod car (docs/modding/cop-cars.md): the waves it drives in, what kind of unit it is, and the
//! numbers the game gives a police car that has no record of its own.
//!
//! A mod cop car is a car of its own. The original keeps each cop car's damage, rep value and radio class in the records of
//! its `pvehicle`; a mod car has none, so every one of those is given here, by the car's unit class, and never taken from
//! a stock cop.

use super::{ModCar, published};
use crate::pursuit::{CopRecord, HeatRow};

/// How many cars of a mod's kind a wave entry wants when the manifest does not say.
pub const COP_COUNT: u32 = 2;
/// The spawn weight of a mod's cop car in a wave entry, in percent, when the manifest does not say.
pub const COP_CHANCE: u32 = 10;

/// What kind of police unit a mod car is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopKind {
    /// An ordinary patrol or pursuit car.
    #[default]
    Patrol,
    /// A fast pursuit car of the high heat levels.
    Pursuit,
    /// A heavy unit: a sport utility or a van.
    Heavy,
}

/// The damage numbers of a cop unit class, the generic ones the game gives a cop car that has no damage record
/// (`wreck::WreckTuning`): hit points, the threshold under which a hit does nothing, the camera distance beyond which a hit
/// is ignored, and the hit-point scale of the front and rear zones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopDamage {
    pub hit_points: f32,
    pub threshold: f32,
    pub suppress_distance: f32,
    pub front_rear_scale: f32,
}

impl CopKind {
    /// The name the manifest and the devtools use: `patrol`, `pursuit` or `heavy`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Patrol => "patrol",
            Self::Pursuit => "pursuit",
            Self::Heavy => "heavy",
        }
    }

    /// The class's damage numbers. A patrol car takes about five solid hits, a pursuit car eight and a heavy unit ten (the
    /// original's cop cars range from 5 to 10 hit points; the numbers are the class's, not those of any one car).
    pub fn damage(self) -> CopDamage {
        let (hit_points, threshold) = match self {
            Self::Patrol => (5.0, 0.75),
            Self::Pursuit => (8.0, 1.5),
            Self::Heavy => (10.0, 2.0),
        };
        CopDamage { hit_points, threshold, suppress_distance: 35.0, front_rear_scale: 0.5 }
    }

    /// The rep points for wrecking a unit of the class, at every heat level (`RepPointsForDestroying`): the original's
    /// values for its patrol, pursuit and sport-utility cars, one per class.
    pub fn destroy_points(self) -> u32 {
        match self {
            Self::Patrol => 250,
            Self::Pursuit => 5000,
            Self::Heavy => 25000,
        }
    }
}

/// One wave entry of a cop car: at heat level `heat` the wave wants `count` cars of the kind, picked with weight `chance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopWave {
    pub heat: u32,
    pub count: u32,
    pub chance: u32,
}

/// What makes a mod car a police car.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopUnit {
    pub kind: CopKind,
    /// The wave entries, one per heat level, in level order. Never empty for a registered car.
    pub waves: Vec<CopWave>,
}

impl CopUnit {
    /// The entry of heat level `heat`, if the car is in that level's wave.
    pub fn wave(&self, heat: u32) -> Option<&CopWave> {
        self.waves.iter().find(|wave| wave.heat == heat)
    }

    /// A line for the devtools: `heavy, heat 6 x2 @80%, 7 x6 @90%`.
    pub fn describe(&self) -> String {
        let waves: Vec<String> = self.waves.iter().map(|w| format!("{} x{} @{}%", w.heat, w.count, w.chance)).collect();
        format!("{}, heat {}", self.kind.name(), waves.join(", "))
    }
}

/// The police unit of the mod car in `folder`. `None` for a stock car or a mod car that is not a cop.
pub fn cop_unit(folder: &str) -> Option<&'static CopUnit> {
    super::find_in(published(), folder).and_then(|car| car.cop.as_ref())
}

/// Whether the car in `folder` is a mod car that drives as a cop.
pub fn is_cop_car(folder: &str) -> bool {
    cop_unit(folder).is_some()
}

/// The unit class of the mod cop car in `folder`.
pub fn cop_kind(folder: &str) -> Option<CopKind> {
    cop_unit(folder).map(|unit| unit.kind)
}

/// The mod cars of this run that drive as cops, in registration order.
pub fn cop_cars() -> Vec<&'static ModCar> {
    published().iter().filter(|car| car.cop.is_some()).collect()
}

/// Add a cop row for each mod car whose waves have an entry for `heat` to `row`, the wave of that heat level, with the
/// entry's count and chance. The row names the car by its folder in lower case, which is how the cop tables and the AI name
/// cars.
pub fn add_cop_rows(cars: &[ModCar], heat: u32, row: &mut HeatRow) {
    for car in cars {
        let Some(wave) = car.cop.as_ref().and_then(|unit| unit.wave(heat)) else { continue };
        row.cops.push(CopRecord { name: car.folder.to_ascii_lowercase(), count: wave.count, chance: wave.chance });
    }
}
