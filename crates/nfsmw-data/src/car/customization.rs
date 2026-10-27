//! Compatible installed model parts. Assembly rules: docs/specs/car-assembly.md.

use std::collections::HashMap;

use anyhow::{Context, Result, ensure};
use blackbox_carparts::{CarTypeInfo, PartQuery};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use game_install::GameDir;

pub use super::decals::{DecalVariant, DecalZone, DecalZoneCatalog};
use super::{CarTables, LAYOUT, Placement, Slots, WheelSetup, slot, stock_parts};

mod names;
mod palettes;
pub use palettes::ColorChoice;

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
    /// Explicit installed color-part identities; RGB-only developer values leave these unset.
    pub vinyl_color_parts: [Option<u32>; 3],
    /// Installed decal part identities, in native zone/position order (slots 83–130).
    pub decals: [[Option<u32>; 8]; 6],
    /// The text the player typed on the car's licence plate (`[customization.plate]` of a mod car); `None` keeps the car's own
    /// text. Not a part: it has no price and the original game's saves cannot hold it, so it is kept beside the save.
    pub plate: Option<String>,
}

impl Selection {
    pub fn is_stock(&self) -> bool {
        self.parts.iter().all(Option::is_none)
            && self.rim_color.is_none()
            && self.vinyl_colors.iter().all(Option::is_none)
            && self.vinyl_color_parts.iter().all(Option::is_none)
            && self.decals.iter().flatten().all(Option::is_none)
            && self.plate.is_none()
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
    /// Native vinyl channels 0..2, then gauge backing, needle and character palettes.
    pub color_palettes: [Vec<ColorChoice>; 6],
    pub finishes: Vec<(String, u32)>,
    /// Installed material family, retained for the garage's finish tabs.
    pub paint_families: HashMap<u32, String>,
    /// Installed BRAND_NAME and INNER_RADIUS attributes, keyed by wheel part.
    pub rim_metadata: HashMap<u32, (u32, u32)>,
    /// Original low-five-bit customization group, including mod-defined groups.
    pub part_groups: HashMap<u32, u8>,
    pub gauge_skins: Vec<u8>,
    /// Native paint finish and spoken-colour language labels for cart descriptions.
    pub paint_names: HashMap<u32, (u32, u32)>,
    /// Native custom-HUD part names, indexed by HUDINDEX.
    pub gauge_names: HashMap<u8, Choice>,
    /// Original CarPart name and optional language label, including stock pieces.
    pub native_names: HashMap<u32, (String, Option<u32>)>,
    pub vinyl_remaps: HashMap<u32, usize>,
    pub paintable_wheels: std::collections::HashSet<u32>,
    pub decals: [DecalZoneCatalog; 6],
    pub number_choices: [Vec<Choice>; 2],
    /// A mod car whose rim paint works on the wheels it ships with, with or without aftermarket rims.
    pub custom_rim_paint: bool,
}

/// Shared geometry identities and tables are read once, then reused for each selected car.
pub struct Inventory {
    tables: CarTables,
    shared: HashMap<u32, String>,
    decal_textures: HashMap<u32, String>,
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
        let decal_textures = super::decals::available_textures(dir, &tables)?;
        Ok(Self { tables, shared, decal_textures })
    }

    /// Whether the car tables this inventory was read with have the car in `folder`. A mod car published after the inventory
    /// was loaded is not in it, and the inventory is read again.
    pub fn knows(&self, folder: &str) -> bool {
        self.tables.car_type_for_folder(folder).is_some()
    }

    pub fn catalog(&self, dir: &GameDir, folder: &str) -> Result<Catalog> {
        if let Some(custom) = super::mods::custom::of(folder) {
            // A mod car that carries its own customization is offered that, and nothing of a stock car's parts.
            let mut catalog = super::mods::custom::catalog::catalog(dir, folder, custom)?;
            catalog.gauge_skins =
                (0..=10).filter(|skin| dir.exists(&format!("GLOBAL/HUDS_Custom_{skin:02}.bin"))).collect();
            catalog.number_choices = super::decals::number_choices(&self.tables, &self.decal_textures);
            names::read(&self.tables, &mut catalog);
            return Ok(catalog);
        }
        let car = self.tables.car_type_for_folder(folder).context("car type unavailable")?;
        let solids: HashMap<_, _> = super::read_solids(dir, folder)?.into_iter().map(|s| (s.name_hash, s)).collect();
        let mut catalog = Catalog::from_geometry(&self.tables, car, &solids, &self.shared);
        let vinyls = super::vinyl::available(dir, folder, &self.tables, car)?;
        catalog.choices[Category::Vinyl as usize].retain(|p| vinyls.contains(&p.hash));
        catalog.gauge_skins =
            (0..=10).filter(|skin| dir.exists(&format!("GLOBAL/HUDS_Custom_{skin:02}.bin"))).collect();
        catalog.decals = super::decals::catalog(&self.tables, car, &solids, &self.decal_textures);
        catalog.number_choices = super::decals::number_choices(&self.tables, &self.decal_textures);
        names::read(&self.tables, &mut catalog);
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
        let wheels = WheelSetup::for_car(&t.attributes, car).is_some();
        let mut catalog = Self { stock, color_palettes: palettes::read(t, car), ..Self::default() };
        for category in [Category::Paint, Category::RimPaint] {
            if let Some(part) = stock[category as usize].and_then(|hash| t.parts.by_name_hash(hash)) {
                let paint = super::Paint::of(t, part);
                catalog.colors.insert(part.name_hash, paint.rgb);
                catalog
                    .paint_families
                    .insert(part.name_hash, paint.name.split('_').next().unwrap_or("Paint").to_owned());
            }
        }
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
                        catalog.paint_families.insert(part.name_hash, family.to_owned());
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
                if category == Category::Wheels {
                    catalog.rim_metadata.insert(
                        part.name_hash,
                        (
                            t.parts.attribute(part, "BRAND_NAME").unwrap_or(0),
                            t.parts.attribute(part, "INNER_RADIUS").unwrap_or(0),
                        ),
                    );
                }
                catalog.choices[category as usize].push(Choice {
                    hash: part.name_hash,
                    label,
                    language_hash: (!matches!(category, Category::Paint | Category::RimPaint))
                        .then(|| t.parts.attribute(part, "LANGUAGEHASH"))
                        .flatten(),
                    carbon: t.parts.attribute(part, "CARBONFIBRE").is_some_and(|v| v != 0),
                });
                catalog.part_groups.insert(part.name_hash, part.group);
            }
        }
        catalog
    }

    pub fn validate(&self, selection: &Selection) -> Result<()> {
        for (index, hash) in selection.vinyl_color_parts.into_iter().enumerate() {
            self.validate_palette_color(index, hash, selection.vinyl_colors[index])?;
        }
        if selection.rim_color.is_some() || selection.parts[Category::RimPaint as usize].is_some() {
            ensure!(
                self.custom_rim_paint
                    || selection.parts[Category::Wheels as usize].is_some_and(|h| self.paintable_wheels.contains(&h)),
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
        for zone in DecalZone::ALL {
            let options = &self.decals[zone as usize];
            let body = selection.parts[Category::Body as usize].or(self.stock[Category::Body as usize]);
            let positions =
                body.and_then(|hash| options.body_positions.get(&hash)).copied().unwrap_or(options.positions);
            for (position, hash) in selection.decals[zone as usize].iter().enumerate() {
                let Some(hash) = hash else { continue };
                ensure!(options.supports(body, *hash), "{} bitmap is unavailable for this body kit", zone.label());
                if matches!(zone, DecalZone::LeftDoor | DecalZone::RightDoor) && position >= 6 {
                    let number = position - 6;
                    let supported = body
                        .and_then(|hash| options.body_number_positions.get(&hash))
                        .copied()
                        .unwrap_or(options.number_positions);
                    ensure!(supported[number], "{} number position is unavailable", zone.label());
                    ensure!(self.number_choices[number].iter().any(|p| p.hash == *hash), "Door number is unavailable");
                    continue;
                }
                ensure!(position < positions, "{} decal position is unavailable", zone.label());
                ensure!(options.choices.iter().any(|p| p.hash == *hash), "{} decal is unavailable", zone.label());
            }
        }
        Ok(())
    }

    pub fn validate_palette_color(&self, index: usize, hash: Option<u32>, rgb: Option<[u8; 3]>) -> Result<()> {
        let Some(hash) = hash else { return Ok(()) };
        let color = self
            .color_palettes
            .get(index)
            .and_then(|palette| palette.iter().find(|color| color.hash == hash))
            .context("Native color part is unavailable for this car and channel")?;
        ensure!(rgb == Some(color.rgb), "Native color part does not match its RGB value");
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
    super::decals::resolve(t, car, selection, &mut slots)?;
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
