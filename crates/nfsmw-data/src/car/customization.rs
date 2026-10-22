//! Compatible installed model parts. Assembly rules: docs/specs/car-assembly.md.

use std::collections::HashMap;

use anyhow::{Context, Result, ensure};
use blackbox_carparts::{CarTypeInfo, PartQuery};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use game_install::GameDir;

use super::{CarTables, LAYOUT, Placement, Slots, WheelSetup, slot, stock_parts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Category {
    Body,
    Spoiler,
    Hood,
    Roof,
    Wheels,
    Paint,
    WindowTint,
    RimPaint,
    Vinyl,
}

impl Category {
    pub const MODELS: [Self; 5] = [Self::Body, Self::Spoiler, Self::Hood, Self::Roof, Self::Wheels];
    pub const ALL: [Self; 9] = [
        Self::Body,
        Self::Spoiler,
        Self::Hood,
        Self::Roof,
        Self::Wheels,
        Self::Paint,
        Self::WindowTint,
        Self::RimPaint,
        Self::Vinyl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Body => "Body kit",
            Self::Spoiler => "Spoiler",
            Self::Hood => "Hood",
            Self::Roof => "Roof scoop",
            Self::Wheels => "Rims",
            Self::Paint => "Paint finish / preset",
            Self::WindowTint => "Window tint",
            Self::RimPaint => "Rim paint preset",
            Self::Vinyl => "Fixed vinyl",
        }
    }

    pub fn slot(self) -> usize {
        match self {
            Self::Body => slot::BODY,
            Self::Spoiler => slot::SPOILER,
            Self::Hood => slot::HOOD,
            Self::Roof => slot::ROOF,
            Self::Wheels => slot::FRONT_WHEEL,
            Self::Paint => slot::BASE_PAINT,
            Self::WindowTint => slot::WINDOW_TINT,
            Self::RimPaint => 78,
            Self::Vinyl => slot::VINYL_LAYER,
        }
    }
}

/// None means the installed stock part, including pieces built into the body solid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub parts: [Option<u32>; 9],
    pub rim_color: Option<[u8; 3]>,
    pub vinyl_colors: [Option<[u8; 3]>; 3],
}

impl Selection {
    pub fn is_stock(&self) -> bool {
        self.parts.iter().all(Option::is_none)
            && self.rim_color.is_none()
            && self.vinyl_colors.iter().all(Option::is_none)
    }
}

#[derive(Clone, Debug)]
pub struct Choice {
    pub hash: u32,
    pub label: String,
    pub language_hash: Option<u32>,
    pub carbon: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub choices: [Vec<Choice>; 9],
    pub stock: [Option<u32>; 9],
    pub colors: HashMap<u32, [u8; 3]>,
    pub finishes: Vec<(String, u32)>,
    pub gauge_skins: Vec<u8>,
    pub vinyl_remaps: HashMap<u32, usize>,
    pub paintable_wheels: std::collections::HashSet<u32>,
}

/// Shared geometry identities and tables are read once, then reused for each selected car.
pub struct Inventory {
    tables: CarTables,
    shared: HashMap<u32, String>,
}

impl Inventory {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let tables = CarTables::load(dir)?;
        let mut shared = HashMap::new();
        for folder in dir.subdirectories("CARS").into_iter().filter(|f| super::is_shared_part_folder(f)) {
            if let Ok(pack) = super::read_solids(dir, &folder) {
                shared.extend(pack.into_iter().map(|s| (s.name_hash, s.name)));
            }
        }
        Ok(Self { tables, shared })
    }

    pub fn catalog(&self, dir: &GameDir, folder: &str) -> Result<Catalog> {
        let car = self.tables.car_type_for_folder(folder).context("car type unavailable")?;
        let solids: HashMap<_, _> = super::read_solids(dir, folder)?.into_iter().map(|s| (s.name_hash, s)).collect();
        let mut catalog = Catalog::from_geometry(&self.tables, car, &solids, &self.shared);
        let vinyls = super::vinyl::available(dir, folder, &self.tables, car)?;
        catalog.choices[Category::Vinyl as usize].retain(|p| vinyls.contains(&p.hash));
        catalog.gauge_skins =
            (0..=10).filter(|skin| dir.exists(&format!("GLOBAL/HUDS_Custom_{skin:02}.bin"))).collect();
        Ok(catalog)
    }
}

impl Catalog {
    /// Read geometry once per selected car; don't offer missing solids or attachment markers.
    pub fn load(dir: &GameDir, folder: &str) -> Result<Self> {
        Inventory::load(dir)?.catalog(dir, folder)
    }

    fn from_geometry(
        t: &CarTables,
        car: &CarTypeInfo,
        solids: &HashMap<u32, Solid>,
        shared: &HashMap<u32, String>,
    ) -> Self {
        let slots = stock_parts(t, car);
        let stock = Category::ALL.map(|c| slots[c.slot()].map(|p| p.name_hash));
        let base = slots[slot::BASE].and_then(|p| t.parts.model_hash(&p, 0)).and_then(|h| solids.get(&h));
        let wheels = WheelSetup::read(&t.attributes, &car.base_model_name).is_some();
        let mut catalog = Self { stock, ..Self::default() };
        for category in Category::ALL {
            let slot = category.slot();
            let types: Vec<_> = t.slot_types.search_types(slot, car.type_name_hash).collect();
            for part in t.parts.parts() {
                if part.part_id != LAYOUT.slots[slot].part_id
                    || !types.contains(&t.parts.type_name_hash(part))
                    || Some(part.name_hash) == stock[category as usize]
                    || (Category::MODELS.contains(&category)
                        && t.parts
                            .model_hash(part, 0)
                            .is_none_or(|h| !solids.contains_key(&h) && !shared.contains_key(&h)))
                    || (matches!(category, Category::Paint | Category::Vinyl) && !car.skinnable)
                    || (matches!(category, Category::Paint | Category::WindowTint | Category::RimPaint)
                        && t.parts
                            .attribute(part, "LIGHT_MATERIAL_NAME")
                            .is_none_or(|hash| !t.light_materials.iter().any(|m| m.name_hash == hash)))
                    || (category == Category::Wheels && !wheels)
                {
                    continue;
                }
                let marker = match (category, part.upgrade_level) {
                    (Category::Spoiler, 1..) if t.parts.attribute(part, "USEMARKER2").is_some_and(|v| v != 0) => {
                        Some("SPOILER2")
                    }
                    (Category::Spoiler, 1..) => Some("SPOILER"),
                    (Category::Roof, 1..) => Some("ROOF_SCOOP"),
                    _ => None,
                };
                if marker.is_some_and(|m| base.is_none_or(|b| b.marker(bstring_hash(m)).is_none())) {
                    continue;
                }
                let solid_name = t
                    .parts
                    .model_hash(part, 0)
                    .and_then(|h| {
                        solids.get(&h).map(|s| s.name.as_str()).or_else(|| shared.get(&h).map(String::as_str))
                    })
                    .unwrap_or("");
                let label = match category {
                    Category::Body => format!("Body kit {}", super::stock::kit_number(t, part)),
                    Category::Wheels => rim_label(solid_name),
                    Category::Paint | Category::RimPaint => {
                        let paint = super::Paint::of(t, part);
                        catalog.colors.insert(part.name_hash, paint.rgb);
                        let family = paint.name.split('_').next().unwrap_or("Paint");
                        let finish = match family {
                            "GLOSS" => "Gloss",
                            "METAL" => "Metallic",
                            "PEARL" => "Pearlescent",
                            _ => family,
                        };
                        format!("{finish} · {}", paint.hex())
                    }
                    _ if t.parts.name(part).trim().is_empty() => solid_name.replace('_', " "),
                    _ => t.parts.name(part).replace('_', " "),
                };
                if category == Category::Vinyl {
                    catalog
                        .vinyl_remaps
                        .insert(part.name_hash, t.parts.attribute(part, "NUMREMAPCOLOURS").unwrap_or(0) as usize);
                }
                if category == Category::Paint {
                    let name = t.parts.name(part);
                    for (prefix, label) in [
                        ("GLOSS", "Gloss"),
                        ("METAL", "Metallic"),
                        ("PEARL", "Pearl"),
                        ("CHROME", "Chrome"),
                        ("MATTE", "Matte"),
                    ] {
                        if name.starts_with(prefix) && !catalog.finishes.iter().any(|(n, _)| n == label) {
                            catalog.finishes.push((label.to_owned(), part.name_hash));
                        }
                    }
                }
                if category == Category::Wheels && part.upgrade_level > 0 {
                    catalog.paintable_wheels.insert(part.name_hash);
                }
                catalog.choices[category as usize].push(Choice {
                    hash: part.name_hash,
                    label,
                    language_hash: (!matches!(category, Category::Paint | Category::RimPaint))
                        .then(|| t.parts.attribute(part, "LANGUAGEHASH"))
                        .flatten(),
                    carbon: t.parts.attribute(part, "CARBONFIBRE").is_some_and(|v| v != 0),
                });
            }
        }
        catalog
    }

    pub fn validate(&self, selection: &Selection) -> Result<()> {
        if selection.rim_color.is_some() || selection.parts[Category::RimPaint as usize].is_some() {
            ensure!(
                selection.parts[Category::Wheels as usize].is_some_and(|h| self.paintable_wheels.contains(&h)),
                "fit aftermarket rims before selecting rim paint"
            );
        }
        for category in Category::ALL {
            if let Some(hash) = selection.parts[category as usize] {
                ensure!(
                    self.choices[category as usize].iter().any(|p| p.hash == hash),
                    "{} is not available for this car",
                    category.label()
                );
            }
        }
        Ok(())
    }

    /// Original aerodynamics gate compares body/spoiler identities with their stock parts.
    pub fn aero_changed(&self, selection: &Selection) -> bool {
        [Category::Body, Category::Spoiler]
            .into_iter()
            .any(|c| selection.parts[c as usize].or(self.stock[c as usize]) != self.stock[c as usize])
    }
}

fn rim_label(solid: &str) -> String {
    let name = solid.strip_suffix("_A").unwrap_or(solid);
    let Some((brand, style)) = name.split_once("_STYLE") else { return name.replace('_', " ") };
    let mut fields = style.split('_');
    let style = fields.next().and_then(|s| s.parse::<u32>().ok());
    let inches = fields.next().and_then(|s| s.parse::<u32>().ok());
    let (Some(style), Some(inches)) = (style, inches) else { return name.replace('_', " ") };
    format!("{brand} · Style {style} · {inches} in")
}

pub(super) fn resolve(t: &CarTables, car: &CarTypeInfo, selection: &Selection) -> Result<Slots> {
    let mut slots = stock_parts(t, car);
    for category in Category::ALL {
        let Some(hash) = selection.parts[category as usize] else { continue };
        let s = category.slot();
        let part = t
            .slot_types
            .search_types(s, car.type_name_hash)
            .find_map(|type_hash| {
                t.parts.find(&PartQuery::new(LAYOUT.slots[s].part_id, type_hash).name_hash(hash)).copied()
            })
            .with_context(|| format!("incompatible {} part", category.label()))?;
        slots[s] = Some(part);
    }
    super::stock::fill_from_body(t, car, &mut slots);
    Ok(slots)
}

pub(super) fn validate_placements(selection: &Selection, placements: &[Placement]) -> Result<()> {
    for category in Category::MODELS {
        if selection.parts[category as usize].is_none() {
            continue;
        }
        let count = placements
            .iter()
            .filter(|p| p.slot == category.slot() || (category == Category::Wheels && p.slot == slot::REAR_WHEEL))
            .count();
        let required = match category {
            Category::Wheels => 4,
            _ => 1,
        };
        ensure!(count == required, "{} model or attachment is missing at this LOD", category.label());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
