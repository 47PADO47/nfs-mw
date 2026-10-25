//! `Polyline` and `Polylines`: lines stroked through a list of points in canvas units, for roads, routes and outlines.
//!
//! The presenter draws quads only, one colour each, so a stroke is a run of rotated rectangles, one per segment (more
//! across a fade band). A join is either a mitre, which runs both segments on to the mitre's point and costs no node,
//! or a bevel, which costs one rectangle over the outer notch. A polyline of hundreds of roads is one [`Polylines`].
//!
//! The width is in canvas units, which the presenter scales evenly, so a stroke keeps its width on screen whatever its
//! direction. Nothing is cached: `build` walks the points and pushes nodes straight into the draw list, so a stroke
//! allocates nothing beyond the nodes it emits.
//!
//! - `clip`: the [`Clip`] region strokes stay inside, and its soft edge.
//! - `geometry`: the quad of a segment, the join fills, and the pieces across a fade band.
//! - `stroke`: the walk that turns one polyline into nodes.

mod clip;
mod geometry;
mod stroke;
#[cfg(test)]
mod tests;

pub use clip::Clip;

use glam::Vec2;

use super::DEFAULT_Z;
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;

/// How a polyline fills the corner where one segment turns into the next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineJoin {
    /// The outer corner cut straight across: one extra rectangle per turn.
    #[default]
    Bevel,
    /// The outer edges run on until they meet in a point, with no extra node. A turn whose point would reach farther than
    /// `limit` half widths from the joint is bevelled instead. A limit of 2 or less (turns up to 120 degrees) keeps the
    /// inner side of the corner within the stroke too.
    Mitre { limit: f32 },
}

/// How a polyline ends at its first and last point. A clipped end is always cut flat at the clip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    /// The stroke stops at the point.
    #[default]
    Butt,
    /// The stroke runs on by half its width past the point, a small square end.
    Square,
}

/// A stroked line through `points`, in one colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline {
    points: Vec<Vec2>,
    width: f32,
    colour: [u8; 4],
    join: LineJoin,
    cap: LineCap,
    clip: Clip,
    fade: f32,
    z: f32,
}

impl Polyline {
    /// The line through `points` (canvas units: FEng, origin at the centre), `width` units wide, in `colour` (straight
    /// RGBA). Repeated and non-finite points are skipped, so a zero-length segment draws nothing. Bevel joins, butt caps,
    /// no clip, at the default depth.
    pub fn new(points: Vec<Vec2>, width: f32, colour: [u8; 4]) -> Self {
        Self {
            points,
            width,
            colour,
            join: LineJoin::Bevel,
            cap: LineCap::Butt,
            clip: Clip::None,
            fade: 0.0,
            z: DEFAULT_Z,
        }
    }

    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    /// Keeps the line inside `clip`. `fade` is the width of a band inside the clip's edge across which the line fades
    /// out, in canvas units (0 for a hard edge). The band is drawn in short pieces, each in the opacity at its middle.
    pub fn with_clip(mut self, clip: Clip, fade: f32) -> Self {
        self.clip = clip;
        self.fade = fade;
        self
    }

    /// Draws the line at depth `z` (a lower `z` draws on top).
    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// The points, to refill in place, so a line rebuilt every few frames keeps its allocation.
    pub fn points_mut(&mut self) -> &mut Vec<Vec2> {
        &mut self.points
    }

    pub fn set_colour(&mut self, colour: [u8; 4]) {
        self.colour = colour;
    }

    /// About how many nodes `build` pushes: one per segment and one per bevel. A fade band adds more; this is a hint
    /// for reserving room, not a bound.
    pub fn node_estimate(&self) -> usize {
        self.points.len().saturating_sub(1) * 2
    }
}

impl Component for Polyline {
    /// The points are already in canvas units, so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        out.reserve(self.node_estimate());
        stroke::emit(self, out);
    }
}

/// Many polylines in one component: a whole road map, drawn in the order the lines were added.
#[derive(Clone, Debug, Default)]
pub struct Polylines {
    lines: Vec<Polyline>,
    reserve: usize,
}

impl Polylines {
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty set with room for `lines` lines.
    pub fn with_capacity(lines: usize) -> Self {
        Self { lines: Vec::with_capacity(lines), reserve: 0 }
    }

    /// How many nodes `build` should reserve room for, when the caller knows better than [`Polylines::node_estimate`]
    /// (a fade band, for one). The larger of the two is reserved.
    pub fn with_reserve(mut self, nodes: usize) -> Self {
        self.reserve = nodes;
        self
    }

    pub fn with_line(mut self, line: Polyline) -> Self {
        self.lines.push(line);
        self
    }

    pub fn push(&mut self, line: Polyline) {
        self.lines.push(line);
    }

    /// Removes every line and keeps the room, for a set rebuilt in place.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn lines(&self) -> &[Polyline] {
        &self.lines
    }

    /// The nodes `build` reserves room for: the reserve hint, or the lines' own estimates when those are more.
    pub fn node_estimate(&self) -> usize {
        let estimate: usize = self.lines.iter().map(Polyline::node_estimate).sum();
        estimate.max(self.reserve)
    }
}

impl Component for Polylines {
    /// The lines are already in canvas units, so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        out.reserve(self.node_estimate());
        for line in &self.lines {
            stroke::emit(line, out);
        }
    }
}
