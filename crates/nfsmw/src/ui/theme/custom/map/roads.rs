//! The roads on the overlay: the local set round the car, cached between frames, and the road layer built from it.
//!
//! **The cache rule.** The local set is the roads of the grid cells under a circle round the car, [`MARGIN`] times the
//! view radius. It is queried again only when the car has moved more than [`MOVE_SHARE`] of the view radius from where
//! the set was taken, the view radius has changed by more than [`ZOOM_SHARE`], or the network is another one. The
//! margin covers both thresholds, so no road in view is missing between two queries. Every frame only projects the
//! cached roads and builds their strokes.
//!
//! **The look.** Main roads are two units wide in the primary text tone, side roads one unit in the secondary tone,
//! paths (Expanded only) one unit in the rule tone. With high contrast every main road gets a dark under-stroke one
//! unit wider. Widths are in HUD units: the same on screen at every zoom.

use blackbox_ui::{Clip, LineJoin, Polyline, Polylines};

use super::view::MapView;
use crate::hud::{MapRoadsHandle, RoadClass};

/// The share of the view radius the car may move before the local set is queried again.
pub const MOVE_SHARE: f32 = 0.25;
/// The change of the view radius, as a share of it, that queries the local set again.
pub const ZOOM_SHARE: f32 = 0.10;
/// The local set's radius over the view radius: room for both thresholds, and a little more.
pub const MARGIN: f32 = 1.0 + MOVE_SHARE + ZOOM_SHARE + 0.05;

/// Road widths at scale 1, HUD units.
pub const MAIN_WIDTH: f32 = 2.0;
pub const SIDE_WIDTH: f32 = 1.0;
pub const PATH_WIDTH: f32 = 1.0;
/// How much wider the high-contrast under-stroke is than its road.
pub const UNDER_EXTRA: f32 = 1.0;
/// A mitre limit of 2 keeps a road's corners inside its stroke.
const JOIN: LineJoin = LineJoin::Mitre { limit: 2.0 };

/// The roads round the car, as the grid last gave them.
#[derive(Debug, Default)]
pub struct LocalSet {
    /// The network the set came from.
    source: Option<MapRoadsHandle>,
    /// Where the car was, and the view radius, when the set was taken.
    centre: [f32; 2],
    radius: f32,
    lines: Vec<u32>,
}

impl LocalSet {
    /// Takes the set again from `roads` for a car at `centre` and a view radius of `radius` metres, if the cache rule
    /// says so. Returns whether it did.
    pub fn refresh(&mut self, roads: &MapRoadsHandle, centre: [f32; 2], radius: f32) -> bool {
        if self.is_current(roads, centre, radius) {
            return false;
        }
        self.lines = roads.roads().near(centre, radius * MARGIN);
        self.source = Some(roads.clone());
        self.centre = centre;
        self.radius = radius;
        true
    }

    /// Whether the set still covers a car at `centre` with a view radius of `radius`.
    fn is_current(&self, roads: &MapRoadsHandle, centre: [f32; 2], radius: f32) -> bool {
        let Some(source) = &self.source else { return false };
        if !source.same(roads) || self.radius <= 0.0 {
            return false;
        }
        let moved = (centre[0] - self.centre[0]).hypot(centre[1] - self.centre[1]);
        if moved > MOVE_SHARE * self.radius {
            return false;
        }
        (radius / self.radius - 1.0).abs() <= ZOOM_SHARE
    }

    /// The ids of the roads in the set.
    pub fn lines(&self) -> &[u32] {
        &self.lines
    }
}

/// The colours (already faded) of each road class and of the high-contrast under-stroke, and the HUD scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadLook {
    pub main: [u8; 4],
    pub side: [u8; 4],
    pub path: [u8; 4],
    /// The under-stroke, with high contrast only.
    pub under: Option<[u8; 4]>,
    pub scale: f32,
}

/// Pushes the strokes of the local set's roads onto `out`: projected by `view`, clipped to `clip` with a soft edge
/// `band` wide, at depth `z` (the under-strokes just behind). A class whose colour is transparent is skipped.
pub fn road_lines(
    roads: &MapRoadsHandle,
    local: &LocalSet,
    view: &MapView,
    (clip, band): (Clip, f32),
    look: &RoadLook,
    z: f32,
    out: &mut Polylines,
) {
    let all = roads.roads().roads();
    for road in local.lines().iter().filter_map(|&id| all.get(usize::try_from(id).ok()?)) {
        let (colour, width) = match road.class {
            RoadClass::Main => (look.main, MAIN_WIDTH),
            RoadClass::Side => (look.side, SIDE_WIDTH),
            RoadClass::Path => (look.path, PATH_WIDTH),
        };
        if colour[3] == 0 {
            continue;
        }
        let points: Vec<_> = road.points.iter().map(|p| view.to_screen(*p)).collect();
        let stroke = |colour: [u8; 4], width: f32, z: f32| {
            Polyline::new(points.clone(), width * look.scale, colour).with_join(JOIN).with_clip(clip, band).with_z(z)
        };
        if let (RoadClass::Main, Some(under)) = (road.class, look.under) {
            out.push(stroke(under, width + UNDER_EXTRA, z + 0.5));
        }
        out.push(stroke(colour, width, z));
    }
}
