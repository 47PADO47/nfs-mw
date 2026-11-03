//! The entry zones: the places that offer a prompt when the car drives into them. These are the auto shops and safe
//! houses (their stored spheres) and authored race engage triggers. A start-grid sphere is retained only for
//! data sets without engage markers. Checkpoint and finish-line box triggers are not entries. Spec:
//! `docs/specs/world-markers.md`.

use glam::Vec3;
use nfsmw_data::gameplay::{ActivityKind, GameplayData};

use super::space;
use crate::modding::content::interior::ModInterior;

/// Fallback race-entry radius for data sets without authored engage markers, in metres.
/// This remake fallback is not an original-game radius.
pub const RACE_RADIUS: f32 = 6.0;

/// What a zone is, as the prompt names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    AutoShop,
    SafeHouse,
    CarLot,
    Race,
}

impl ZoneKind {
    /// The name of the place, lower case, for prompts and the log.
    pub fn label(self) -> &'static str {
        match self {
            Self::AutoShop => "auto shop",
            Self::SafeHouse => "safe house",
            Self::CarLot => "car lot",
            Self::Race => "race",
        }
    }
}

/// One entry zone: a sphere around `centre`, metres. The centre is in render space, the space the car and the effects
/// use (`space.rs`: x forward, y left, z up, the map's own coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub kind: ZoneKind,
    /// Installed event icon category; place zones have no race category.
    pub race_type: Option<nfsmw_data::gameplay::RaceType>,
    /// The collection name, for the log.
    pub name: String,
    pub centre: Vec3,
    pub radius: f32,
    /// Where the car is put after the zone (its `respawn` marker), in render space; races have none.
    pub respawn: Option<Vec3>,
}

/// Every entry zone of the world, in data order, and the mods' interiors (their own zones, tested after the stock ones).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryZones {
    pub(super) rivals: std::collections::HashSet<String>,
    /// Positive BinIndex thresholds for authored place icons.
    pub(super) unlock_bins: std::collections::HashMap<String, i32>,
    pub(super) zones: Vec<Zone>,
    /// The mods' interiors: each has an entry zone in render space (`ModInterior::entry`).
    pub(super) interiors: Vec<ModInterior>,
    /// The career's start point (`career_start_<region>`), in render space: where a world starts when no zone was visited.
    pub career_start: Option<Vec3>,
}

impl EntryZones {
    /// The zones of the loaded gameplay data: places, then race engage triggers.
    pub fn from_data(data: &GameplayData) -> Self {
        let interiors = data.markers.iter().map(|m| {
            let kind = match m.kind {
                ActivityKind::AutoShop => ZoneKind::AutoShop,
                ActivityKind::SafeHouse => ZoneKind::SafeHouse,
                ActivityKind::CarLot => ZoneKind::CarLot,
            };
            Zone {
                kind,
                race_type: None,
                name: m.name.clone(),
                centre: render(m.position),
                radius: m.radius,
                respawn: m.respawn.map(|placement| render(placement.position)),
            }
        });
        let races = data.races.iter().filter_map(|race| {
            let engage = data.engages.iter().find(|entry| entry.target == race.key);
            let (position, radius) = match engage {
                Some(entry) => (entry.position, entry.radius),
                None if data.engages.is_empty() => (race.start?.position, RACE_RADIUS),
                None => return None,
            };
            Some(Zone {
                kind: ZoneKind::Race,
                race_type: race.race_type,
                name: race.name.clone(),
                centre: render(position),
                radius,
                respawn: None,
            })
        });
        Self {
            rivals: data.races.iter().filter(|race| race.story.boss_race).map(|race| race.name.clone()).collect(),
            unlock_bins: data
                .markers
                .iter()
                .filter_map(|marker| marker.icon_index.map(|bin| (marker.name.clone(), bin)))
                .collect(),
            zones: interiors.chain(races).collect(),
            interiors: Vec::new(),
            career_start: data.career_start.map(|placement| render(placement.position)),
        }
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
