//! The traffic patterns of the attribute database: which cars fill the roads of each neighbourhood.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§3, §4).

use blackbox_attrib::{Database, Value, vlt_hash};

/// The pattern in force before any zone names another one.
pub const DEFAULT_PATTERN: &str = "default";

/// `SpeedStreet` when a collection has none (the value of every pattern in the install), mph.
const FALLBACK_SPEED_STREET_MPH: f32 = 35.0;
/// `SpeedHighway` when a collection has none, mph.
const FALLBACK_SPEED_HIGHWAY_MPH: f32 = 55.0;
/// `SpawnTime` when a collection has none, seconds.
const FALLBACK_SPAWN_TIME: f32 = 4.0;

/// A `Vehicles` record is 24 bytes: a 12-byte reference to a `pvehicle` collection (class key,
/// collection key, then a zero word), `Rate`, `MaxInstances` and `Percent`.
const RECORD_LEN: usize = 24;
const REFERENCE_KEY_OFFSET: usize = 4;
const RATE_OFFSET: usize = 12;
const MAX_INSTANCES_OFFSET: usize = 16;
const PERCENT_OFFSET: usize = 20;

/// One entry of a pattern's car list.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternVehicle {
    /// The `pvehicle` collection name (`trafha`, `semib`...), lower case.
    pub name: String,
    /// Density-weighted waiting time before this type may be chosen again.
    pub rate: f32,
    /// At most this many active cars of the type at once; 0 means no limit.
    pub max_instances: u32,
    /// The type may take at most this share of the traffic slots; 0 means no limit.
    pub percent: u32,
}

/// One `trafficpattern` collection.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficPattern {
    /// The collection name (`default`, `downtown`...), lower case.
    pub name: String,
    /// Cruising speed on roads with fewer than four traffic lanes, mph.
    pub speed_street_mph: f32,
    /// Cruising speed on roads with four or more traffic lanes, mph.
    pub speed_highway_mph: f32,
    /// Minimum time between two new vehicle instances, seconds.
    pub spawn_time: f32,
    pub vehicles: Vec<PatternVehicle>,
}

/// The string hash the zone data stores to name a pattern (`Data[0]` of a traffic-pattern zone).
pub fn pattern_hash(name: &str) -> u32 {
    blackbox_hash::bstring_hash(name)
}

/// Every `trafficpattern` collection of the database, in database order. Fields a collection does not
/// set come from `default` (the lookups follow the inheritance).
pub fn patterns(db: &Database) -> Vec<TrafficPattern> {
    let cars: Vec<(u32, String)> =
        db.collections_of("pvehicle").filter_map(|c| c.name().map(|n| (vlt_hash(n), n.to_ascii_lowercase()))).collect();
    db.collections_of("trafficpattern")
        .filter_map(|c| {
            let name = c.name()?.to_ascii_lowercase();
            let vehicles = match c.get("Vehicles") {
                Some(Value::Array(records)) => records.iter().filter_map(|r| vehicle(r, &cars)).collect(),
                _ => Vec::new(),
            };
            Some(TrafficPattern {
                name,
                speed_street_mph: c.get_f32("SpeedStreet").unwrap_or(FALLBACK_SPEED_STREET_MPH),
                speed_highway_mph: c.get_f32("SpeedHighway").unwrap_or(FALLBACK_SPEED_HIGHWAY_MPH),
                spawn_time: c.get_f32("SpawnTime").unwrap_or(FALLBACK_SPAWN_TIME),
                vehicles,
            })
        })
        .collect()
}

fn vehicle(record: &Value, cars: &[(u32, String)]) -> Option<PatternVehicle> {
    let Value::Raw { bytes, .. } = record else { return None };
    if bytes.len() < RECORD_LEN {
        return None;
    }
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let key = word(REFERENCE_KEY_OFFSET);
    let name = cars.iter().find(|(hash, _)| *hash == key)?.1.clone();
    Some(PatternVehicle {
        name,
        rate: f32::from_bits(word(RATE_OFFSET)),
        max_instances: word(MAX_INSTANCES_OFFSET),
        percent: word(PERCENT_OFFSET),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_hash_is_the_engine_string_hash() {
        assert_eq!(pattern_hash(""), 0xFFFF_FFFF);
        assert_eq!(pattern_hash("a"), 0xFFFF_FFFFu32.wrapping_mul(33).wrapping_add(u32::from(b'a')));
    }

    fn record(key: u32, rate: f32, max: u32, percent: u32) -> Value {
        let mut bytes = Vec::new();
        for word in [0x4A97_EC8F, key, 0, rate.to_bits(), max, percent] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        Value::Raw { type_key: 0, bytes }
    }

    #[test]
    fn a_record_names_its_car_by_hash() {
        let cars = vec![(vlt_hash("trafha"), "trafha".to_owned()), (vlt_hash("traftaxi"), "traftaxi".to_owned())];
        let v = vehicle(&record(vlt_hash("traftaxi"), 6.0, 1, 20), &cars).unwrap();
        assert_eq!(v, PatternVehicle { name: "traftaxi".into(), rate: 6.0, max_instances: 1, percent: 20 });
        assert!(vehicle(&record(vlt_hash("unknown"), 6.0, 1, 20), &cars).is_none());
    }

    #[test]
    fn a_short_record_is_skipped() {
        let short = Value::Raw { type_key: 0, bytes: vec![0; RECORD_LEN - 1] };
        assert!(vehicle(&short, &[]).is_none());
    }
}
