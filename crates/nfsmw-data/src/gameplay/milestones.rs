//! The career milestone tables: the pursuit goals per Blacklist bin, the speed traps, the statistic types they track and
//! the rival requirements of each bin. Numbers and evidence: `docs/specs/career-milestones-data.md`; rules:
//! `docs/specs/career-milestones.md` sections 1 and 8.
//!
//! The milestones, speed traps and rival requirements are `gameplay` collections (`GLOBAL/gameplay.bin`). The
//! statistic types (`milestonetypes`) are read from `GLOBAL/attributes.bin`, so their reader needs the attributes
//! database as well. Every reader takes the whole [`Database`], like the rest of this module.

use blackbox_attrib::{CollectionRef, Database, Value};
use glam::Vec3;

use super::{CLASS, inherits, instances, key_field, name_of, text, to_world};

/// The class of the statistic types (in `GLOBAL/attributes.bin`).
const TYPES_CLASS: &str = "milestonetypes";
/// The number of Blacklist bins (`milestones/bin_01` to `bin_15`, `race_bin_01` to `race_bin_15`).
const BINS: u8 = 15;

/// Field `0x8445AF47` (`f32`): the threshold that completes the milestone. Name not known.
const THRESHOLD_FIELD: u32 = 0x8445_AF47;
/// Field `0x3B9BBFC2` (`f32`): a second, "harder" number. Unconfirmed; no code found reads it.
const GOAL_FIELD: u32 = 0x3B9B_BFC2;
/// The step field (`f32`): negative for evasion time. Unconfirmed; no code found reads it.
const STEP_FIELD: u32 = 0x006E_C903;

/// Field `0xAB4A196F` (`bool`) of a statistic type: the statistic restarts at each player pursuit.
const PER_PURSUIT_FIELD: u32 = 0xAB4A_196F;
/// Field `0xDB89AB5C` (`i32`) of a statistic type: the index `NN` of the `BLACKLIST_PURSUIT_MILESTONES_NN` label.
const LABEL_FIELD: u32 = 0xDB89_AB5C;

/// Field `0x39BF8002` (`i32`) on three speed traps (25 or 30). Unconfirmed; no use found.
const TRAP_UNKNOWN_FIELD: u32 = 0x39BF_8002;

/// One pursuit goal of a bin (55 in total, three or four per bin).
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `milestones/bin_01/challenge_1_cops_damaged`.
    pub name: String,
    /// `BinIndex`, 1 to 15 (0 when unset).
    pub bin: u8,
    /// The kind in the name, for example `cops_damaged`. `None` when the name does not have the form.
    pub kind: Option<String>,
    /// `MilestoneName`: the statistic it tracks, for example `pursuit_length`. Bounty goals of kind `cops_destroyed`
    /// track `bounty_in_pursuit`, cost goals track `cost_to_state_in_pursuit`.
    pub statistic: String,
    /// The threshold that completes it (seconds for the time kinds, points or dollars for bounty and cost, counts
    /// otherwise). Passing it only makes the milestone "at stake"; banking happens at the end of an escape.
    pub threshold: f32,
    /// `Bounty`: the reward in bounty points (rep points), not cash. 0 when unset.
    pub bounty: i32,
    /// `SpawnPoint`: key of the tagged car marker that the rap sheet pins on the map. `None` when unset.
    pub spawn_point: Option<u32>,
    /// Field `0x3B9BBFC2`. Unconfirmed: possibly the last step of a ladder; no code reads it. 0 when unset.
    pub unconfirmed_goal: f32,
    /// The step field. Unconfirmed: possibly the spacing of a ladder; no code reads it. 0 when unset.
    pub unconfirmed_step: f32,
}

/// One speed trap (39 in total: 3 in bins 1 to 10, 2 in bins 11 to 14, 1 in bin 15).
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedTrap {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `speedtraps/bin_01/speedtrap1`.
    pub name: String,
    /// `BinIndex`, 1 to 15 (0 when unset).
    pub bin: u8,
    /// `ThreshholdSpeed` in km/h (the field's spelling). The trap must be crossed faster than this.
    pub threshold_kmh: f32,
    /// `Bounty`: the reward in bounty points, the same for every trap of a bin. 0 when unset.
    pub bounty: i32,
    /// `Position` in physics space ([`to_world`]). `None` when unset.
    pub position: Option<Vec3>,
    /// `SpawnPoint`: key of the tagged car marker next to the trap. `None` when unset.
    pub spawn_point: Option<u32>,
    /// Field `0x39BF8002` (25 or 30 on three traps). Unconfirmed; use not found.
    pub unconfirmed_field: Option<i32>,
}

/// A statistic that milestones can track (30 in total, `milestonetypes` in `GLOBAL/attributes.bin`).
#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneType {
    /// The statistic name, which is also the collection name, for example `pursuit_evasion_time`.
    pub name: String,
    /// `MilestoneType`: 1 or 3 when bigger is better, 2 when smaller is better.
    pub kind: i32,
    /// Field `0xAB4A196F`: the statistic is cleared at the start of each player pursuit.
    pub per_pursuit: bool,
    /// Field `0xDB89AB5C`: the `NN` of the `BLACKLIST_PURSUIT_MILESTONES_NN` label. `None` when absent.
    pub label: Option<i32>,
}

impl MilestoneType {
    /// Whether a bigger value is better. Only `pursuit_evasion_time` is smaller-is-better in the data.
    pub fn bigger_is_better(&self) -> bool {
        self.kind != 2
    }
}

/// The rival requirements of one bin (`race_bin_NN`), the three conditions that unlock the rival.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RivalRequirements {
    /// The bin, 1 to 15.
    pub bin: u8,
    /// `RequiredRacesWon`: races won in the bin.
    pub races_won: i32,
    /// `RequiredBounty`: the player's total bounty over all cars.
    pub bounty: i32,
    /// `RequiredChallenges`: completed milestones plus completed speed traps of the bin.
    pub challenges: i32,
}

/// Every milestone of the database, sorted by bin and then name. Templates are skipped; the kind template each one
/// inherits from is what gives it its statistic.
pub fn milestones(db: &Database) -> Vec<Milestone> {
    let mut out: Vec<Milestone> = instances(db).filter(|c| inherits(*c, "milestone")).map(milestone).collect();
    out.sort_by(|a, b| a.bin.cmp(&b.bin).then_with(|| a.name.cmp(&b.name)));
    out
}

/// Every speed trap of the database, sorted by bin and then name.
pub fn speed_traps(db: &Database) -> Vec<SpeedTrap> {
    let mut out: Vec<SpeedTrap> = instances(db).filter(|c| inherits(*c, "speedtrap")).map(speed_trap).collect();
    out.sort_by(|a, b| a.bin.cmp(&b.bin).then_with(|| a.name.cmp(&b.name)));
    out
}

/// The statistic types, in database order. They are in `GLOBAL/attributes.bin`, so the database must hold it.
pub fn milestone_types(db: &Database) -> Vec<MilestoneType> {
    db.collections_of(TYPES_CLASS).map(milestone_type).collect()
}

/// The rival requirements of bins 1 to 15, in bin order. A bin missing from the database is skipped.
///
/// Unconfirmed: the `race_bin_NN` collections are read as class `gameplay`; the class was not checked.
pub fn rival_requirements(db: &Database) -> Vec<RivalRequirements> {
    (1..=BINS).filter_map(|bin| rival(db, bin)).collect()
}

fn milestone(c: CollectionRef<'_>) -> Milestone {
    let name = name_of(c);
    Milestone {
        key: c.key(),
        kind: kind_from_name(&name).map(str::to_owned),
        bin: bin_of(c),
        statistic: text(c, "MilestoneName").unwrap_or_default(),
        threshold: f32_by_key(c, THRESHOLD_FIELD).unwrap_or(0.0),
        bounty: c.get_i32("Bounty").unwrap_or(0),
        spawn_point: key_field(c, "SpawnPoint"),
        unconfirmed_goal: f32_by_key(c, GOAL_FIELD).unwrap_or(0.0),
        unconfirmed_step: f32_by_key(c, STEP_FIELD).unwrap_or(0.0),
        name,
    }
}

fn speed_trap(c: CollectionRef<'_>) -> SpeedTrap {
    SpeedTrap {
        key: c.key(),
        name: name_of(c),
        bin: bin_of(c),
        threshold_kmh: c.get_f32("ThreshholdSpeed").unwrap_or(0.0),
        bounty: c.get_i32("Bounty").unwrap_or(0),
        position: c.get_vector3("Position").map(to_world),
        spawn_point: key_field(c, "SpawnPoint"),
        unconfirmed_field: c.get_by_key(TRAP_UNKNOWN_FIELD).and_then(Value::as_i32),
    }
}

fn milestone_type(c: CollectionRef<'_>) -> MilestoneType {
    MilestoneType {
        name: name_of(c),
        kind: c.get_i32("MilestoneType").unwrap_or(1),
        per_pursuit: c.get_by_key(PER_PURSUIT_FIELD).and_then(Value::as_bool).unwrap_or(false),
        label: c.get_by_key(LABEL_FIELD).and_then(Value::as_i32),
    }
}

fn rival(db: &Database, bin: u8) -> Option<RivalRequirements> {
    let c = db.collection(CLASS, &race_bin_name(bin))?;
    Some(RivalRequirements {
        bin,
        races_won: c.get_i32("RequiredRacesWon").unwrap_or(0),
        bounty: c.get_i32("RequiredBounty").unwrap_or(0),
        challenges: c.get_i32("RequiredChallenges").unwrap_or(0),
    })
}

/// The bin of a collection, from `BinIndex`. 0 when unset or out of range.
fn bin_of(c: CollectionRef<'_>) -> u8 {
    c.get_i32("BinIndex").and_then(|bin| u8::try_from(bin).ok()).unwrap_or(0)
}

/// An `f32` field by its hash, for fields whose name is not known. Inherited like [`CollectionRef::get`].
fn f32_by_key(c: CollectionRef<'_>, key: u32) -> Option<f32> {
    c.get_by_key(key)?.as_f32()
}

/// The kind in a milestone's name: `milestones/bin_01/challenge_1_cops_damaged` gives `cops_damaged`.
fn kind_from_name(name: &str) -> Option<&str> {
    let leaf = name.rsplit('/').next()?;
    let rest = leaf.strip_prefix("challenge_")?;
    let (number, kind) = rest.split_once('_')?;
    let is_number = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    is_number.then_some(kind).filter(|kind| !kind.is_empty())
}

/// The name of a bin's rival collection: bin 3 gives `race_bin_03`.
fn race_bin_name(bin: u8) -> String {
    format!("race_bin_{bin:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_is_the_part_after_the_challenge_number() {
        assert_eq!(kind_from_name("milestones/bin_01/challenge_1_cops_damaged"), Some("cops_damaged"));
        assert_eq!(kind_from_name("milestones/bin_08/challenge_4_tire_spikes_dodged"), Some("tire_spikes_dodged"));
    }

    #[test]
    fn names_without_the_challenge_form_have_no_kind() {
        assert_eq!(kind_from_name("milestones/bin_01/taggedcarmarker1"), None);
        assert_eq!(kind_from_name("milestones/default_spawn_point"), None);
        assert_eq!(kind_from_name("challenge__cops"), None);
        assert_eq!(kind_from_name("challenge_1_"), None);
        assert_eq!(kind_from_name("challenge_x_cops"), None);
    }

    #[test]
    fn rival_collections_are_two_digits() {
        assert_eq!(race_bin_name(1), "race_bin_01");
        assert_eq!(race_bin_name(15), "race_bin_15");
    }

    #[test]
    fn only_type_two_is_smaller_is_better() {
        let mut t = MilestoneType { name: String::new(), kind: 1, per_pursuit: false, label: None };
        assert!(t.bigger_is_better());
        t.kind = 3;
        assert!(t.bigger_is_better());
        t.kind = 2;
        assert!(!t.bigger_is_better());
    }
}
