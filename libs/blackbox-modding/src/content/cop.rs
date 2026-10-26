//! The police side of a car manifest: the waves a cop car drives in, and what kind of unit it is.
//!
//! A cop car is listed in the wave of each heat level it appears at, with the two numbers every wave entry carries: how
//! many cars of the kind the wave wants (`count`) and the weight with which the chase picks the kind among the ones it
//! still lacks (`chance`, percent). `heat = [..]` is the short form (one entry per level with the default numbers);
//! `[[cop]]` tables give the numbers per level. The unit class says how the game treats the car beyond the wave.

use serde::Deserialize;

use super::ContentError;

/// The cars of the kind a wave entry wants when the manifest does not say.
pub const DEFAULT_COUNT: u32 = 2;
/// The spawn weight, in percent, of a wave entry when the manifest does not say.
pub const DEFAULT_CHANCE: u32 = 10;
/// The most cars of one kind a wave entry may ask for (a chase never has more than 8 cop cars).
pub const MAX_COUNT: u32 = 8;
/// The largest spawn weight, percent.
pub const MAX_CHANCE: u32 = 100;

/// What kind of police unit a car is: how the radio and the damage rules treat it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopClass {
    /// An ordinary patrol or pursuit car (the default).
    #[default]
    Patrol,
    /// A fast pursuit car of the high heat levels (called a super pursuit unit on the radio).
    Pursuit,
    /// A heavy unit, a sport utility or a van: sturdier, and called a heavy unit on the radio.
    Heavy,
}

/// One entry of a cop car in the wave of one heat level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopWave {
    /// The heat level, from 1.
    pub heat: u32,
    /// How many cars of the kind the wave wants, 1 to [`MAX_COUNT`].
    pub count: u32,
    /// The weight of the kind in the wave's pick, percent, 1 to [`MAX_CHANCE`].
    pub chance: u32,
}

/// What `[[cop]]` tables give, as the file holds them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawWave {
    heat: RawHeat,
    #[serde(default)]
    count: Option<u32>,
    #[serde(default)]
    chance: Option<u32>,
}

/// A `heat` of a `[[cop]]` table: one level or a list of levels.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawHeat {
    One(u32),
    Many(Vec<u32>),
}

/// The waves a manifest names: the short form `heat` (default numbers) and the `[[cop]]` tables, in level order. A level
/// named twice, in either form, is refused. Empty when the manifest names no level at all (the game then reads "every
/// level" for a cop car).
pub(super) fn check(short: &[u32], tables: Option<Vec<RawWave>>) -> Result<Vec<CopWave>, ContentError> {
    let mut waves: Vec<CopWave> =
        short.iter().map(|&heat| CopWave { heat, count: DEFAULT_COUNT, chance: DEFAULT_CHANCE }).collect();
    for table in tables.unwrap_or_default() {
        let count = table.count.unwrap_or(DEFAULT_COUNT);
        let chance = table.chance.unwrap_or(DEFAULT_CHANCE);
        if !(1..=MAX_COUNT).contains(&count) {
            return Err(ContentError::InvalidCopCount(count));
        }
        if !(1..=MAX_CHANCE).contains(&chance) {
            return Err(ContentError::InvalidCopChance(chance));
        }
        let levels = match table.heat {
            RawHeat::One(level) => vec![level],
            RawHeat::Many(levels) => levels,
        };
        if levels.is_empty() {
            return Err(ContentError::EmptyHeat);
        }
        for heat in levels {
            if heat == 0 {
                return Err(ContentError::InvalidHeat(0));
            }
            waves.push(CopWave { heat, count, chance });
        }
    }
    waves.sort_by_key(|wave| wave.heat);
    if let Some(pair) = waves.windows(2).find(|pair| pair[0].heat == pair[1].heat) {
        return Err(ContentError::RepeatedHeat(pair[0].heat));
    }
    Ok(waves)
}
