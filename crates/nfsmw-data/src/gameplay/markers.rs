//! The activity zones: the auto shops (`menu_gate_customshop/zone_customshop_N`), the car lots
//! (`menu_gate_carlot/zone_carlot_N`) and the safe houses (`menu_gate_safehouse/zone_safehouse_N`). Each is a sphere trigger of class `zonetrigger` with a `ZoneType`, and
//! a child `respawn` marker. Spec: `docs/formats/gameplay-races.md` section 7.

use blackbox_attrib::{CollectionRef, Database};
use glam::Vec3;

use super::{Placement, collection, inherits, instances, name_of, placement};

/// Field `0x6CE23062` (`Int32`), set on each zone; its meaning is unconfirmed (see [`ActivityMarker::icon_index`]).
const ICON_INDEX_FIELD: u32 = 0x6CE2_3062;
/// Field `0x2241F4CD` (`GCollectionKey`), set on each zone to its `respawn` marker (the zone's first child).
const RESPAWN_FIELD: u32 = 0x2241_F4CD;

/// The kinds of activity zone this module reads, by `ZoneType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    /// `customshop`: the auto shop (paint, parts and performance; 8 zones).
    AutoShop,
    /// `carlot`: the car lot (buy a car; 3 zones).
    CarLot,
    /// `safehouse`: the safe house (career hub; 4 zones).
    SafeHouse,
}

impl ActivityKind {
    /// The kind of a `ZoneType` string. `None` for the zone types this module does not read (empty, other).
    pub fn from_zone_type(zone_type: &str) -> Option<Self> {
        Some(match zone_type {
            "customshop" => Self::AutoShop,
            "carlot" => Self::CarLot,
            "safehouse" => Self::SafeHouse,
            _ => return None,
        })
    }
}

/// One auto shop, car lot or safe house.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityMarker {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `menu_gate_customshop/zone_customshop_1`.
    pub name: String,
    pub kind: ActivityKind,
    /// The zone's centre (`Position`) in physics space.
    pub position: Vec3,
    /// `Radius` in metres: the zone is a sphere around `position`.
    pub radius: f32,
    /// The `respawn` marker the game places the car at after this zone (its `Position` and `Rotation`).
    pub respawn: Option<Placement>,
    /// The `Int32` in field `0x6CE23062`: 8 to 16 across the zones. Its meaning is **unconfirmed**. It is read so
    /// callers can see it, not used as a map icon yet (the map icon of a safe house is `SAFEHOUSEINDICATOR_%d` in
    /// `speed.exe`; the index that goes with it is not established).
    pub icon_index: Option<i32>,
}

/// Every auto shop, car lot and safe house zone, in database order.
pub(crate) fn markers(db: &Database) -> Vec<ActivityMarker> {
    instances(db).filter(|c| inherits(*c, "zonetrigger")).filter_map(|c| marker(db, c)).collect()
}

fn marker(db: &Database, c: CollectionRef<'_>) -> Option<ActivityMarker> {
    let kind = ActivityKind::from_zone_type(c.get_str("ZoneType")?)?;
    Some(ActivityMarker {
        key: c.key(),
        name: name_of(c),
        kind,
        position: placement(c).position,
        radius: c.get_f32("Radius").unwrap_or(0.0),
        respawn: key_field_at(c, RESPAWN_FIELD).and_then(|k| collection(db, k)).map(placement),
        icon_index: c.get_by_key(ICON_INDEX_FIELD).and_then(|v| v.as_i32()),
    })
}

/// A `GCollectionKey` at a field key the database has no name for.
fn key_field_at(c: CollectionRef<'_>, field_key: u32) -> Option<u32> {
    c.get_by_key(field_key).and_then(super::key_of).filter(|k| *k != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_types_select_the_two_activities() {
        assert_eq!(ActivityKind::from_zone_type("customshop"), Some(ActivityKind::AutoShop));
        assert_eq!(ActivityKind::from_zone_type("safehouse"), Some(ActivityKind::SafeHouse));
        assert_eq!(ActivityKind::from_zone_type("carlot"), Some(ActivityKind::CarLot));
        assert_eq!(ActivityKind::from_zone_type(""), None);
    }
}
