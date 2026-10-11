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
    pub sequence: Sequence,
    /// FORCE converted from kN to N.
    pub strength: f32,
    pub suppress_distance: f32,
    pub visual_scale: [f32; 10],
}

impl CarEventSpec {
    pub fn read(vehicle: CollectionRef<'_>, sequences: &SequenceCatalog) -> Option<Self> {
        let key = vehicle.get_string_key("EventSequencer")?.hash32;
        let damage = vehicle.follow("damagespecs")?;
        let visual_scale = DAMAGE_ZONES.map(|zone| {
            let Some(Value::Raw { bytes, .. }) = damage.get(zone) else { return 0.0 };
            let Some(value) = bytes.get(..4) else { return 0.0 };
            f32::from_le_bytes(value.try_into().unwrap())
        });
        Some(Self {
            sequence: sequences.get(key)?.clone(),
            strength: damage.get_f32("FORCE").unwrap_or(0.0) * 1000.0,
            suppress_distance: damage.get_f32("SUPPRESS_DIST").unwrap_or(0.0),
            visual_scale,
        })
    }
}
