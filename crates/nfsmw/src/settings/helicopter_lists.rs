//! The helicopter options that take several numbers per line (`Leading = 30, 45`): a comma-separated list of up to
//! four parts under the same `default`, `heatNN` and `raceNN` keys as the single-number options. Each part has its own
//! range; a part the line leaves out keeps the vanilla number, a part that is not a number is the vanilla number and is
//! logged. The meaning of each part is the order of the mod's file (`docs/mods/nfsmwhelicopteroptions.md` section 5).
//! The mapping to the controller's constants is by value: the mod's defaults equal the vanilla constants
//! (`docs/specs/ai-helicopter-constants.md`) **[inferred]**.

use blackbox_pursuit::helicopter::Gains;
use ini_tuning::{Entry, Range, Read, read_numbers};

use super::helicopter::LEVELS;

/// One list option at each heat level and each race level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListLevels<const N: usize> {
    pub heat: [[f32; N]; LEVELS],
    pub race: [[f32; N]; LEVELS],
}

impl<const N: usize> ListLevels<N> {
    pub const fn flat(values: [f32; N]) -> Self {
        Self { heat: [values; LEVELS], race: [values; LEVELS] }
    }

    /// The numbers at `level` (clamped to 1 to 10), of the race table when `racing`.
    pub fn at(&self, level: u32, racing: bool) -> [f32; N] {
        let index = (level as usize).clamp(1, LEVELS) - 1;
        match racing {
            true => self.race[index],
            false => self.heat[index],
        }
    }
}

/// The list options of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct HeliLists {
    /// `Leading`: base and cap of the follow's lead, metres.
    pub leading: ListLevels<2>,
    /// `ClosingSpeed`: the follow's speed for a gap beyond 3, 15 and 300 m, m/s.
    pub closing_speed: ListLevels<3>,
    /// `Acceleration`: the cap and the base of the acceleration limit.
    pub acceleration: ListLevels<2>,
    /// `Brake`: the divisor of the stopping distance and the share of the speed kept while braking hard.
    pub brake: ListLevels<2>,
    /// `Turning`: the yaw rate limit and the rate per radian of error. The mod writes the second as -8.
    pub turning: ListLevels<2>,
    /// `SearchRadius` (80, 250 in the mod): read and held. The controller has no constant of these values (its search
    /// radius is the level's roadblock radius), so nothing applies them yet.
    pub search_radius: ListLevels<2>,
}

impl Default for HeliLists {
    fn default() -> Self {
        let vanilla = Gains::default();
        Self {
            leading: ListLevels::flat(vanilla.leading),
            closing_speed: ListLevels::flat(vanilla.closing_speed),
            acceleration: ListLevels::flat(vanilla.acceleration),
            brake: ListLevels::flat(vanilla.brake),
            turning: ListLevels::flat(vanilla.turning),
            search_radius: ListLevels::flat([80.0, 250.0]),
        }
    }
}

const METRES: Range = Range { min: 0.0, max: 5000.0 };
const SPEED: Range = Range { min: 1.0, max: 1000.0 };
const ACCELERATION: Range = Range { min: 1.0, max: 1000.0 };
const DIVISOR: Range = Range { min: 0.1, max: 100.0 };
const SHARE: Range = Range { min: 0.0, max: 1.0 };
const RATE: Range = Range { min: -100.0, max: 100.0 };

impl HeliLists {
    pub fn parse(e: &[Entry]) -> Self {
        let d = Self::default();
        Self {
            leading: list(e, "Helicopter:Leading", [METRES; 2], d.leading),
            closing_speed: list(e, "Helicopter:ClosingSpeed", [SPEED; 3], d.closing_speed),
            acceleration: list(e, "Helicopter:Acceleration", [ACCELERATION; 2], d.acceleration),
            brake: list(e, "Helicopter:Brake", [DIVISOR, SHARE], d.brake),
            turning: list(e, "Helicopter:Turning", [RATE; 2], d.turning),
            search_radius: list(e, "Helicopter:SearchRadius", [METRES; 2], d.search_radius),
        }
    }

    /// The controller's numbers for `level`. The turning gain is used by its size: the mod writes it as -8.
    pub fn gains(&self, level: u32, racing: bool) -> Gains {
        let turning = self.turning.at(level, racing);
        Gains {
            leading: self.leading.at(level, racing),
            closing_speed: self.closing_speed.at(level, racing),
            acceleration: self.acceleration.at(level, racing),
            brake: self.brake.at(level, racing),
            turning: [turning[0].abs().max(0.01), turning[1].abs().max(0.1)],
        }
    }
}

/// One list option from `section`: a level's own line, else the `default` line, else `vanilla`, part by part.
fn list<const N: usize>(e: &[Entry], section: &str, ranges: [Range; N], vanilla: ListLevels<N>) -> ListLevels<N> {
    let fallback = parts(e, section, "default", ranges);
    let row = |prefix: &str, vanilla: &[[f32; N]; LEVELS]| -> [[f32; N]; LEVELS] {
        std::array::from_fn(|i| {
            let own = parts(e, section, &format!("{prefix}{:02}", i + 1), ranges);
            std::array::from_fn(|slot| own[slot].or(fallback[slot]).unwrap_or(vanilla[i][slot]))
        })
    };
    ListLevels { heat: row("heat", &vanilla.heat), race: row("race", &vanilla.race) }
}

/// The usable part values of one line; a clamped or malformed part is logged.
fn parts<const N: usize>(e: &[Entry], section: &str, key: &str, ranges: [Range; N]) -> [Option<f32>; N] {
    let reads = read_numbers(e, section, key, ranges);
    for (slot, read) in reads.iter().enumerate() {
        match read {
            Read::Clamped(v) => {
                log::warn!("helicopter: [{section}] {key} part {} is out of range, using {v}", slot + 1)
            }
            Read::Malformed => {
                log::warn!("helicopter: [{section}] {key} part {} is not a number, using the vanilla value", slot + 1)
            }
            Read::Absent | Read::Value(_) => {}
        }
    }
    reads.map(Read::value)
}

#[cfg(test)]
#[path = "helicopter_lists_tests.rs"]
mod tests;
