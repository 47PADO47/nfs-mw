//! The car tables (`GLOBAL/GLOBALB.LZC`) and gameplay records (`GLOBAL/ATTRIBUTES.BIN`)
//! that car assembly reads, loaded once.

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use blackbox_carparts::layout::{CarDataLayout, MOST_WANTED};
use blackbox_carparts::{CarTypeInfo, PartsDb, PresetRide, SlotTypes};
use game_install::GameDir;

use crate::read_unwrapped;

/// NFS: Most Wanted's car-table layout.
pub const LAYOUT: &CarDataLayout = &MOST_WANTED;

/// Slot numbers car assembly refers to (`CAR_SLOT_ID`, docs/formats/cardata.md "Slots").
pub mod slot {
    use std::ops::RangeInclusive;

    pub const BASE: usize = 0;
    pub const DAMAGE_PANELS: RangeInclusive<usize> = 1..=22;
    pub const BODY: usize = 23;
    pub const FRONT_BRAKE: usize = 24;
    pub const REAR_BRAKE: usize = 34;
    pub const DRIVER: usize = 43;
    pub const SPOILER: usize = 44;
    pub const UNIVERSAL_SPOILER_BASE: usize = 45;
    pub const DAMAGE0: RangeInclusive<usize> = 46..=51;
    pub const ROOF: usize = 62;
    pub const FRONT_WHEEL: usize = 66;
    pub const REAR_WHEEL: usize = 67;
    pub const LICENSE_PLATE: usize = 69;
    pub const DECAL_MODELS: RangeInclusive<usize> = 70..=75;
    /// Door and quarter decal models, filled from the body kit.
    pub const KIT_DECALS: RangeInclusive<usize> = 72..=75;
    /// Slots 0..MODELS name solids.
    pub const MODELS: usize = 76;
    pub const BASE_PAINT: usize = 76;
    pub const VINYL_LAYER: usize = 77;
    pub const VINYL_COLOURS: RangeInclusive<usize> = 79..=82;
    pub const DECAL_TEXTURES: RangeInclusive<usize> = 83..=130;
    pub const HUD_COLOURS: RangeInclusive<usize> = 133..=135;
}

pub struct CarTables {
    pub types: Vec<CarTypeInfo>,
    pub parts: PartsDb,
    pub slot_types: SlotTypes,
    pub presets: Vec<PresetRide>,
    /// Gameplay database (class `ecar` places the wheels).
    pub attributes: Database,
}

impl CarTables {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let globalb = read_unwrapped(dir, "GLOBAL/GLOBALB.LZC")?;
        let attributes = dir.read("GLOBAL/ATTRIBUTES.BIN")?;
        Ok(Self {
            types: blackbox_carparts::read_car_types(&globalb, LAYOUT),
            parts: blackbox_carparts::read_parts_db(&globalb, LAYOUT).context("reading the parts database")?,
            slot_types: blackbox_carparts::read_slot_types(&globalb, LAYOUT),
            presets: blackbox_carparts::read_preset_rides(&globalb, LAYOUT),
            attributes: Database::open(&attributes).context("reading GLOBAL/ATTRIBUTES.BIN")?,
        })
    }

    /// The car type whose geometry lives in `CARS/<folder>/`, or whose type name is `folder`.
    pub fn car_type_for_folder(&self, folder: &str) -> Option<&CarTypeInfo> {
        let in_folder = |t: &&CarTypeInfo| {
            let path = t.geometry_filename.replace('\\', "/");
            path.split('/').nth(1).is_some_and(|f| f.eq_ignore_ascii_case(folder))
        };
        self.types
            .iter()
            .find(|t| t.type_name.eq_ignore_ascii_case(folder))
            .or_else(|| self.types.iter().find(in_folder))
    }

    pub fn preset(&self, name: &str) -> Option<&PresetRide> {
        self.presets.iter().find(|p| p.preset_name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_numbers_match_the_layout() {
        for (name, at) in [
            ("BASE", slot::BASE),
            ("BODY", slot::BODY),
            ("FRONT_BRAKE", slot::FRONT_BRAKE),
            ("REAR_BRAKE", slot::REAR_BRAKE),
            ("DRIVER", slot::DRIVER),
            ("SPOILER", slot::SPOILER),
            ("UNIVERSAL_SPOILER_BASE", slot::UNIVERSAL_SPOILER_BASE),
            ("DAMAGE0_FRONT", *slot::DAMAGE0.start()),
            ("ROOF", slot::ROOF),
            ("FRONT_WHEEL", slot::FRONT_WHEEL),
            ("REAR_WHEEL", slot::REAR_WHEEL),
            ("LICENSE_PLATE", slot::LICENSE_PLATE),
            ("DECAL_FRONT_WINDOW", *slot::DECAL_MODELS.start()),
            ("DECAL_LEFT_DOOR", *slot::KIT_DECALS.start()),
            ("BASE_PAINT", slot::BASE_PAINT),
            ("VINYL_LAYER0", slot::VINYL_LAYER),
            ("VINYL_COLOUR0_0", *slot::VINYL_COLOURS.start()),
            ("DECAL_FRONT_WINDOW_TEX0", *slot::DECAL_TEXTURES.start()),
            ("HUD_BACKING_COLOUR", *slot::HUD_COLOURS.start()),
        ] {
            assert_eq!(LAYOUT.slot(name), Some(at), "{name}");
        }
    }
}
