//! The career tables read from the install, converted to the rule crate's types (`nfsmw-career`).
//!
//! The readers in `nfsmw-data` give the data as the install stores it: collection keys, data names, `i32` bounties.
//! This module keeps only what the rules need:
//!
//! - Blacklist bins 1 to 15 ([`BinDef`]). Race collection keys become event ids (`1.5.2`), see [`event_id`].
//! - The 55 pursuit milestones ([`MilestoneDef`]) and the 39 speed traps ([`SpeedTrapDef`]).
//! - The car lot ([`LotCar`]).
//! - The heat values and the rap sheet thresholds, as the readers give them.
//!
//! Entries that do not convert are skipped, with no log line. The skips are listed on each conversion.

use anyhow::{Context, Result};
use blackbox_attrib::{Database, vlt_hash};
use blackbox_vehicle::performance::measure_with_integration;
use game_install::GameDir;
use nfsmw_career::blacklist::BinDef;
use nfsmw_career::carlot::stats::{CarStats, SPEED_100_MPH, time_to_speed};
use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::milestones::speedtrap::{self, SpeedTrapDef};
use nfsmw_career::milestones::{MilestoneDef, StatType};
use nfsmw_data::car::physics::PhysicsData;
use nfsmw_data::career_cars::{self, CarClass, CatalogueCar};
use nfsmw_data::gameplay::{self, GameplayData};
use nfsmw_data::pursuit::{self, CareerHeat};
use nfsmw_data::rank_thresholds::{self, RankThresholds};
use nfsmw_data::{read_unwrapped, shared};

/// The Blacklist bins the rules take: bins 1 to 15, exactly as `Blacklist::new` needs them.
const FIRST_BIN: u8 = 1;
const LAST_BIN: u8 = 15;

/// The career tables of one install, in the rule crate's types.
#[derive(Debug, Clone, PartialEq)]
pub struct CareerData {
    /// Blacklist bins 1 to 15, in bin order.
    pub bins: Vec<BinDef>,
    /// The pursuit milestones of bins 1 to 15, in the reader's order (bin, then name).
    pub milestones: Vec<MilestoneDef>,
    /// The speed traps of bins 1 to 15, in the reader's order (bin, then name).
    pub traps: Vec<SpeedTrapDef>,
    /// The car lot's cars, in the reader's order.
    pub cars: Vec<LotCar>,
    /// The stat bars' raw stats of each car, parallel to `cars`. `None` when the car's physics cannot be read (and
    /// everywhere the tables are built without an install, see [`CareerData::from_parts`]).
    pub stats: Vec<Option<CarStats>>,
    /// The cop heat values. `None` when `pursuitlevels/default` is missing from the attribute database.
    pub heat: Option<CareerHeat>,
    /// The rap sheet thresholds. `None` when a ranking table is missing or malformed.
    pub ranks: Option<RankThresholds>,
    /// The gameplay data the bins, milestones and traps came from (races, triggers, markers, types).
    pub gameplay: GameplayData,
}

impl CareerData {
    /// Read the career tables from the install.
    ///
    /// Gameplay comes from `GLOBAL/gameplay.bin`. The car lot, the heat and the rap sheet read `GLOBAL/FE_ATTRIB.bin`
    /// and `pursuitlevels` from the attribute database. The attribute database is parsed once per session (shared).
    pub fn load(dir: &GameDir) -> Result<CareerData> {
        let gameplay = gameplay::load(dir)?;
        let attributes = shared::attributes(dir)?;
        let frontend = frontend_database(dir, &attributes)?;
        let cars = career_cars::catalogue(&frontend);
        let heat = pursuit::career_heat(&attributes);
        let ranks = rank_thresholds::rank_thresholds(&frontend);
        let mut data = CareerData::from_parts(gameplay, &cars, heat, ranks);
        data.stats = lot_stats(dir, &data.cars);
        Ok(data)
    }

    /// Build the tables from data already read. Pure: no install is touched, so tests can pass synthetic inputs.
    pub fn from_parts(
        gameplay: GameplayData,
        cars: &[CatalogueCar],
        heat: Option<CareerHeat>,
        ranks: Option<RankThresholds>,
    ) -> CareerData {
        let bins = gameplay.bins.iter().filter_map(|bin| blacklist_bin(&gameplay, bin)).collect();
        let milestones = gameplay.milestones.iter().filter_map(milestone_def).collect();
        let traps = gameplay.speed_traps.iter().filter_map(trap_def).collect();
        let cars: Vec<LotCar> = cars.iter().filter_map(lot_car).collect();
        let stats = vec![None; cars.len()];
        CareerData { bins, milestones, traps, cars, stats, heat, ranks, gameplay }
    }
}

/// The stat bars' raw stats of each lot car, parallel to `cars`. The physics are read once for the whole lot. A car
/// whose physics cannot be read has `None`; when the physics as a whole cannot load, every car does.
///
/// Each car is at stock (no upgrades): `PhysicsData::car` is the stock spec, not `car_with_performance`. **[unconfirmed]**
/// the lot car's `name` is taken as the physics type name (the `pvehicle` collection of the same name).
fn lot_stats(dir: &GameDir, cars: &[LotCar]) -> Vec<Option<CarStats>> {
    let Ok(physics) = PhysicsData::load(dir) else { return vec![None; cars.len()] };
    cars.iter().map(|car| car_stats(&physics, &car.name)).collect()
}

/// One car's stats from the repo's estimator. **[unconfirmed]** the estimator (`blackbox_vehicle::performance`) is the
/// repo's own measurement, not the game's simulation: top speed is its top speed, time to 100 mph is integrated from its
/// acceleration table, and handling is its start grip. `None` when the car's physics cannot be read.
fn car_stats(physics: &PhysicsData, name: &str) -> Option<CarStats> {
    let car = physics.car(name).ok()?;
    let performance = measure_with_integration(&car.spec, car.tire_integration);
    let time_to_100 = time_to_speed(SPEED_100_MPH, performance.top_speed, |speed| performance.acceleration_at(speed));
    Some(CarStats { top_speed: performance.top_speed, handling: performance.start_grip, time_to_100 })
}

/// The attribute database with `GLOBAL/FE_ATTRIB.bin` added: the `frontend` collections of the car lot and the rap
/// sheet tables. The shared attribute database itself is not changed.
fn frontend_database(dir: &GameDir, attributes: &Database) -> Result<Database> {
    let mut db = Database::clone(attributes);
    let bytes = read_unwrapped(dir, career_cars::FE_ATTRIB_FILE)?;
    db.load(&bytes).with_context(|| format!("parsing {}", career_cars::FE_ATTRIB_FILE))?;
    Ok(db)
}

/// One Blacklist bin. `None` for a bin outside 1 to 15 (16 is the prologue, 18 to 21 are the side bins) and for a
/// count that is negative. Race keys with no race in `gameplay.races` are left out of the lists.
fn blacklist_bin(gameplay: &GameplayData, bin: &gameplay::Bin) -> Option<BinDef> {
    let number = u8::try_from(bin.number).ok().filter(|n| (FIRST_BIN..=LAST_BIN).contains(n))?;
    Some(BinDef {
        number,
        races: event_ids(gameplay, &bin.world_races),
        races_needed: u32::try_from(bin.required_races_won).ok()?,
        challenges_needed: u32::try_from(bin.required_challenges).ok()?,
        bounty_needed: u32::try_from(bin.required_bounty).ok()?,
        rival_events: event_ids(gameplay, &bin.boss_races),
    })
}

/// The event ids of the races with these collection keys, in order. Keys with no race are skipped.
fn event_ids(gameplay: &GameplayData, keys: &[u32]) -> Vec<String> {
    keys.iter().filter_map(|key| race_name(gameplay, *key)).map(event_id).collect()
}

/// The data name of the race with this collection key.
fn race_name(gameplay: &GameplayData, key: u32) -> Option<&str> {
    gameplay.races.iter().find(|race| race.key == key).map(|race| race.name.as_str())
}

/// The event id the Blacklist uses for a race, from its data name: the first three numbers of the leaf name joined
/// with dots. `race_bin_10/10_2_1_sprint` gives `10.2.1`.
///
/// A leaf without three leading numbers (for example `epic_pursuit`) gives the leaf name itself. **[unconfirmed]**:
/// the specs name `1.8.1` as the final race, but the data name of that race is not checked here, so it may not match.
pub fn event_id(race_name: &str) -> String {
    let leaf = race_name.rsplit('/').next().unwrap_or(race_name);
    let parts: Vec<&str> = leaf.split('_').take(3).collect();
    let numeric = parts.len() == 3 && parts.iter().all(|part| is_number(part));
    if !numeric {
        return leaf.to_owned();
    }
    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

/// The event id of a race: the one it stores (`19.8.18`, the challenges and the side bins), else the one its data name
/// gives ([`event_id`]).
pub fn race_event_id(race: &gameplay::Race) -> String {
    race.event.clone().unwrap_or_else(|| event_id(&race.name))
}

/// A non-empty run of ASCII digits.
fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// One milestone. `None` when its bin is outside 1 to 15, its statistic is not one of the 30 tracked statistics, or
/// its bounty is negative.
fn milestone_def(milestone: &gameplay::Milestone) -> Option<MilestoneDef> {
    let bin = Some(milestone.bin).filter(|b| (FIRST_BIN..=LAST_BIN).contains(b))?;
    let stat = StatType::ALL.iter().copied().find(|stat| stat.name() == milestone.statistic)?;
    let bounty = u32::try_from(milestone.bounty).ok()?;
    Some(MilestoneDef { key: milestone.key, bin, stat, threshold: milestone.threshold, bounty })
}

/// One speed trap. The trap number is the trailing digits of the data name (`speedtraps/bin_01/speedtrap1` is trap
/// 1). The threshold and bounty come from the spec's table (`speedtrap::def_for`), not from the data row. `None` when
/// there is no trailing number or the table has no such trap (for example trap 3 of bin 15).
fn trap_def(trap: &gameplay::SpeedTrap) -> Option<SpeedTrapDef> {
    let index = trap_index(&trap.name)?;
    speedtrap::def_for(trap.bin, index, trap.key)
}

/// The trailing number of a data name's leaf.
fn trap_index(name: &str) -> Option<u8> {
    let leaf = name.rsplit('/').next()?;
    let digits: String = leaf.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    let digits: String = digits.chars().rev().collect();
    digits.parse().ok()
}

/// One car of the lot. `None` when the car has no name, no unlock rank or no class (the rules need all three).
///
/// The handle and the vehicle key are both the name hash (`vlt_hash` of the name). **[unconfirmed]** the spec does not
/// say which handle a bought car gets, nor which vehicle collection the lot record points at; the name hash is what
/// the preset cars use. A negative price becomes 0 (the rules take a `u32`).
fn lot_car(car: &CatalogueCar) -> Option<LotCar> {
    let name = car.name.clone()?;
    let unlock_rank = car.unlock_rank?;
    let class = lot_class(car.class?);
    Some(LotCar {
        handle: vlt_hash(&name),
        vehicle_key: vlt_hash(&name),
        name,
        frontend_key: car.key,
        price: u32::try_from(car.price).unwrap_or(0),
        unlock_rank,
        for_sale: car.for_sale,
        class,
    })
}

/// The lot's class of a reader class. The root class is the rules' `Vehicles`.
fn lot_class(class: CarClass) -> LotClass {
    match class {
        CarClass::NorthAmerica => LotClass::NorthAmerica,
        CarClass::Europe => LotClass::Europe,
        CarClass::Japan => LotClass::Japan,
        CarClass::Root => LotClass::Vehicles,
    }
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
