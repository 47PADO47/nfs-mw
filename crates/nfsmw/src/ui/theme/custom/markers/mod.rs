//! The Custom HUD's world-space markers, the `WorldMarkers` slot (ADR 0013; `docs/specs/custom-hud/world-markers.md`,
//! with what was built and left out in `docs/specs/custom-hud-markers-impl.md`): the race's next gate, a recovery arrow
//! back to it, the race starts and places near the car, and the pursuit breakers in sight, projected through the
//! frame's gameplay camera, with edge arrows for the gate when it is off screen.
//!
//! Each frame, `layout` reads the feed and:
//!
//! 1. draws nothing, and forgets, with `[custom_hud] world_markers` off, the HUD hidden or no camera;
//! 2. takes the frame's events into the memory: a checkpoint pass starts its tick, a wrong way the recovery;
//! 3. gathers the candidates, places each through the camera, and keeps the first [`rules::MAX_MARKERS`] in priority
//!    order ([`arrange`]);
//! 4. draws the marks and arrows where they project, and the labels where the memory steadies them, snapped to whole
//!    pixels, so the camera's shake neither jitters nor swims the text.
//!
//! Submodules:
//!
//! - [`project`]: world to canvas, the edge arrows' ellipse, the label low-pass and the pixel snap.
//! - [`rules`]: the candidates, the priority order and the cap, the styling by distance and occlusion.
//! - [`draw`]: the shapes and the labels.
//! - [`memory`]: what is kept between frames, in [`MarkerMemoryCell`].

mod draw;
mod memory;
mod project;
#[cfg(test)]
mod project_tests;
mod rules;
#[cfg(test)]
mod rules_tests;
#[cfg(test)]
mod tests;

pub use memory::MarkerMemoryCell;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};
use glam::Vec2;

use crate::hud::HudCamera;
use crate::settings::SpeedUnits;

use super::feed::{CustomFeed, CustomFeedCell};
use super::layout::{HudBox, window};
use super::text::Sizing;
use super::tokens::Motion;
use draw::{ARROW_HALF, LABEL_GAP, MARK_HALF, Paint, Parts};
use memory::MarkerMemory;
use project::{PixelGrid, Placement};
use rules::{Candidate, Look};

/// The `WorldMarkers` component. Built per resolve from the shared feed; what it keeps between frames is in the feed's
/// marker memory.
pub struct WorldMarkers {
    feed: CustomFeedCell,
    parts: Option<Parts>,
}

impl WorldMarkers {
    /// The markers that read `feed` (the cell the Custom theme holds).
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, parts: None }
    }
}

impl Component for WorldMarkers {
    /// The markers do not use `bounds`: they point at the world, so they are placed on the whole window.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let cell = self.feed.markers();
        let mut memory = cell.lock();
        self.parts = compose(&feed, &mut memory, cx);
    }

    fn build(&self, out: &mut DrawList) {
        let Some(parts) = &self.parts else { return };
        parts.build(out);
    }
}

/// A candidate placed on screen this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub candidate: Candidate,
    pub look: Look,
    /// The mark on screen, or the edge arrow's centre.
    pub at: Vec2,
    /// For an edge arrow, the unit direction it points in (y down).
    pub arrow: Option<Vec2>,
}

/// What the frame's labels are drawn with.
struct Pass {
    paint: Paint,
    grid: PixelGrid,
    units: SpeedUnits,
    dt: f32,
}

/// The frame's markers, or `None` when nothing draws.
fn compose(feed: &CustomFeed, memory: &mut MarkerMemory, cx: &LayoutCx<'_>) -> Option<Parts> {
    let camera = feed.hud.camera.as_ref().filter(|_| feed.settings.world_markers && feed.hud.visible);
    let Some(camera) = camera else {
        memory.clear();
        return None;
    };
    let now = feed.elapsed_s;
    let dt = memory.observe(&feed.hud, now);
    let frame = HudBox::new(feed.screen, feed.settings.scale);
    let paint =
        Paint { opacity: feed.settings.opacity, scale: frame.scale(), sizing: Sizing::of(&frame, &feed.settings) };
    let pass = Pass { paint, grid: PixelGrid::of(feed.screen), units: feed.units, dt };
    let window = window(feed.screen);
    let candidates = rules::gather(&feed.hud, &feed.settings, memory.recovering(now));
    let placed = arrange(candidates, camera, window, paint.scale);
    let mut parts = Parts::new();
    for marker in &placed {
        draw_marker(&mut parts, marker, memory, &pass, cx);
    }
    let ids: Vec<u64> = placed.iter().map(|marker| marker.candidate.id).collect();
    memory.keep_labels(&ids);
    let motion = Motion::for_settings(&feed.settings);
    let tick = memory.tick(now, motion.pulse).and_then(|(tick, progress)| {
        let [x, y, z] = tick.at;
        match project::place(camera, window, [x, y, z + rules::GATE_LIFT], 0.0)? {
            Placement::OnScreen(at) => Some((at, progress)),
            Placement::OffScreen(_) => None,
        }
    });
    if let Some((at, progress)) = tick {
        parts.tick(at, progress, motion.pulse_moves, paint, cx);
    }
    (!parts.is_empty()).then_some(parts)
}

/// Places `candidates` through `camera` on the canvas of `window` at HUD scale `scale`: on screen where they project,
/// on the edge ellipse for a guiding marker off screen or behind the camera, and dropped otherwise (or when their look
/// hides them). Then keeps the first [`rules::MAX_MARKERS`] in priority order.
pub fn arrange(candidates: Vec<Candidate>, camera: &HudCamera, window: Rect, scale: f32) -> Vec<Placed> {
    let edge = project::edge_rect(window, ARROW_HALF * scale);
    let mut placed: Vec<Placed> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let look = rules::look(candidate.kind, candidate.distance, candidate.occluded)?;
            let margin = MARK_HALF * scale * look.size;
            match project::place(camera, window, candidate.position, margin)? {
                Placement::OnScreen(at) => Some(Placed { candidate, look, at, arrow: None }),
                Placement::OffScreen(direction) if candidate.kind.guides() => {
                    // Off screen the geometry in front of the target says nothing: the arrow keeps its distance look
                    // and always names the distance.
                    let look = rules::look(candidate.kind, candidate.distance, false)?;
                    let look = Look { label: true, ..look };
                    let at = project::edge_point(edge, direction);
                    Some(Placed { candidate, look, at, arrow: Some(direction) })
                }
                Placement::OffScreen(_) => None,
            }
        })
        .collect();
    rules::select(&mut placed, rules::MAX_MARKERS, |marker| (marker.candidate.kind, marker.candidate.distance));
    placed
}

/// Draws one placed marker: its mark or arrow at the raw projected point, and its label where the memory steadies it.
fn draw_marker(parts: &mut Parts, marker: &Placed, memory: &mut MarkerMemory, pass: &Pass, cx: &LayoutCx<'_>) {
    let Placed { candidate, look, at, arrow } = marker;
    match arrow {
        Some(direction) => parts.arrow(candidate.kind, *at, *direction, look.alpha, pass.paint, cx),
        None => parts.mark(candidate.kind, *at, look.size, look.alpha, pass.paint, cx),
    }
    if !look.label {
        return;
    }
    let Some(label) = draw::label_of(candidate, arrow.is_some(), pass.units, pass.paint.sizing) else { return };
    let target = label_target(marker, label.extent, pass.paint.scale);
    let steady = memory.label(candidate.id, target, pass.dt);
    parts.label(&label, pass.grid.snap(steady), look.alpha, pass.paint, cx);
}

/// Where a marker's label of `extent` (canvas units) goes: under the mark, or for an edge arrow toward the window's
/// centre from it, clear of the arrow whichever way it points.
pub fn label_target(marker: &Placed, extent: Vec2, scale: f32) -> Vec2 {
    let Some(direction) = marker.arrow else {
        let below = (MARK_HALF * marker.look.size + LABEL_GAP) * scale + extent.y / 2.0;
        return marker.at + Vec2::Y * below;
    };
    let reach = (ARROW_HALF + LABEL_GAP) * scale + (direction.x.abs() * extent.x + direction.y.abs() * extent.y) / 2.0;
    marker.at - direction * reach
}
