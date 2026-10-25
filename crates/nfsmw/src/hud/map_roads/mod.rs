//! The roads the Custom HUD's map draws (`ui/theme/custom/map`): the game's road network as simplified 2D polylines in
//! render space (x forward, y left, metres, as `MapPosition` uses), each with a class, and a uniform grid that finds the
//! roads near a point. Spec: `docs/specs/custom-hud-map-impl.md`.
//!
//! The roads are built once per loaded network, on the scene side where the network lives (`scenes/world/open.rs`),
//! and handed to the HUD in `HudState::map_roads` as a [`MapRoadsHandle`]: a shared pointer, so a frame costs a
//! reference count, never a copy of the roads.
//!
//! - `simplify`: the curve sampling and the Douglas-Peucker thinning.
//! - `classify`: main road, side road or path, from the lanes and the segment flags.
//! - `grid`: the uniform grid and its circle query.
//! - `race_route`: one lap of the race's route, built once per race.

mod classify;
mod grid;
mod race_route;
#[cfg(test)]
mod race_route_tests;
mod simplify;
#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::Arc;

use blackbox_roads::{RoadNetwork, centre_line};
use glam::Vec3;

pub use classify::RoadClass;
use grid::RoadGrid;
pub use race_route::{RaceRoute, RaceRouteHandle};

/// How far a simplified road may stray from the network's curve, metres: the curve is sampled until each chord is
/// this close to it, then thinned at the same tolerance.
pub const TOLERANCE_M: f32 = 1.5;

/// One drawable road: a road segment of the network as a polyline.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRoad {
    pub class: RoadClass,
    /// Render space (x, y), metres; at least two points.
    pub points: Vec<[f32; 2]>,
}

/// Every drawable road of a network and the grid that finds them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapRoads {
    roads: Vec<MapRoad>,
    grid: RoadGrid,
}

impl MapRoads {
    /// The roads of `net`, its points carried to render space by `to_render` (physics space in, render (x, y) out).
    /// A turning connector inside a junction is left out (see `classify`).
    pub fn build(net: &RoadNetwork, to_render: impl Fn(Vec3) -> [f32; 2]) -> Self {
        let segments = (0..net.segments.len()).filter_map(|i| u16::try_from(i).ok());
        let roads = segments.filter_map(|segment| road(net, segment, &to_render)).collect();
        Self::from_roads(roads)
    }

    /// The roads given, with their grid. A road of fewer than two points is dropped.
    pub fn from_roads(roads: Vec<MapRoad>) -> Self {
        let roads: Vec<MapRoad> = roads.into_iter().filter(|road| road.points.len() >= 2).collect();
        let grid = RoadGrid::new(&roads);
        Self { roads, grid }
    }

    /// Every road; a road's id is its index here.
    pub fn roads(&self) -> &[MapRoad] {
        &self.roads
    }

    /// The ids of the roads in the grid cells a circle of `radius` metres round `centre` touches, sorted, each once.
    /// The cells are coarse, so a road just outside the circle may be in the list; none inside it is missing.
    pub fn near(&self, centre: [f32; 2], radius: f32) -> Vec<u32> {
        self.grid.query(centre, radius)
    }
}

/// The drawable road of segment `segment`, or `None` for a connector the map leaves out.
fn road(net: &RoadNetwork, segment: u16, to_render: &impl Fn(Vec3) -> [f32; 2]) -> Option<MapRoad> {
    let class = classify::class_of(net, segment)?;
    let curve = centre_line(net, segment, 1);
    let points = simplify::polyline(&curve, net.segment(segment).is_curved(), TOLERANCE_M, to_render);
    (points.len() >= 2).then_some(MapRoad { class, points })
}

/// The roads of the loaded network, shared between the scene and the HUD. Two handles are equal when they share the
/// same roads, so comparing two HUD states never walks the roads.
#[derive(Clone)]
pub struct MapRoadsHandle(Arc<MapRoads>);

impl MapRoadsHandle {
    pub fn new(roads: MapRoads) -> Self {
        Self(Arc::new(roads))
    }

    pub fn roads(&self) -> &MapRoads {
        &self.0
    }

    /// Whether both handles share one set of roads (one network load).
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for MapRoadsHandle {
    fn eq(&self, other: &Self) -> bool {
        self.same(other)
    }
}

impl fmt::Debug for MapRoadsHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapRoadsHandle").field("roads", &self.0.roads.len()).finish()
    }
}
