//! The cars of the custom race's car select, in the lists the player scrolls through with up and down
//! (`docs/specs/custom-race.md` section 5). The original has six lists (`FE_CAR_FILTER_LIST_*`): Stock (every car of the
//! catalogue), Career (the cars of the loaded profile), My Cars (the quick race garage, which this build does not have: the list
//! is always empty), Bonus (the rivals' cars and the unlockable specials), Preset (the career-start M3 GTR) and a debug list,
//! which is not offered.

use super::super::quick_play::pool::{CarPool, PoolCar, car_unlocked};
use super::super::quick_play::text::{self, Unlock};

/// The preset the player's first car is: the BMW M3 GTR of the career start, which the menus show.
pub const DEFAULT_PRESET: &str = "M3GTRCAREERSTART";

/// A list of the car select.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    Stock,
    Career,
    MyCars,
    Bonus,
    Preset,
}

impl List {
    pub const ALL: [List; 5] = [List::Stock, List::Career, List::MyCars, List::Bonus, List::Preset];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|list| *list == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> List {
        Self::ALL.get(index).copied().unwrap_or(List::Stock)
    }

    /// The next list (`forward`) or the previous one, wrapping.
    pub fn step(self, forward: bool) -> List {
        let count = Self::ALL.len();
        Self::from_index(match forward {
            true => (self.index() + 1) % count,
            false => (self.index() + count - 1) % count,
        })
    }

    /// The list's name and the picture beside it **[decomp]** (`UIQRCarSelect::RefreshHeader`).
    pub fn label(self) -> u32 {
        match self {
            List::Stock => 0xD9D6_B954,
            List::Career => 0xEE05_3562,
            List::MyCars => 0x2414_DE28,
            List::Bonus => 0xD8A0_58F7,
            List::Preset => 0x0D85_00C3,
        }
    }

    pub fn icon(self) -> u32 {
        match self {
            List::Stock => 0x3A54_1F7F,
            List::Career => 0xF0BD_DECD,
            List::MyCars => 0x9996_CA1E,
            List::Bonus => 0xBE5A_D8A2,
            List::Preset => 0x0370_4F3D,
        }
    }
}

/// A car of a list.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub car: PoolCar,
    /// The preset the car is built with, for the Bonus and Preset lists.
    pub preset: Option<String>,
    /// The name the choice is remembered by (the catalogue name, or the preset's name).
    pub key: String,
    pub locked: bool,
    /// What the locked line says.
    pub unlock: Unlock,
}

/// What the career has done that opens the bonus cars.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// The Blacklist stage (16 before the first rival, 1 for the last).
    pub stage: u8,
    /// The career was won (`HasBeatenCareer`), which opens the C6R.
    pub career_won: bool,
    /// The profile was awarded the Castrol GT (`HasBeenAwardedCastrolGT`), which opens it.
    pub castrol_awarded: bool,
    /// Every car is open, a developer's switch.
    pub open_all: bool,
    /// The catalogue names of the cars the loaded profile owns, for the Career list.
    pub owned: Vec<String>,
    /// The catalogue names of the profile's quick race custom cars, for the My Cars list.
    pub customs: Vec<String>,
    /// What the challenge series has been won of, which opens the SL 65 and the GT2.
    pub series: crate::frontend::challenge_series::Completion,
}

/// The cars of a garage list: the named cars of the pool, all open. A car the pool does not know is left out.
fn garage_entries(names: &[String], prefix: &str, pool: &CarPool) -> Vec<Entry> {
    names
        .iter()
        .filter_map(|name| pool.cars.iter().find(|car| car.name.eq_ignore_ascii_case(name)))
        .map(|car| Entry {
            car: car.clone(),
            preset: None,
            key: format!("{prefix}:{}", car.name),
            locked: false,
            unlock: Unlock::None,
        })
        .collect()
}

/// The cars of a list.
pub fn entries(list: List, pool: &CarPool, progress: &Progress) -> Vec<Entry> {
    match list {
        List::Stock => pool.cars.iter().map(|car| stock_entry(car, progress)).collect(),
        List::Career => garage_entries(&progress.owned, "career", pool),
        List::MyCars => garage_entries(&progress.customs, "custom", pool),
        List::Bonus => {
            pool.presets.iter().filter_map(|preset| bonus_entry(&preset.name, &preset.car, progress)).collect()
        }
        List::Preset => pool
            .presets
            .iter()
            .filter(|preset| preset.name.eq_ignore_ascii_case(DEFAULT_PRESET))
            .map(|preset| Entry {
                car: preset.car.clone(),
                preset: Some(preset.name.clone()),
                key: preset.name.clone(),
                locked: false,
                unlock: Unlock::None,
            })
            .collect(),
    }
}

/// The car types a quick race always offers (`QuickRaceUnlocker::IsCarUnlocked`) **[decomp]**, as folder names. The install
/// spells the Subaru's `IMPREZZAWRX`.
const STARTERS: [&str; 11] =
    ["RX8", "IMPREZAWRX", "IMPREZZAWRX", "MUSTANGGT", "SL500", "997S", "IS300", "GTI", "GALLARDO", "COBALTSS", "PUNTO"];

fn stock_entry(car: &PoolCar, progress: &Progress) -> Entry {
    let starter = STARTERS.iter().any(|name| name.eq_ignore_ascii_case(&car.folder));
    let open = progress.open_all || starter || car_unlocked(car.rank, progress.stage);
    Entry {
        car: car.clone(),
        preset: None,
        key: car.name.clone(),
        locked: !open,
        unlock: if open { Unlock::None } else { text::unlock_of(car, progress.stage) },
    }
}

/// How a bonus car opens **[decomp]** (`QuickRaceUnlocker::IsCarUnlocked`): a rival's car when rival `n` is beaten (the stage is
/// below `n`), the C6R when the career is won, the SL65 with event 68 of the series and the GT2 with every event of it, the Castrol
/// GT once the profile was awarded it, the collector's cars never.
fn bonus_entry(preset: &str, car: &PoolCar, progress: &Progress) -> Option<Entry> {
    let upper = preset.to_ascii_uppercase();
    let (open, unlock) = match upper.as_str() {
        "BONUS_C6R" => (progress.career_won, Unlock::Label(0xBD8B_AC94)),
        "BONUS_SL65" => (progress.series.special, Unlock::Label(0xBD8B_AC91)),
        "BONUS_GT2" => (progress.series.all, Unlock::Label(0xBD8B_AC92)),
        "CASTROLGT" => (progress.castrol_awarded, Unlock::None),
        _ => {
            let member: u8 = upper.strip_prefix("BL")?.parse().ok().filter(|n| (2..=15).contains(n))?;
            (progress.stage < member, Unlock::Rival(member))
        }
    };
    Some(Entry {
        car: car.clone(),
        preset: Some(preset.to_owned()),
        key: preset.to_owned(),
        locked: !(open || progress.open_all),
        unlock: if open { Unlock::None } else { unlock },
    })
}

/// The list and position of the car remembered under `key`, in the lists of a pool; `None` when no list has it.
pub fn find(pool: &CarPool, progress: &Progress, key: &str) -> Option<(List, usize)> {
    List::ALL.into_iter().find_map(|list| {
        let at = entries(list, pool, progress).iter().position(|entry| entry.key.eq_ignore_ascii_case(key))?;
        Some((list, at))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::challenge_series::Completion;
    use crate::frontend::quick_play::pool::PoolPreset;

    fn car(name: &str, folder: &str, rank: u8) -> PoolCar {
        PoolCar { name: name.into(), folder: folder.into(), rank, handle: 1, manufacturer: Some(1), stats: None }
    }

    fn pool() -> CarPool {
        let preset = |name: &str| PoolPreset { name: name.into(), car: car("m3", "BMWM3GTRE46", 0) };
        CarPool {
            cars: vec![car("cobaltss", "COBALTSS", 3), car("gt2", "911GT2", 3), car("a3", "A3", 15)],
            population: Vec::new(),
            presets: vec![preset("BL6"), preset("M3GTRCAREERSTART"), preset("BONUS_C6R"), preset("CE_ELISE")],
        }
    }

    fn progress(stage: u8) -> Progress {
        Progress {
            stage,
            career_won: false,
            castrol_awarded: false,
            open_all: false,
            owned: Vec::new(),
            customs: Vec::new(),
            series: Default::default(),
        }
    }

    #[test]
    fn the_stock_list_opens_by_rank_and_for_the_starter_cars() {
        let locked =
            |stage| entries(List::Stock, &pool(), &progress(stage)).iter().map(|e| e.locked).collect::<Vec<_>>();
        assert_eq!(
            locked(16),
            [false, true, true],
            "the Cobalt is a starter; the GT2 (rank 3) and A3 (rank 15) follow rank"
        );
        assert_eq!(locked(4), [false, true, false]);
        assert_eq!(locked(2), [false, false, false]);
    }

    #[test]
    fn a_rivals_car_opens_when_the_rival_is_beaten() {
        let bonus = |stage| entries(List::Bonus, &pool(), &progress(stage));
        assert_eq!(bonus(16).iter().map(|e| e.key.as_str()).collect::<Vec<_>>(), ["BL6", "BONUS_C6R"]);
        assert!(bonus(16)[0].locked);
        assert_eq!(bonus(16)[0].unlock, Unlock::Rival(6));
        assert!(!bonus(5)[0].locked);
        assert_eq!(bonus(5)[0].unlock, Unlock::None);
        assert!(bonus(1)[1].locked, "the C6R needs the career won");
        let won = entries(List::Bonus, &pool(), &Progress { career_won: true, ..progress(1) });
        assert!(!won[1].locked);
    }

    #[test]
    fn the_series_cars_open_with_the_series_wins() {
        let preset = |name: &str| PoolPreset { name: name.into(), car: car("m3", "BMWM3GTRE46", 0) };
        let pool = CarPool {
            presets: vec![preset("BONUS_SL65"), preset("BONUS_GT2"), preset("CASTROLGT")],
            ..CarPool::default()
        };
        let locked = |series| {
            let progress = Progress { series, ..progress(16) };
            entries(List::Bonus, &pool, &progress).iter().map(|e| e.locked).collect::<Vec<_>>()
        };
        let none = Completion::default();
        assert_eq!(locked(none), [true, true, true]);
        assert_eq!(locked(Completion { special: true, all: false }), [false, true, true], "event 68 opens the SL 65");
        assert_eq!(
            locked(Completion { special: true, all: true }),
            [false, false, true],
            "the Castrol GT waits for its award"
        );
        let awarded = Progress { castrol_awarded: true, ..progress(16) };
        assert!(!entries(List::Bonus, &pool, &awarded)[2].locked, "the award opens the Castrol GT");
    }

    #[test]
    fn the_career_list_holds_the_cars_the_profile_owns_and_my_cars_the_customs() {
        let owned = Progress { owned: vec!["cobaltss".into(), "unknown".into()], ..progress(16) };
        let career = entries(List::Career, &pool(), &owned);
        assert_eq!(career.len(), 1, "a car the pool does not know is left out");
        assert_eq!(career[0].car.name, "cobaltss");
        assert!(!career[0].locked);
        assert!(entries(List::Career, &pool(), &progress(16)).is_empty());
        assert!(entries(List::MyCars, &pool(), &owned).is_empty());
        let customs = Progress { customs: vec!["cobaltss".into()], ..progress(16) };
        let mine = entries(List::MyCars, &pool(), &customs);
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].key, "custom:cobaltss");
        assert!(entries(List::Career, &pool(), &customs).is_empty(), "a custom car is not a career car");
    }

    #[test]
    fn the_preset_list_holds_the_career_start_car() {
        let list = entries(List::Preset, &pool(), &progress(16));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].preset.as_deref(), Some(DEFAULT_PRESET));
        assert!(!list[0].locked);
    }

    #[test]
    fn the_lists_step_round_and_a_remembered_car_is_found() {
        assert_eq!(List::Stock.step(false), List::Preset);
        assert_eq!(List::Preset.step(true), List::Stock);
        assert_eq!(List::Stock.step(true), List::Career);
        assert_eq!(find(&pool(), &progress(16), "BL6"), Some((List::Bonus, 0)));
        assert_eq!(find(&pool(), &progress(16), "a3"), Some((List::Stock, 2)));
        assert_eq!(find(&pool(), &progress(16), "nothing"), None);
    }
}
