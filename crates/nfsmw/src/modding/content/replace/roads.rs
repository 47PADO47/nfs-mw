//! The carriageways of the world's roads in plan, for keeping a replaced tree off them (`[fit] settle`, `docs/modding/api-replace.md`,
//! "Settling").
//!
//! The road network (`blackbox_roads::RoadNetwork`, the lanes AI cars drive on) knows every road's centre line, its height and its
//! cross section. [`Roads`] turns it into short pieces of centre line in the 2005 solid space (x forward, y left, z up: the road network's
//! physics space is x right, y up, z forward, so a point `(px, py, pz)` is at `(pz, -px)` with height `py`), each with the half width of
//! the **carriageway**: the outer edge of the zones a vehicle drives on ([`CARRIAGEWAY`]; sidewalks, medians and barriers are not). The
//! pieces sit in a plan grid; [`Roads::nearest`] answers how far a point is from the edge of the nearest carriageway at about its height.
//! The set is built once when a world opens and shared with the loader threads ([`set_roads`]).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use blackbox_roads::{RoadNetwork, centre_line, travel_profile, zone};

/// The zone kinds that are carriageway: lanes of traffic, shoulders, parking, the plain road kinds and the centre strip.
pub const CARRIAGEWAY: &[u8] =
    &[zone::TRAFFIC, zone::SHOULDER, zone::PARKING, zone::ROAD, zone::CENTER, zone::DRIVABLE];
/// How far apart the points of a centre line are, metres.
const SPACING: f32 = 4.0;
/// The side of a cell of the plan grid, metres.
const CELL: f32 = 32.0;

/// One piece of a centre line.
#[derive(Debug, Clone, Copy)]
struct Piece {
    a: [f32; 2],
    b: [f32; 2],
    /// The heights at the ends, and the half widths of the carriageway there.
    level: [f32; 2],
    half: [f32; 2],
}

/// A centre line to make [`Roads`] of: its points (plan, height, half width of the carriageway).
#[derive(Debug, Clone, Default)]
pub struct Line {
    pub points: Vec<([f32; 2], f32, f32)>,
}

/// The nearest carriageway to a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Near {
    /// Metres from the point to the edge of the carriageway; negative inside it.
    pub gap: f32,
    /// The height of the road there.
    pub level: f32,
    /// The unit direction from the road's centre line towards the point (to the left of the line when the point is on it).
    pub away: [f32; 2],
}

/// The carriageways of a world, in plan.
#[derive(Debug, Default)]
pub struct Roads {
    cells: HashMap<(i32, i32), Vec<u32>>,
    pieces: Vec<Piece>,
    widest: f32,
}

fn cell_of(point: [f32; 2]) -> (i32, i32) {
    ((point[0] / CELL).floor() as i32, (point[1] / CELL).floor() as i32)
}

fn lerp(pair: [f32; 2], t: f32) -> f32 {
    pair[0] + (pair[1] - pair[0]) * t
}

impl Roads {
    /// The roads of `lines`.
    pub fn from_lines(lines: &[Line]) -> Self {
        let mut out = Self::default();
        for line in lines {
            for pair in line.points.windows(2) {
                let ((a, la, ha), (b, lb, hb)) = (pair[0], pair[1]);
                out.add(Piece { a, b, level: [la, lb], half: [ha, hb] });
            }
        }
        out
    }

    /// The roads of a network: every segment's centre line, a point every [`SPACING`] metres, with the carriageway's half width at its ends.
    pub fn from_network(net: &RoadNetwork) -> Self {
        let mut lines = Vec::with_capacity(net.segments.len());
        for (number, segment) in net.segments.iter().enumerate() {
            let number = number as u16;
            let half = |heading_end: bool| half_width(&travel_profile(net, number, 1, heading_end));
            let (start, end) = (half(false), half(true));
            if start <= 0.0 && end <= 0.0 {
                continue;
            }
            let curve = centre_line(net, number, 1);
            let steps = (segment.length / SPACING).ceil().max(1.0) as usize;
            let points = (0..=steps)
                .map(|step| {
                    let t = step as f32 / steps as f32;
                    let at = curve.position(t);
                    ([at.z, -at.x], at.y, start + (end - start) * t)
                })
                .collect();
            lines.push(Line { points });
        }
        Self::from_lines(&lines)
    }

    fn add(&mut self, piece: Piece) {
        let reach = piece.half[0].max(piece.half[1]);
        self.widest = self.widest.max(reach);
        let low = [piece.a[0].min(piece.b[0]) - reach, piece.a[1].min(piece.b[1]) - reach];
        let high = [piece.a[0].max(piece.b[0]) + reach, piece.a[1].max(piece.b[1]) + reach];
        let ((c0, r0), (c1, r1)) = (cell_of(low), cell_of(high));
        let number = self.pieces.len() as u32;
        self.pieces.push(piece);
        for row in r0..=r1 {
            for col in c0..=c1 {
                self.cells.entry((col, row)).or_default().push(number);
            }
        }
    }

    /// Whether there are no roads.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// The carriageway nearest to `at` whose edge is at most `reach` metres away, among the roads whose height there is from `low` to
    /// `high` (a bridge over the point, or a road under it, is another road). `None` when there is none.
    pub fn nearest(&self, at: [f32; 2], reach: f32, low: f32, high: f32) -> Option<Near> {
        let ((c0, r0), (c1, r1)) = (cell_of([at[0] - reach, at[1] - reach]), cell_of([at[0] + reach, at[1] + reach]));
        let mut best: Option<Near> = None;
        for row in r0..=r1 {
            for col in c0..=c1 {
                let Some(cell) = self.cells.get(&(col, row)) else { continue };
                for &number in cell {
                    let Some(near) = measure(&self.pieces[number as usize], at, low, high) else { continue };
                    if near.gap > reach || best.is_some_and(|known| known.gap <= near.gap) {
                        continue;
                    }
                    best = Some(near);
                }
            }
        }
        best
    }
}

/// How `at` stands to one piece: `None` when the road there is outside the heights `low` to `high`.
fn measure(piece: &Piece, at: [f32; 2], low: f32, high: f32) -> Option<Near> {
    let along = [piece.b[0] - piece.a[0], piece.b[1] - piece.a[1]];
    let length2 = along[0] * along[0] + along[1] * along[1];
    let t = match length2 > 1e-9 {
        true => (((at[0] - piece.a[0]) * along[0] + (at[1] - piece.a[1]) * along[1]) / length2).clamp(0.0, 1.0),
        false => 0.0,
    };
    let level = lerp(piece.level, t);
    if level < low || level > high {
        return None;
    }
    let foot = [piece.a[0] + along[0] * t, piece.a[1] + along[1] * t];
    let off = [at[0] - foot[0], at[1] - foot[1]];
    let distance = (off[0] * off[0] + off[1] * off[1]).sqrt();
    let away = match distance > 1e-3 {
        true => [off[0] / distance, off[1] / distance],
        false => {
            let length = length2.sqrt().max(1e-6);
            [-along[1] / length, along[0] / length]
        }
    };
    Some(Near { gap: distance - lerp(piece.half, t), level, away })
}

/// The half width of the carriageway of a cross section: the outer edge of its outermost [`CARRIAGEWAY`] zone.
fn half_width(profile: &blackbox_roads::RoadProfile) -> f32 {
    let edge = |index: usize| profile.signed_offset(index).abs() + profile.zones[index].width * 0.5;
    (0..profile.zones.len()).filter(|&i| CARRIAGEWAY.contains(&profile.zones[i].kind)).map(edge).fold(0.0, f32::max)
}

/// The roads the loader threads keep the trees off: `None` for no world, or a world without roads.
static ROADS: RwLock<Option<Arc<Roads>>> = RwLock::new(None);

/// Make the roads of `net` the ones trees keep off, or none. Called when a world opens, before its sections stream.
pub fn set_roads(net: Option<&RoadNetwork>) {
    let roads = net.map(Roads::from_network).filter(|roads| !roads.is_empty()).map(Arc::new);
    if let Ok(mut slot) = ROADS.write() {
        *slot = roads;
    }
}

/// The roads in effect.
pub fn roads() -> Option<Arc<Roads>> {
    ROADS.read().ok().and_then(|slot| slot.clone())
}

#[cfg(test)]
#[path = "roads_tests.rs"]
mod tests;
