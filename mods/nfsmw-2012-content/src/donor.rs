//! The stock car a 2012 car clones, as far as the 2012 car needs it: the markers the game finds a car's parts by, and
//! the texture layout of the clone's paint surface ([`crate::liveries`]).
//!
//! The 2012 body is fitted onto the clone's hubs, so it stands where the clone's body stood. Everything the game
//! attaches to a body by a marker (the tail pipes the nitrous flames come out of, the mounts of aftermarket
//! spoilers and roof scoops, the cop lights) is at the clone's position on the new body, adjusted to its shape.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result};
use blackbox_hash::bstring_hash;
use blackbox_solid::{PositionMarker, Solid, read_solids_filtered};
use game_install::GameDir;
use nfsmw_data::read_unwrapped;

use crate::fit::StockCar;
use crate::liveries::{PaintMap, Triangle};

/// The levels of detail the mod writes, as the letters of the solid names.
pub const LOD_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];

/// The light material of the paint surface of a stock car (`car::CARSKIN`).
const CARSKIN: u32 = nfsmw_data::car::CARSKIN;

/// How far a marker may be moved to sit on the new body, in metres.
const MAX_SNAP: f32 = 0.35;
/// Half the side of the square, seen from above or behind, in which the body is searched for a marker's surface.
const SNAP_WINDOW: f32 = 0.2;

/// What a mod car takes from the stock car it clones. A stock car is two solids per level of detail: the body kit
/// (`<CAR>_KIT00_BODY`: bumpers, skirts, the pipes) and the base (`<CAR>_BASE`: the cabin shell, with the mounts of the
/// spoilers and roof scoops and the lamp positions). Both are paint surfaces; the mod's body replaces them.
pub struct Donor {
    /// The markers of the clone's body kit solid at each level of detail, by position in [`LOD_LETTERS`].
    body_markers: [Vec<PositionMarker>; 4],
    /// The markers of the clone's base solid, likewise.
    base_markers: [Vec<PositionMarker>; 4],
    pub paint: PaintMap,
}

impl Donor {
    /// A donor with nothing to give.
    pub fn empty() -> Self {
        Self { body_markers: Default::default(), base_markers: Default::default(), paint: PaintMap::new(Vec::new()) }
    }

    /// Read the body kit and the base of `clone` from the install. A clone without a body kit is an error; without a
    /// base it only has no base markers.
    pub fn read(game: &GameDir, clone: &StockCar) -> Result<Self> {
        let base = clone.base_model_name.to_ascii_uppercase();
        let name = |stem: &str, letter: char| format!("{base}_{stem}_{letter}");
        let wanted: Vec<u32> = LOD_LETTERS
            .iter()
            .flat_map(|&letter| [name("KIT00_BODY", letter), name("BASE", letter)])
            .map(|n| bstring_hash(&n))
            .collect();
        let path = format!("CARS/{}/GEOMETRY.BIN", clone.folder);
        let data = read_unwrapped(game, &path).with_context(|| format!("reading the body of {}", clone.type_name))?;
        let solids = read_solids_filtered(&data, |hash| wanted.contains(&hash))
            .with_context(|| format!("decoding the body of {}", clone.type_name))?;
        let find = |stem: &str, letter: char| {
            let hash = bstring_hash(&name(stem, letter));
            solids.iter().find(|solid| solid.name_hash == hash)
        };
        let finest_body = find("KIT00_BODY", LOD_LETTERS[0]).context("the clone has no finest body")?;
        let finest_base = find("BASE", LOD_LETTERS[0]);
        let markers = |stem: &str, finest: Option<&Solid>| -> [Vec<PositionMarker>; 4] {
            std::array::from_fn(|at| {
                let solid = find(stem, LOD_LETTERS[at]).or(finest);
                solid.map(|solid| solid.markers.clone()).unwrap_or_default()
            })
        };
        let mut triangles = paint_triangles(finest_body);
        triangles.extend(finest_base.map(paint_triangles).unwrap_or_default());
        Ok(Self {
            body_markers: markers("KIT00_BODY", Some(finest_body)),
            base_markers: markers("BASE", finest_base),
            paint: PaintMap::new(triangles),
        })
    }

    /// The markers of the body kit solid at level of detail `lod` (a position in [`LOD_LETTERS`]), moved onto `body`:
    /// the points of the new body's finest level, in car space.
    pub fn body_markers(&self, lod: usize, body: &[[f32; 3]]) -> Vec<PositionMarker> {
        snapped(&self.body_markers[lod.min(LOD_LETTERS.len() - 1)], body)
    }

    /// The markers of the base solid at level of detail `lod`, moved onto `body` likewise.
    pub fn base_markers(&self, lod: usize, body: &[[f32; 3]]) -> Vec<PositionMarker> {
        snapped(&self.base_markers[lod.min(LOD_LETTERS.len() - 1)], body)
    }
}

fn snapped(markers: &[PositionMarker], body: &[[f32; 3]]) -> Vec<PositionMarker> {
    let mut markers = markers.to_vec();
    for marker in &mut markers {
        snap(marker, body);
    }
    markers
}

/// The triangles of the solid's paint groups (the ones drawn with the `CARSKIN` light material).
fn paint_triangles(solid: &Solid) -> Vec<Triangle> {
    let mut triangles = Vec::new();
    for group in &solid.groups {
        let light = solid.light_material_hashes.get(usize::from(group.light_material));
        if light != Some(&CARSKIN) {
            continue;
        }
        let first = group.first_index as usize;
        let indices = solid.indices.get(first..first + group.num_indices as usize).unwrap_or_default();
        for corners in indices.as_chunks::<3>().0 {
            let vertex = |corner: u16| solid.vertices.get((group.base_vertex + u32::from(corner)) as usize);
            let [Some(a), Some(b), Some(c)] = [vertex(corners[0]), vertex(corners[1]), vertex(corners[2])] else {
                continue;
            };
            triangles.push(Triangle { corners: [a.position, b.position, c.position], uv: [a.uv, b.uv, c.uv] });
        }
    }
    triangles
}

/// How a marker sits on a body: along which axis it is moved, and whether to the lowest or the highest point of the body
/// around it.
struct Rule {
    names: &'static [&'static str],
    axis: usize,
    highest: bool,
}

/// Tail pipes sit on the rear face of the body (the lowest x), the mounts of spoilers and roof scoops on its upper face.
const RULES: [Rule; 2] = [
    Rule { names: &["EXHAUST", "LEFT_EXHAUST", "RIGHT_EXHAUST"], axis: 0, highest: false },
    Rule { names: &["SPOILER", "SPOILER2", "ROOF_SCOOP"], axis: 2, highest: true },
];

/// Move `marker` along its rule's axis to the surface of `body` near it, by at most [`MAX_SNAP`].
fn snap(marker: &mut PositionMarker, body: &[[f32; 3]]) {
    let Some(rule) = RULES.iter().find(|rule| rule.names.iter().any(|name| bstring_hash(name) == marker.name_hash))
    else {
        return;
    };
    let at = marker.translation();
    // The two other axes find the body around the marker; along the rule's axis only points near the marker count, so
    // a roof does not pull a rear-deck mount up and a bumper does not pull a pipe forwards.
    let across: Vec<usize> = (0..3).filter(|&axis| axis != rule.axis).collect();
    let near = body.iter().filter(|point| {
        let in_window = across.iter().all(|&axis| (point[axis] - at[axis]).abs() <= SNAP_WINDOW);
        in_window && (point[rule.axis] - at[rule.axis]).abs() <= MAX_SNAP
    });
    let surface = match rule.highest {
        true => near.map(|point| point[rule.axis]).reduce(f32::max),
        false => near.map(|point| point[rule.axis]).reduce(f32::min),
    };
    let Some(surface) = surface else { return };
    marker.matrix[12 + rule.axis] = surface;
}

/// The donors of a conversion run: each stock car is read once, however many 2012 cars clone it.
pub struct Donors<'a> {
    game: &'a GameDir,
    read: Mutex<HashMap<String, Arc<Donor>>>,
}

impl<'a> Donors<'a> {
    pub fn new(game: &'a GameDir) -> Self {
        Self { game, read: Mutex::new(HashMap::new()) }
    }

    /// The donor of `clone`, read the first time it is asked for. A clone whose body cannot be read gives an empty donor
    /// (no markers, no paint layout): the car is converted as it was before donors, and the log says why.
    pub fn of(&self, clone: &StockCar) -> Arc<Donor> {
        let mut read = self.read.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(donor) = read.get(&clone.type_name) {
            return Arc::clone(donor);
        }
        let donor = Donor::read(self.game, clone).unwrap_or_else(|error| {
            log::warn!("{}: its markers and paint layout are not used: {error:#}", clone.type_name);
            Donor::empty()
        });
        let donor = Arc::new(donor);
        read.insert(clone.type_name.clone(), Arc::clone(&donor));
        donor
    }
}

#[cfg(test)]
mod tests;
