//! The petrol station's explosion effect, read from the install (`effects/exp_gasstation`, which names the
//! `fxex_gasstation` particle group of four emitters). Which explosions use it is the rewrite's rule
//! ([`is_petrol_station_sequence`]); the emitters themselves are the original's data.

use blackbox_attrib::{Database, vlt_hash};

use crate::car::exhaust::{GroupEmitter, group_emitters};

/// The `effects` record of a petrol station's explosion.
pub const PETROL_STATION_EFFECT: &str = "exp_gasstation";

/// The `effects` field that holds its particle group (`emittergroup`), by the field key read from the install.
const GROUP_FIELD: u32 = 0xABA8_6E60;

/// Event sequences of the petrol station's breakable pieces (`smackable` records `gaspumpexplosion`,
/// `gasstationroof` and `gasstationsupport`). An explosion ESpawned by one of them is a petrol-station blast.
const PETROL_STATION_SEQUENCES: [&str; 3] = ["gaspumpexplosion_seq", "gasstationroof_seq", "gasstationsupport_seq"];

/// Whether an explosion ESpawned by a scenery model with event sequence `key` is a petrol-station blast.
pub fn is_petrol_station_sequence(key: u32) -> bool {
    PETROL_STATION_SEQUENCES.iter().any(|name| vlt_hash(name) == key)
}

/// The emitters of the petrol station's explosion (empty when the record or its group is missing).
pub fn petrol_station_emitters(db: &Database) -> Vec<GroupEmitter> {
    let Some(effect) = db.collection("effects", PETROL_STATION_EFFECT) else {
        return Vec::new();
    };
    let Some(group) = effect.get_by_key(GROUP_FIELD).and_then(|v| v.as_ref_spec()).and_then(|r| db.resolve(r)) else {
        return Vec::new();
    };
    group_emitters(db, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petrol_station_sequence_hashes_match_the_install_keys() {
        // The `StringKey` hash32 values the install's smackable records carry for these sequences.
        assert_eq!(vlt_hash("gaspumpexplosion_seq"), 2_752_513_555);
        assert_eq!(vlt_hash("gasstationroof_seq"), 2_276_482_522);
        assert_eq!(vlt_hash("gasstationsupport_seq"), 503_242_438);
        assert!(is_petrol_station_sequence(vlt_hash("gasstationroof_seq")));
        assert!(!is_petrol_station_sequence(vlt_hash("metalexplosion_seq")));
    }
}
