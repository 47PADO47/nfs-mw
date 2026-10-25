//! The walk that turns one polyline into nodes. It reads the points once, looking one point ahead so each segment knows
//! the joint at its end before it is drawn: a mitre there runs the segment on, a bevel adds one rectangle after it.
//!
//! Every node is an untextured image (texture 0, which the presenter fills with its solid white) in the line's colour,
//! its alpha scaled by the fade at the middle of the piece.

use glam::{Mat4, Vec2};

use super::clip::{Clip, positive};
use super::geometry::{Joint, distinct, joint, pieces, quad};
use super::{LineCap, Polyline};
use crate::components::{FULL_UV, UNROTATED, node_at};
use crate::draw::DrawList;
use crate::tree::NodeKind;

/// How many pieces a fade band is cut into across its width: enough that the steps in opacity do not show.
const BAND_STEPS: f32 = 6.0;

/// A polyline's clip, worked out once per line: the region, the part of it at full opacity, and the fade band.
struct Region {
    clip: Clip,
    /// The region less the band, or `None` when the band covers all of it. Unused without a band.
    inner: Option<Clip>,
    /// The band width; 0 for a hard edge or no clip.
    band: f32,
}

impl Region {
    fn new(clip: Clip, band: f32) -> Self {
        if !positive(band) || clip == Clip::None {
            return Self { clip, inner: None, band: 0.0 };
        }
        Self { clip, inner: clip.inset(band), band }
    }

    /// The longest band piece of a segment `length` units long, as a fraction of it; `None` without a band.
    fn step(&self, length: f32) -> Option<f32> {
        positive(self.band).then_some(self.band / BAND_STEPS / length)
    }

    /// The opacity at `p`.
    fn fade_at(&self, p: Vec2) -> f32 {
        self.clip.fade(p, self.band)
    }
}

/// Pushes the nodes of `line` onto `out`.
pub(super) fn emit(line: &Polyline, out: &mut DrawList) {
    if !positive(line.width) || line.colour[3] == 0 {
        return;
    }
    let region = Region::new(line.clip, line.fade);
    let half = line.width / 2.0;
    let cap = match line.cap {
        LineCap::Butt => 0.0,
        LineCap::Square => half,
    };
    let mut points = distinct(&line.points);
    let (Some(mut from), Some(mut to)) = (points.next(), points.next()) else { return };
    let mut start = cap;
    loop {
        let direction = (to - from).normalize();
        let next = points.next();
        let corner = next.map_or(Joint::None, |n| joint(direction, (n - to).normalize(), half, line.join));
        let end = match (next, corner) {
            (None, _) => cap,
            (Some(_), Joint::Extend(by)) => by,
            (Some(_), _) => 0.0,
        };
        segment(line, &region, (from, to), (start, end), out);
        if let Joint::Bevel { offset, along, across } = corner {
            bevel(line, &region, to, quad(to + offset, along, across, line.z), out);
        }
        let Some(after) = next else { return };
        start = match corner {
            Joint::Extend(by) => by,
            _ => 0.0,
        };
        (from, to) = (to, after);
    }
}

/// The segment from `a` to `b`, clipped, cut into pieces across the fade band, and run on by `start` before `a` and
/// `end` past `b` (a cap or a mitre) where those ends are not clipped away.
fn segment(line: &Polyline, region: &Region, (a, b): (Vec2, Vec2), (start, end): (f32, f32), out: &mut DrawList) {
    let Some(span) = region.clip.interval(a, b) else { return };
    let d = b - a;
    let length = d.length();
    let direction = d / length;
    let across = direction.perp() * line.width;
    let full = region.inner.and_then(|inner| inner.interval(a, b));
    for piece in pieces(span, full, region.step(length)) {
        let (p0, p1) = (a + d * piece.t0, a + d * piece.t1);
        let Some(colour) = faded(line.colour, region.fade_at((p0 + p1) / 2.0)) else { continue };
        let s0 = p0 - direction * extension(piece.first && span.0 <= 0.0, start);
        let s1 = p1 + direction * extension(piece.last && span.1 >= 1.0, end);
        push(out, quad((s0 + s1) / 2.0, s1 - s0, across, line.z), line.z, colour);
    }
}

/// The bevel fill `world` of the joint at `at`, when the joint is inside the clip.
fn bevel(line: &Polyline, region: &Region, at: Vec2, world: Mat4, out: &mut DrawList) {
    if !region.clip.contains(at) {
        return;
    }
    let Some(colour) = faded(line.colour, region.fade_at(at)) else { return };
    push(out, world, line.z, colour);
}

/// `by` where an end is run on, 0 where it is not.
fn extension(applies: bool, by: f32) -> f32 {
    if !applies {
        return 0.0;
    }
    by
}

/// `colour` with its alpha scaled by `opacity`, or `None` when nothing would show.
fn faded(colour: [u8; 4], opacity: f32) -> Option<[u8; 4]> {
    let alpha = (f32::from(colour[3]) * opacity).round().clamp(0.0, 255.0) as u8;
    (alpha > 0).then_some([colour[0], colour[1], colour[2], alpha])
}

/// Pushes one untextured quad.
fn push(out: &mut DrawList, world: Mat4, z: f32, colour: [u8; 4]) {
    let kind = NodeKind::Image { texture: 0, uv: FULL_UV, mask: None, mask_rotation: UNROTATED, mask_uv: FULL_UV };
    out.push(node_at(world, z, colour, kind));
}
