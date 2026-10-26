//! The car tables (`GLOBAL/GLOBALB.LZC`) and gameplay records (`GLOBAL/ATTRIBUTES.BIN`)
//! that car assembly reads, loaded once.

use std::ops::Deref;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use blackbox_carparts::layout::{CarDataLayout, MOST_WANTED};
use blackbox_carparts::{CarTypeInfo, LightMaterial, PartsDb, PresetRide, SlotTypes};
use game_install::GameDir;

use crate::{memo, shared};

/// NFS: Most Wanted's car-table layout.
pub const LAYOUT: &CarDataLayout = &MOST_WANTED;

/// Slot numbers car assembly refers to (`CAR_SLOT_ID`, docs/formats/cardata.md "Slots").
pub mod slot {
    use std::ops::RangeInclusive;

    pub const BASE: usize = 0;
    pub const DAMAGE_PANELS: RangeInclusive<usize> = 1..=22;
    pub const DAMAGE_BODY: usize = 2;
    pub const BODY: usize = 23;
    pub const FRONT_BRAKE: usize = 24;
    pub const REAR_BRAKE: usize = 34;
    pub const DRIVER: usize = 43;
    pub const SPOILER: usize = 44;
    pub const UNIVERSAL_SPOILER_BASE: usize = 45;
    pub const DAMAGE0: RangeInclusive<usize> = 46..=51;
    pub const DAMAGE0_FRONT: usize = 46;
    pub const DAMAGE0_FRONT_LEFT: usize = 47;
    pub const DAMAGE0_FRONT_RIGHT: usize = 48;
    pub const DAMAGE0_REAR: usize = 49;
    pub const DAMAGE0_REAR_LEFT: usize = 50;
    pub const DAMAGE0_REAR_RIGHT: usize = 51;
    pub const ROOF: usize = 62;
    pub const HOOD: usize = 63;
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
    pub const WINDOW_TINT: usize = 131;
    pub const HUD_COLOURS: RangeInclusive<usize> = 133..=135;
}

/// The car tables that the mod cars do not change, read and parsed once per session (see [`CarTables::load`]).
pub struct StockTables {
    /// The stock car types, in table order.
    pub stock_types: Vec<CarTypeInfo>,
    pub parts: PartsDb,
    pub slot_types: SlotTypes,
    pub presets: Vec<PresetRide>,
    /// The shading constants of the car materials (`LightMaterials`).
    pub light_materials: Vec<LightMaterial>,
    /// Gameplay database (class `ecar` places the wheels), the one `shared::attributes` holds.
    pub attributes: Arc<Database>,
}

impl StockTables {
    fn read(dir: &GameDir) -> Result<Self> {
        let globalb = shared::globalb(dir)?;
        Ok(Self {
            stock_types: blackbox_carparts::read_car_types(&globalb, LAYOUT),
            parts: blackbox_carparts::read_parts_db(&globalb, LAYOUT).context("reading the parts database")?,
            slot_types: blackbox_carparts::read_slot_types(&globalb, LAYOUT),
            presets: blackbox_carparts::read_preset_rides(&globalb, LAYOUT),
            light_materials: blackbox_carparts::read_light_materials(&globalb, LAYOUT),
            attributes: shared::attributes(dir)?,
        })
    }
}

/// The stock tables with the parts of the mod cars added (see [`CarTables::load`]), for the stock tables and the number
/// of published mod cars they were built for.
struct Extended {
    stock: Arc<StockTables>,
    cars: usize,
    tables: Arc<StockTables>,
}

static EXTENDED: Mutex<Option<Extended>> = Mutex::new(None);

impl StockTables {
    /// These tables with the parts of the mod cars `cars` added to the parts database.
    fn with_mod_parts(&self, cars: &[blackbox_carparts::CarTypeInfo]) -> Self {
        let mut parts = self.parts.clone();
        super::mods::parts::extend(&mut parts, cars);
        Self {
            stock_types: self.stock_types.clone(),
            parts,
            slot_types: self.slot_types.clone(),
            presets: self.presets.clone(),
            light_materials: self.light_materials.clone(),
            attributes: Arc::clone(&self.attributes),
        }
    }
}

/// The tables of `stock` for the mod cars `cars`: `stock` itself when there are none, else the extended tables, built
/// once for this `stock` and this many cars and kept until a car is published or the stock tables are read again.
fn tables_for(stock: Arc<StockTables>, cars: &[blackbox_carparts::CarTypeInfo]) -> Arc<StockTables> {
    if cars.is_empty() {
        return stock;
    }
    let mut cached = EXTENDED.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = cached.as_ref().filter(|known| Arc::ptr_eq(&known.stock, &stock) && known.cars == cars.len()) {
        return Arc::clone(&known.tables);
    }
    let tables = Arc::new(stock.with_mod_parts(cars));
    *cached = Some(Extended { stock, cars: cars.len(), tables: Arc::clone(&tables) });
    tables
}

/// The car tables of one install: the stock tables (reached through [`Deref`]) and the car types, mod cars included.
pub struct CarTables {
    /// The stock car types first, so a stock folder always finds its own type; the mod cars follow (car/mods.rs).
    pub types: Vec<CarTypeInfo>,
    stock: Arc<StockTables>,
}

impl Deref for CarTables {
    type Target = StockTables;

    fn deref(&self) -> &StockTables {
        &self.stock
    }
}

impl CarTables {
    /// The car tables of `dir`. The stock tables are read once per session, keyed on the two files they come from.
    /// The car types are assembled on every call: the mod cars are sealed once at start-up, and a load made before
    /// that must not fix the list for the rest of the session. With mod cars published, the parts database also holds
    /// their parts (`mods/parts.rs`); those tables are built when the first load sees a new set of cars and shared after.
    pub fn load(dir: &GameDir) -> Result<Self> {
        let stock = memo(dir, &[shared::GLOBALB_FILE, shared::ATTRIBUTES_FILE], || StockTables::read(dir))?;
        let mod_types = super::mods::car_types();
        let mut types = stock.stock_types.clone();
        types.extend(mod_types.iter().cloned());
        Ok(Self { types, stock: tables_for(stock, &mod_types) })
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
    use blackbox_carparts::CarTypeInfo;
    use blackbox_hash::bstring_hash;

    use super::*;

    fn empty_stock() -> Arc<StockTables> {
        Arc::new(StockTables {
            stock_types: Vec::new(),
            parts: PartsDb::default(),
            slot_types: SlotTypes::default(),
            presets: Vec::new(),
            light_materials: Vec::new(),
            attributes: Arc::new(Database::default()),
        })
    }

    fn mod_car(folder: &str) -> CarTypeInfo {
        CarTypeInfo {
            type_name: folder.to_owned(),
            base_model_name: folder.to_owned(),
            geometry_filename: format!(r"CARS\{folder}\GEOMETRY.BIN"),
            manufacturer: String::new(),
            type_name_hash: bstring_hash(folder),
            index: 0,
            usage_type: 0,
            default_skin_number: 1,
            skinnable: true,
            default_base_paint: 0,
        }
    }

    #[test]
    fn a_mod_car_finds_its_own_body_and_wheel_in_the_extended_tables() {
        let stock = empty_stock();
        let cars = [mod_car("MODCAR"), mod_car("OTHERCAR")];
        let tables = CarTables { types: cars.to_vec(), stock: tables_for(Arc::clone(&stock), &cars) };
        let slots = super::super::stock_parts(&tables, &cars[0]);
        let body = slots[slot::BODY].expect("the car's own body part");
        let wheel = slots[slot::FRONT_WHEEL].expect("the car's own wheel part");
        assert_eq!(tables.parts.model_hash(&body, 0), Some(bstring_hash("MODCAR_KIT00_BODY_A")));
        assert_eq!(tables.parts.model_hash(&body, 3), Some(bstring_hash("MODCAR_KIT00_BODY_D")));
        assert_eq!(tables.parts.model_hash(&body, 4), None, "the car has no level E");
        assert_eq!(tables.parts.model_hash(&wheel, 1), Some(bstring_hash("MODCAR_KIT00_FRONT_TIRE_B")));
        // Nothing else is filled, and the other car's parts are its own.
        let filled: Vec<usize> = (0..slots.len()).filter(|&s| slots[s].is_some()).collect();
        assert_eq!(filled, [slot::BODY, slot::FRONT_WHEEL]);
        let other = super::super::stock_parts(&tables, &cars[1]);
        assert_eq!(
            tables.parts.model_hash(&other[slot::BODY].unwrap(), 0),
            Some(bstring_hash("OTHERCAR_KIT00_BODY_A"))
        );
        assert!(stock.parts.parts().is_empty(), "the shared stock tables are not changed");
    }

    #[test]
    fn the_extended_tables_are_built_once_per_set_of_cars() {
        let stock = empty_stock();
        assert!(Arc::ptr_eq(&tables_for(Arc::clone(&stock), &[]), &stock), "no mod cars: the stock tables themselves");
        let one = [mod_car("ONECAR")];
        let first = tables_for(Arc::clone(&stock), &one);
        let again = tables_for(Arc::clone(&stock), &one);
        assert!(Arc::ptr_eq(&first, &again));
        let two = [mod_car("ONECAR"), mod_car("TWOCAR")];
        let grown = tables_for(Arc::clone(&stock), &two);
        assert!(!Arc::ptr_eq(&first, &grown));
        assert_eq!(grown.parts.parts().len(), 4);
        assert_eq!(first.parts.parts().len(), 2);
    }

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
