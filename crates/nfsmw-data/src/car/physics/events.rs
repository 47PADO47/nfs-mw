//! Vehicle event links and damage-zone scaling from the gameplay database.

use blackbox_attrib::{CollectionRef, Value};

use crate::world::sequences::{Sequence, SequenceCatalog};

pub const DAMAGE_ZONES: [&str; 10] = [
    "DZ_FRONT",
    "DZ_REAR",
    "DZ_LEFT",
    "DZ_RIGHT",
    "DZ_LFRONT",
    "DZ_RFRONT",
    "DZ_LREAR",
    "DZ_RREAR",
    "DZ_TOP",
    "DZ_BOTTOM",
];

#[derive(Debug, Clone)]
pub struct CarEventSpec {
    /// The car's sequencer, when the car names one that the catalog has. Damage does not need it.
    pub sequence: Option<Sequence>,
    /// FORCE converted from kN to N.
    pub strength: f32,
    pub suppress_distance: f32,
    pub visual_scale: [f32; 10],
    pub hit_points: f32,
    pub hit_point_threshold: f32,
    pub hit_point_scale: [f32; 10],
    pub shock_force: f32,
    pub shock_time: f32,
}

impl CarEventSpec {
    pub fn read(vehicle: CollectionRef<'_>, sequences: &SequenceCatalog) -> Option<Self> {
        let key = vehicle.get_string_key("EventSequencer").map(|k| k.hash32);
        let damage = vehicle.follow("damagespecs")?;
        let visual_scale = DAMAGE_ZONES.map(|zone| {
            let Some(Value::Raw { bytes, .. }) = damage.get(zone) else { return 0.0 };
            let Some(value) = bytes.get(..4) else { return 0.0 };
            f32::from_le_bytes(value.try_into().unwrap())
        });
        let hit_point_scale = DAMAGE_ZONES.map(|zone| {
            let Some(Value::Raw { bytes, .. }) = damage.get(zone) else { return 0.0 };
            bytes.get(4..8).map_or(0.0, |value| f32::from_le_bytes(value.try_into().unwrap()))
        });
        Some(Self {
            sequence: key.and_then(|k| sequences.get(k)).cloned(),
            strength: damage.get_f32("FORCE").unwrap_or(0.0) * 1000.0,
            suppress_distance: damage.get_f32("SUPPRESS_DIST").unwrap_or(0.0),
            visual_scale,
            hit_points: damage.get_f32("HIT_POINTS").unwrap_or(0.0),
            hit_point_threshold: damage.get_f32("HP_THRESHOLD").unwrap_or(0.0),
            hit_point_scale,
            shock_force: damage.get_f32("SHOCK_FORCE").unwrap_or(0.0),
            shock_time: damage.get_f32("SHOCK_TIME").unwrap_or(0.0),
        })
    }
}
