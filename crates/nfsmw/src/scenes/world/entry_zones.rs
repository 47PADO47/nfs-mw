//! The entry zones: the places that offer a prompt when the car drives into them. These are the auto shops and safe
//! houses (their stored spheres) and the race starts (a sphere of [`RACE_RADIUS`] at the start grid). The box triggers
//! of the gameplay data are race checkpoints and finish lines, not entries, so none is a zone here. Spec:
//! `docs/specs/world-markers.md`.

use glam::Vec3;
use nfsmw_data::gameplay::{ActivityKind, GameplayData};

use super::space;
use crate::modding::content::interior::ModInterior;

/// The radius of a race's entry sphere, metres. The data gives no size for a race start. The interior zones are 6 or
/// 7 m, so this uses 6 m **[unconfirmed]**.
pub const RACE_RADIUS: f32 = 6.0;

/// What a zone is, as the prompt names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    AutoShop,
    SafeHouse,
    Race,
}

impl ZoneKind {
    /// The name of the place, lower case, for prompts and the log.
    pub fn label(self) -> &'static str {
        match self {
            Self::AutoShop => "auto shop",
            Self::SafeHouse => "safe house",
            Self::Race => "race",
        }
    }
}

/// One entry zone: a sphere around `centre`, metres. The centre is in render space, the space the car and the effects
/// use (`space.rs`: x forward, y left, z up, the map's own coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub kind: ZoneKind,
    /// The collection name, for the log.
    pub name: String,
    pub centre: Vec3,
    pub radius: f32,
}

/// Every entry zone of the world, in data order, and the mods' interiors (their own zones, tested after the stock ones).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryZones {
    pub(super) zones: Vec<Zone>,
    /// The mods' interiors: each has an entry zone in render space (`ModInterior::entry`).
    pub(super) interiors: Vec<ModInterior>,
}

impl EntryZones {
    /// The zones of the loaded gameplay data: the auto shops and safe houses, then the race starts.
    pub fn from_data(data: &GameplayData) -> Self {
        // Car lots are not entered yet: their zones are not entry zones (the stub is for shops and safe houses).
        let interiors = data.markers.iter().filter_map(|m| {
            let kind = match m.kind {
                ActivityKind::AutoShop => ZoneKind::AutoShop,
                ActivityKind::SafeHouse => ZoneKind::SafeHouse,
                ActivityKind::CarLot => return None,
            };
            Some(Zone { kind, name: m.name.clone(), centre: render(m.position), radius: m.radius })
        });
        let races = data.races.iter().filter_map(|race| race.start.map(|start| (race, start))).map(|(race, start)| {
            Zone { kind: ZoneKind::Race, name: race.name.clone(), centre: render(start.position), radius: RACE_RADIUS }
        });
        Self { zones: interiors.chain(races).collect(), interiors: Vec::new() }
    }

    /// The same zones with the mods' interiors added.
    pub fn with_interiors(mut self, interiors: Vec<ModInterior>) -> Self {
        self.interiors = interiors;
        self
    }

    /// The mod interior whose entry zone holds `point`, if any: the first in load order.
    pub fn entered_interior(&self, point: Vec3) -> Option<usize> {
        self.interiors.iter().position(|interior| interior.entry.contains(point.to_array()))
    }

    /// The zone the point is inside, if any: the one whose centre is nearest when zones overlap.
    pub fn entered(&self, point: Vec3) -> Option<&Zone> {
        self.zones
            .iter()
            .filter(|zone| sphere_contains(zone.centre, zone.radius, point))
            .min_by(|a, b| a.centre.distance_squared(point).total_cmp(&b.centre.distance_squared(point)))
    }

    /// The zones whose centre lies within `range` metres of `point`: the ones worth drawing.
    pub fn near(&self, point: Vec3, range: f32) -> impl Iterator<Item = &Zone> {
        self.zones.iter().filter(move |zone| zone.centre.distance_squared(point) <= range * range)
    }
}

/// A gameplay position (physics space, as `nfsmw_data::gameplay` stores it) in render space.
fn render(physics: Vec3) -> Vec3 {
    space::to_render(physics.to_array())
}

/// Whether `point` is within `radius` of `centre`. A zone with no radius holds nothing.
pub fn sphere_contains(centre: Vec3, radius: f32, point: Vec3) -> bool {
    if radius <= 0.0 {
        return false;
    }
    point.distance_squared(centre) <= radius * radius
}
