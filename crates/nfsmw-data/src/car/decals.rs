//! Installed, fixed-position manufacturer decals. See docs/specs/garage-decals.md.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};
use blackbox_carparts::{CarTypeInfo, Part, PartQuery};
use blackbox_hash::{bstring_hash, bstring_hash_continue};
use blackbox_solid::Solid;
use blackbox_tpk::decode_rgba8;
use game_install::GameDir;

use super::{CarModel, CarTables, Slots, customization::Choice, slot};

mod variant;
pub use variant::DecalVariant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum DecalZone {
    FrontWindow,
    RearWindow,
    LeftDoor,
    RightDoor,
    LeftQuarter,
    RightQuarter,
}

impl DecalZone {
    pub const ALL: [Self; 6] =
        [Self::FrontWindow, Self::RearWindow, Self::LeftDoor, Self::RightDoor, Self::LeftQuarter, Self::RightQuarter];

    pub const fn label(self) -> &'static str {
        match self {
            Self::FrontWindow => "Windshield",
            Self::RearWindow => "Rear window",
            Self::LeftDoor => "Left door",
            Self::RightDoor => "Right door",
            Self::LeftQuarter => "Left quarter panel",
            Self::RightQuarter => "Right quarter panel",
        }
    }

    pub const fn model_slot(self) -> usize {
        70 + self as usize
    }

    pub const fn texture_slot(self, position: usize) -> usize {
        83 + self as usize * 8 + position
    }
}

#[derive(Clone, Debug, Default)]
pub struct DecalZoneCatalog {
    /// Maximum authored positions across the installed body's compatible kits.
    pub positions: usize,
    pub choices: Vec<Choice>,
    /// Native BRAND_NAME and NAME attributes, keyed by decal texture-part identity.
    pub variants: HashMap<u32, DecalVariant>,
    /// Native body part hash -> usable authored positions for that body kit.
    pub body_positions: HashMap<u32, usize>,
    pub number_positions: [bool; 2],
    pub body_number_positions: HashMap<u32, [bool; 2]>,
    /// Texture-part identities whose bitmap exists for this body's authored shape.
    pub body_choices: HashMap<u32, HashSet<u32>>,
}

impl DecalZoneCatalog {
    pub fn supports(&self, body: Option<u32>, part: u32) -> bool {
        body.and_then(|body| self.body_choices.get(&body)).is_none_or(|choices| choices.contains(&part))
    }
}

fn placeholder(zone: DecalZone, position: usize) -> u32 {
    if matches!(zone, DecalZone::LeftDoor | DecalZone::RightDoor) && position >= 6 {
        return bstring_hash(match position {
            6 => "DUMMY_NUMBER_LEFT",
            _ => "DUMMY_NUMBER_RIGHT",
        });
    }
    bstring_hash(&format!("DUMMY_DECAL{}", position + 1))
}

fn positions(solid: &Solid, zone: DecalZone) -> usize {
    // A gap cannot be exposed as an editable position because it has no authored surface.
    let limit = match zone {
        DecalZone::LeftDoor | DecalZone::RightDoor => 6,
        _ => 8,
    };
    (0..limit)
        .take_while(|position| {
            solid.groups.iter().any(|g| g.diffuse_texture(solid) == Some(placeholder(zone, *position)))
        })
        .count()
}

fn number_positions(solid: &Solid, zone: DecalZone) -> [bool; 2] {
    if !matches!(zone, DecalZone::LeftDoor | DecalZone::RightDoor) {
        return [false; 2];
    }
    std::array::from_fn(|i| solid.groups.iter().any(|g| g.diffuse_texture(solid) == Some(placeholder(zone, i + 6))))
}

pub(super) fn number_choices(t: &CarTables, available: &HashMap<u32, String>) -> [Vec<Choice>; 2] {
    ["NUMBER_LEFT", "NUMBER_RIGHT"].map(|brand| {
        t.parts
            .parts()
            .iter()
            .filter(|part| {
                part.part_id == 80
                    && t.parts.attribute(part, "BRAND_NAME") == Some(bstring_hash(brand))
                    && available_part(t, part, available)
            })
            .map(|part| Choice {
                hash: part.name_hash,
                label: t.parts.name(part).to_owned(),
                language_hash: t.parts.attribute(part, "LANGUAGEHASH"),
                carbon: false,
            })
            .collect()
    })
}

pub(super) fn available_textures(dir: &GameDir, t: &CarTables) -> Result<HashMap<u32, String>> {
    let wanted = t
        .parts
        .parts()
        .iter()
        .filter(|p| p.part_id == 80)
        .filter_map(|p| t.parts.attribute(p, "NAME"))
        .flat_map(|base| SHAPES.map(|shape| bstring_hash_continue(base, shape.as_bytes())))
        .collect();
    Ok(super::textures::load(dir, "", &wanted)?
        .into_iter()
        .filter(|(_, texture)| decode_rgba8(texture).is_some())
        .map(|(hash, texture)| (hash, texture.name))
        .collect())
}

const SHAPES: [&str; 3] = ["_SQUARE", "_RECT", "_WIDE"];

fn available_part(t: &CarTables, part: &Part, available: &HashMap<u32, String>) -> bool {
    t.parts.attribute(part, "NAME").is_some_and(|base| {
        SHAPES.iter().any(|suffix| available.contains_key(&bstring_hash_continue(base, suffix.as_bytes())))
    })
}

fn texture_hash(t: &CarTables, model: &Part, texture: &Part) -> Result<u32> {
    let shape = t.parts.attribute(model, "SHAPE").context("Decal model has no authored shape")?;
    let suffix =
        SHAPES.iter().find(|suffix| bstring_hash(&suffix[1..]) == shape).context("Decal model shape is unsupported")?;
    let base = t.parts.attribute(texture, "NAME").context("Decal has no texture identity")?;
    Ok(bstring_hash_continue(base, suffix.as_bytes()))
}

pub(super) fn catalog(
    t: &CarTables,
    car: &CarTypeInfo,
    solids: &HashMap<u32, Solid>,
    available: &HashMap<u32, String>,
) -> [DecalZoneCatalog; 6] {
    let stock = super::stock_parts(t, car);
    std::array::from_fn(|index| {
        let zone = DecalZone::ALL[index];
        let mut result = DecalZoneCatalog::default();
        let mut bodies: Vec<_> = t
            .parts
            .parts()
            .iter()
            .filter(|p| p.part_id == slot::BODY as u8 && t.parts.type_name_hash(p) == car.type_name_hash)
            .copied()
            .collect();
        if bodies.is_empty() {
            bodies.extend(stock[slot::BODY]);
        }
        for body in bodies {
            let mut slots = stock.clone();
            slots[slot::BODY] = Some(body);
            super::stock::fill_from_body(t, car, &mut slots);
            let solid = slots[zone.model_slot()]
                .and_then(|part| t.parts.model_hash(&part, 0))
                .and_then(|hash| solids.get(&hash));
            let count = solid.map_or(0, |solid| positions(solid, zone));
            let numbers = solid.map_or([false; 2], |solid| number_positions(solid, zone));
            result.positions = result.positions.max(count);
            result.body_positions.insert(body.name_hash, count);
            for (i, available) in numbers.into_iter().enumerate() {
                result.number_positions[i] |= available;
            }
            result.body_number_positions.insert(body.name_hash, numbers);
            let choices = slots[zone.model_slot()]
                .map(|model| {
                    t.parts
                        .parts()
                        .iter()
                        .filter(|part| part.part_id == 80)
                        .filter(|part| texture_hash(t, &model, part).is_ok_and(|hash| available.contains_key(&hash)))
                        .map(|part| part.name_hash)
                        .collect()
                })
                .unwrap_or_default();
            result.body_choices.insert(body.name_hash, choices);
        }
        if result.positions == 0 {
            return result;
        }
        let types: Vec<_> = t.slot_types.search_types(zone.texture_slot(0), car.type_name_hash).collect();
        result.choices = t
            .parts
            .parts()
            .iter()
            .filter(|part| {
                part.part_id == 80
                    && types.contains(&t.parts.type_name_hash(part))
                    && !["NUMBER_LEFT", "NUMBER_RIGHT"]
                        .into_iter()
                        .any(|brand| t.parts.attribute(part, "BRAND_NAME") == Some(bstring_hash(brand)))
                    && result.body_choices.values().any(|choices| choices.contains(&part.name_hash))
            })
            .map(|part| Choice {
                hash: part.name_hash,
                label: match t.parts.attribute(part, "BRAND_NAME").zip(t.parts.attribute(part, "NAME")) {
                    Some((brand, name)) if (DecalVariant { brand, name }).matches(false) => {
                        format!("{} · White", t.parts.name(part))
                    }
                    _ => t.parts.name(part).to_owned(),
                },
                language_hash: t.parts.attribute(part, "LANGUAGEHASH"),
                carbon: false,
            })
            .collect();
        result.variants = result
            .choices
            .iter()
            .filter_map(|choice| {
                let part = t.parts.by_name_hash(choice.hash)?;
                Some((
                    choice.hash,
                    DecalVariant {
                        brand: t.parts.attribute(part, "BRAND_NAME")?,
                        name: t.parts.attribute(part, "NAME")?,
                    },
                ))
            })
            .collect();
        result
    })
}

pub(super) fn resolve(
    t: &CarTables,
    car: &CarTypeInfo,
    selection: &super::customization::Selection,
    slots: &mut Slots,
) -> Result<()> {
    for zone in DecalZone::ALL {
        for (position, hash) in selection.decals[zone as usize].iter().enumerate() {
            let Some(hash) = hash else { continue };
            let slot = zone.texture_slot(position);
            slots[slot] = Some(
                t.slot_types
                    .search_types(slot, car.type_name_hash)
                    .find_map(|type_hash| t.parts.find(&PartQuery::new(80, type_hash).name_hash(*hash)).copied())
                    .with_context(|| format!("incompatible {} decal", zone.label()))?,
            );
        }
    }
    Ok(())
}

pub(super) fn apply(dir: &GameDir, folder: &str, t: &CarTables, slots: &Slots, model: &mut CarModel) -> Result<()> {
    let mut wanted = HashSet::new();
    for zone in DecalZone::ALL {
        for part in slots[zone.texture_slot(0)..zone.texture_slot(0) + 8].iter().flatten() {
            let model = slots[zone.model_slot()].context("Decal model is unavailable")?;
            wanted.insert(texture_hash(t, &model, part)?);
        }
    }
    if wanted.is_empty() {
        return Ok(());
    }
    let textures = super::textures::load(dir, folder, &wanted)?;
    for zone in DecalZone::ALL {
        let selected = &slots[zone.texture_slot(0)..zone.texture_slot(0) + 8];
        if selected.iter().all(Option::is_none) {
            continue;
        }
        let mut selected_positions = [false; 8];
        let mut found_model = false;
        for lod in 0..5 {
            let Some(hash) = slots[zone.model_slot()].and_then(|part| t.parts.model_hash(&part, lod)) else { continue };
            let Some(solid) = model.solids.get_mut(&hash) else { continue };
            found_model = true;
            for (position, part) in selected.iter().enumerate() {
                let Some(part) = part else { continue };
                let decal = slots[zone.model_slot()].context("Decal model is unavailable")?;
                let texture = texture_hash(t, &decal, part)?;
                ensure!(
                    textures.get(&texture).is_some_and(|t| decode_rgba8(t).is_some()),
                    "decal texture is unavailable"
                );
                for original in &mut solid.texture_hashes {
                    if *original == placeholder(zone, position) {
                        *original = texture;
                        selected_positions[position] = true;
                    }
                }
            }
        }
        ensure!(found_model, "{} decal model is unavailable for this body kit", zone.label());
        ensure!(
            selected.iter().enumerate().all(|(i, part)| part.is_none() || selected_positions[i]),
            "{} decal position is unavailable for this body kit",
            zone.label()
        );
    }
    model.textures.extend(textures);
    Ok(())
}

#[cfg(test)]
mod tests;
