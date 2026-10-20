//! The game's tables for the chase radio: the voice layout, the battalions and their call sign pools, and the road
//! names the cops can report (the road's speech id to a region and a location). Facts about the original, read from its
//! speech interface and its road-name list; spec `docs/specs/ai-pursuit-speech.md` sections 3 and 4.

use blackbox_pursuit::speech::{Battalion, Layout, Pool};

use crate::audio::chase::feed::Vehicle;
use crate::audio::speech::params::values::{address_group_type, location_region, speaker_battalion};

pub const ROSEWOOD: Battalion = Battalion(speaker_battalion::ROSEWOOD as u16);
pub const COASTAL: Battalion = Battalion(speaker_battalion::COASTAL as u16);
pub const CITY: Battalion = Battalion(speaker_battalion::CITY as u16);
pub const ALPINE: Battalion = Battalion(speaker_battalion::ALPINE as u16);
pub const SUPER_PURSUIT: Battalion = Battalion(speaker_battalion::SUPER_PURSUIT as u16);
pub const RHINO: Battalion = Battalion(speaker_battalion::RHINO_UNITS as u16);
pub const AIR_SUPPORT: Battalion = Battalion(speaker_battalion::AIR_SUPPORT as u16);

/// The voices (speaker ids of the speech interface): dispatch 1, the helicopter 2, primaries 3 to 5, secondaries 6 to 8,
/// the cross car 9; and the call sign pools of the battalions (20 for the city, 10 for the others, 6 Rhino units, 5
/// super pursuit units). A road is "placeable" for a heading up to the Seaside interchange (id 80) and nameable up to
/// the Cascade Park id 86 (84 has no name).
pub fn layout() -> Layout {
    static POOLS: [Pool; 6] = [
        Pool { battalion: CITY, size: 20 },
        Pool { battalion: COASTAL, size: 10 },
        Pool { battalion: ROSEWOOD, size: 10 },
        Pool { battalion: ALPINE, size: 10 },
        Pool { battalion: RHINO, size: 6 },
        Pool { battalion: SUPER_PURSUIT, size: 5 },
    ];
    Layout {
        dispatch: 1,
        heli: 2,
        primary: &[3, 4, 5],
        secondary: &[6, 7, 8],
        cross: 9,
        heli_unit: (AIR_SUPPORT, 1),
        pools: &POOLS,
        random_battalions: &[ROSEWOOD, COASTAL, CITY, ALPINE],
        placeable_roads: (1, 80),
        locatable_roads: (1, 86),
    }
}

/// The battalion a car's model implies: the SUVs are Rhino units, the sports cars of the super pursuit are that.
pub fn vehicle_battalion(vehicle: Vehicle) -> Option<Battalion> {
    match vehicle {
        Vehicle::Patrol => None,
        Vehicle::Suv => Some(RHINO),
        Vehicle::SuperPursuit | Vehicle::Cross => Some(SUPER_PURSUIT),
    }
}

/// The vehicle class of a cop car's `pvehicle` name.
pub fn vehicle_of(name: &str) -> Vehicle {
    match name.to_ascii_lowercase().as_str() {
        "copsuv" | "copsuvl" | "copsuvpatrol" => Vehicle::Suv,
        "copsport" | "copsportghost" | "copsporthench" => Vehicle::SuperPursuit,
        "copcross" => Vehicle::Cross,
        _ => Vehicle::Patrol,
    }
}

/// A place the cops can name: the region and the nth location of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// A `location_region` value.
    pub region: u32,
    /// The location number, 1 to 31 (`location(n)` is its bit).
    pub location: u32,
}

/// The place of a road speech id, or `None` for a road with no name (id 0, 84, 87 and the off-road places).
pub fn place(road: u16) -> Option<Place> {
    let at = |region: u32, location: u32| Some(Place { region, location });
    match road {
        1..=30 => at(location_region::COLLEGE_TOWN, u32::from(road)),
        31 => at(location_region::COASTAL, 1),
        32 => at(location_region::COASTAL, 2),
        33..=61 => at(location_region::COASTAL, u32::from(road) - 30),
        62 => at(location_region::COASTAL_EXTRA, 3),
        63 => at(location_region::COASTAL_EXTRA, 1),
        64 => at(location_region::CITY, 1),
        65 => at(location_region::CITY, 3),
        66 => at(location_region::CITY, 4),
        67..=82 => at(location_region::CITY, u32::from(road) - 62),
        83 => at(location_region::COLLEGE_TOWN, 31),
        85 => at(location_region::COASTAL_EXTRA, 4),
        86 => at(location_region::CITY, 21),
        _ => None,
    }
}

/// The battalion of the area a road is in: what a cop on it calls himself.
pub fn road_battalion(road: u16) -> Option<Battalion> {
    match place(road)?.region {
        location_region::CITY => Some(CITY),
        location_region::COASTAL | location_region::COASTAL_EXTRA => Some(COASTAL),
        location_region::COLLEGE_TOWN => Some(ROSEWOOD),
        _ => None,
    }
}

/// The address group of a battalion's area, for dispatch's escalation (the Coastal and City swap is the original's: its
/// switch reads the battalion number as an address group number).
pub fn address_group_of_battalion(battalion: Battalion) -> u32 {
    match battalion {
        ROSEWOOD => address_group_type::COLLEGE_TOWN,
        COASTAL => address_group_type::COASTAL,
        CITY => address_group_type::CITY,
        ALPINE => address_group_type::ALPINE,
        _ => address_group_type::GENERIC_ANY,
    }
}

/// The address group of a place's region, for the 911 report.
pub fn address_group_of_region(region: u32) -> u32 {
    match region {
        location_region::COASTAL | location_region::COASTAL_EXTRA => address_group_type::COASTAL,
        location_region::CITY => address_group_type::CITY,
        location_region::COLLEGE_TOWN => address_group_type::COLLEGE_TOWN,
        _ => address_group_type::GENERIC_ANY,
    }
}
