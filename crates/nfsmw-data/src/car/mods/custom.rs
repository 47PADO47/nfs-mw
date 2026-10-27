//! A mod car's own customization (the `[customization]` table of the car manifest, docs/modding/car-customization.md).
//!
//! The garage catalogue of a car that carries this table comes from the table and the car's own files, never from a stock
//! car's parts: [`catalog::catalog`] lists the choices and [`apply::apply`] puts the selected ones on the assembled car.
//! A choice is told apart by a hash of the car's folder, the kind of part and the choice's name, so a saved car keeps its
//! choice when the mod reorders the lists.

use std::sync::{PoisonError, RwLock};

use blackbox_hash::bstring_hash;

pub mod apply;
pub mod catalog;
pub mod livery;
#[cfg(test)]
mod tests;

/// How a paint is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Gloss,
    Metallic,
    Chrome,
}

impl Finish {
    /// The shared light material that draws the finish (the game's `LightMaterials`, the paint's shading constants).
    pub fn light_material(self) -> &'static str {
        match self {
            Self::Gloss => "REGPAINTWHITE",
            Self::Metallic => "METPAINTSILVER",
            Self::Chrome => "CHROME",
        }
    }

    /// The finish as the garage's finish tabs name it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Gloss => "Gloss",
            Self::Metallic => "Metallic",
            Self::Chrome => "Chrome",
        }
    }

    /// The paint family the garage groups the finish under (the stock paint parts' prefix).
    pub fn family(self) -> &'static str {
        match self {
            Self::Gloss => "GLOSS",
            Self::Metallic => "METAL",
            Self::Chrome => "CHROME",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paint {
    pub name: String,
    pub color: [u8; 3],
    pub finish: Finish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RimPaint {
    pub name: String,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tint {
    pub name: String,
    pub color: [u8; 3],
    pub opacity: f32,
}

/// A model of the car's own `GEOMETRY.BIN`: `model` is the solid name without its level-of-detail letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vinyl {
    pub name: String,
    pub bitmap: String,
    pub mask: String,
    pub channels: u8,
}

/// A whole-body picture in the layout of the car's paint surfaces: `texture` is a texture of the car's own
/// `TEXTURES.BIN`. It is offered with the vinyls ([`livery`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Livery {
    pub name: String,
    pub texture: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VinylColor {
    pub name: String,
    pub color: [u8; 3],
}

/// What the shop offers for one mod car.
#[derive(Debug, Clone, PartialEq)]
pub struct Customization {
    pub paints: Vec<Paint>,
    pub default_paint: usize,
    pub rim_paints: Vec<RimPaint>,
    pub tints: Vec<Tint>,
    pub rims: Vec<Part>,
    pub spoilers: Vec<Part>,
    pub roof_scoops: Vec<Part>,
    pub vinyls: Vec<Vinyl>,
    /// The colours of the vinyl channels. Empty: [`DEFAULT_VINYL_COLORS`].
    pub vinyl_colors: Vec<VinylColor>,
    pub liveries: Vec<Livery>,
    /// The position in `liveries` of the one the car wears until the player picks a vinyl or a livery: a police car's
    /// markings. `None`: the plain paint.
    pub default_livery: Option<usize>,
}

/// The vinyl channel colours of a car that lists none of its own: name and sRGB colour.
pub const DEFAULT_VINYL_COLORS: [(&str, [u8; 3]); 12] = [
    ("White", [255, 255, 255]),
    ("Black", [16, 16, 16]),
    ("Silver", [170, 174, 180]),
    ("Red", [200, 24, 28]),
    ("Orange", [240, 120, 20]),
    ("Yellow", [245, 205, 30]),
    ("Green", [40, 160, 70]),
    ("Cyan", [30, 190, 215]),
    ("Blue", [30, 70, 200]),
    ("Purple", [120, 50, 170]),
    ("Pink", [235, 90, 150]),
    ("Gold", [200, 160, 60]),
];

/// The kinds of choice, which keep the hashes of two kinds apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Paint,
    RimPaint,
    Tint,
    Rim,
    Spoiler,
    RoofScoop,
    Vinyl,
    VinylColor,
    Livery,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Self::Paint => "PAINT",
            Self::RimPaint => "RIMPAINT",
            Self::Tint => "TINT",
            Self::Rim => "RIM",
            Self::Spoiler => "SPOILER",
            Self::RoofScoop => "ROOF",
            Self::Vinyl => "VINYL",
            Self::VinylColor => "VINYLCOLOR",
            Self::Livery => "LIVERY",
        }
    }
}

/// The hash that stands for a choice in the garage's catalogue and in a saved selection.
pub fn part_hash(folder: &str, kind: Kind, name: &str) -> u32 {
    bstring_hash(&format!("{}.{}.{}", folder.to_ascii_uppercase(), kind.tag(), name.to_ascii_uppercase()))
}

/// The entry of `items` that `hash` stands for.
pub fn pick<'a, T>(folder: &str, kind: Kind, items: &'a [T], name: impl Fn(&T) -> &str, hash: u32) -> Option<&'a T> {
    items.iter().find(|item| part_hash(folder, kind, name(item)) == hash)
}

/// The cars with a customization of their own, by folder. Each publication leaks the new list, as the mod cars do.
static PUBLISHED: RwLock<&'static [(String, Customization)]> = RwLock::new(&[]);

/// Register the customization of the mod car in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, customization: Customization) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, Customization)> = published.to_vec();
    all.push((folder.to_owned(), customization));
    *published = Box::leak(all.into_boxed_slice());
}

/// The customization of the mod car in `folder`. `None` for a stock car and for a mod car that gives none.
pub fn of(folder: &str) -> Option<&'static Customization> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, custom)| custom)
}

/// The shop tier a choice of a mod car is priced at: every such choice is a first-tier part. `None` when `hash` is not a
/// choice of any mod car.
pub fn price_level(hash: u32) -> Option<u32> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().any(|(folder, custom)| hashes(folder, custom).contains(&hash)).then_some(1)
}

/// Every choice hash of a car.
fn hashes(folder: &str, custom: &Customization) -> Vec<u32> {
    let mut all = Vec::new();
    all.extend(custom.paints.iter().map(|e| part_hash(folder, Kind::Paint, &e.name)));
    all.extend(custom.rim_paints.iter().map(|e| part_hash(folder, Kind::RimPaint, &e.name)));
    all.extend(custom.tints.iter().map(|e| part_hash(folder, Kind::Tint, &e.name)));
    all.extend(custom.rims.iter().map(|e| part_hash(folder, Kind::Rim, &e.name)));
    all.extend(custom.spoilers.iter().map(|e| part_hash(folder, Kind::Spoiler, &e.name)));
    all.extend(custom.roof_scoops.iter().map(|e| part_hash(folder, Kind::RoofScoop, &e.name)));
    all.extend(custom.vinyls.iter().map(|e| part_hash(folder, Kind::Vinyl, &e.name)));
    all.extend(custom.liveries.iter().map(|e| part_hash(folder, Kind::Livery, &e.name)));
    all
}

/// The colours a car offers for its vinyl channels: its own list, else [`DEFAULT_VINYL_COLORS`].
pub fn vinyl_palette(custom: &Customization) -> Vec<(String, [u8; 3])> {
    if custom.vinyl_colors.is_empty() {
        return DEFAULT_VINYL_COLORS.iter().map(|(name, rgb)| ((*name).to_owned(), *rgb)).collect();
    }
    custom.vinyl_colors.iter().map(|color| (color.name.clone(), color.color)).collect()
}
