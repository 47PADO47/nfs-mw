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

/// The damage scale of each zone (`DAMAGE_ZONES` order) every racing car of the original shares: the zone's
/// share of an impact that shows as damage. Front 0.8, rear 1.5, the sides and front corners 3.5, the rear corners
/// 7.5, roof 3.5, underbody 0.7.
const RACING_VISUAL_SCALE: [f32; 10] = [0.8, 1.5, 3.5, 3.5, 3.5, 3.5, 7.5, 7.5, 3.5, 0.7];
/// The impulse (kN s) that is one level of visible damage at scale 1.
const RACING_FORCE: f32 = 20.0;
/// Shock a car takes from a hit (`SHOCK_FORCE`) and how long it lasts.
const RACING_SHOCK: (f32, f32) = (40.0, 3.0);

impl CarEventSpec {
    /// The damage numbers of a car that has no vehicle record of its own, such as a mod car: the table every racing car
    /// of the original shares (it is one record, not one per car), written out here instead of read from any car.
    ///
    /// Visible damage depends on the impulse of a hit (`FORCE`), a scale per zone, and which zone was hit. The zone is
    /// decided from the contact point against the half dimensions of the car's own root box, so it fits the car:
    /// the front and rear zones start beyond half the half-length from the centre, the corners also need the contact
    /// beyond half the half-width and a normal across the car, the roof and underbody need the contact beyond half the
    /// half-height and a normal along it (`DamageHit::zone`). The car has no hit points, so it
    /// is never destroyed by damage, takes no event sequencer (its timed sequencer effects do not play, as for the
    /// racing cars of the original) and is not suppressed at a distance.
    pub fn racing_defaults() -> Self {
        Self {
            sequence: None,
            strength: RACING_FORCE * 1000.0,
            suppress_distance: 0.0,
            visual_scale: RACING_VISUAL_SCALE,
            hit_points: 0.0,
            hit_point_threshold: 0.0,
            hit_point_scale: [0.0; 10],
            shock_force: RACING_SHOCK.0,
            shock_time: RACING_SHOCK.1,
        }
    }

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
