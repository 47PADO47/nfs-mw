//! The Custom map: a simplified road map drawn over the middle of the 3D view, translucent so the player sees both
//! the map and the world (`docs/specs/custom-hud-map-impl.md`). It fills the `Minimap` slot; the `Blips` slot is left
//! empty, since the map draws its own marks.
//!
//! `layout` does the frame's work: it reads the feed (`CustomFeedCell`), moves the mode change and the zoom spring on,
//! refreshes the cached local set of roads when the car has moved far enough, and projects the roads, the route, the
//! marks and the player's arrow into canvas units, clipped to a soft ellipse. `build` emits what `layout` made.
//!
//! - [`modes`]: the four modes, the change from one to the next, and the map's keys.
//! - [`memory`]: what the map keeps between frames (the local set, the zoom spring, the mode change).
//! - [`view`]: the projection (north-up or heading-up) and the ellipse.
//! - [`roads`]: the local set's cache rule and the road strokes.
//! - [`marks`]: the marks as data, under the fair-knowledge rules.
//! - [`draw`]: the marks' shapes, the route and the arrow.
//! - [`chip`]: the Compact mode's turn-cue chip.

pub mod chip;
#[cfg(test)]
mod component_tests;
pub mod draw;
pub mod marks;
pub mod memory;
pub mod modes;
pub mod roads;
#[cfg(test)]
mod tests;
pub mod view;
#[cfg(test)]
mod view_tests;

use blackbox_minimap::Orientation;
use blackbox_ui::{Component, DrawList, LayoutCx, Polylines, Rect};
use glam::Vec2;

use self::draw::{Stage, arrow, colour, draw_marks, route_lines};
use self::memory::{MapMemory, MapMemoryCell};
use self::modes::{Overlay, transition};
use self::roads::{RoadLook, road_lines};
use self::view::{MapView, ellipse};
use super::feed::{CustomFeed, CustomFeedCell};
use super::layout::HudBox;
use super::tokens::{Motion, fade, keys};

/// Depths of the map's layers: far behind the HUD's other elements (`DEFAULT_Z`), the roads farthest, the arrow nearest.
pub const ROAD_Z: f32 = 60.0;
pub const ROUTE_Z: f32 = 59.0;
pub const MARK_Z: f32 = 58.0;
pub const ARROW_Z: f32 = 57.0;
/// How much more opaque the route is than the roads, and the share of that the stretch already driven keeps.
pub const ROUTE_GAIN: f32 = 0.4;
pub const DRIVEN_SHARE: f32 = 0.5;
/// The side roads' alpha as a share of the main roads'.
pub const SIDE_SHARE: f32 = 0.8;

/// The Custom map component. Built per resolve from the shared feed; its state between frames is the feed cell's
/// map memory.
pub struct CustomMap {
    feed: CustomFeedCell,
    memory: MapMemoryCell,
    lines: Polylines,
    parts: Vec<Box<dyn Component>>,
}

impl CustomMap {
    /// The map that reads `feed` and keeps its state in the feed cell's map memory.
    pub fn new(feed: CustomFeedCell) -> Self {
        let memory = feed.map_memory();
        Self { feed, memory, lines: Polylines::new(), parts: Vec::new() }
    }
}

impl Component for CustomMap {
    /// The map does not use `bounds`: it sits in the content box's centre, from the feed's window.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        self.lines.clear();
        self.parts.clear();
        let (lines, parts) = (&mut self.lines, &mut self.parts);
        self.memory.with(|memory| compose(&feed, memory, cx, (lines, parts)));
    }

    fn build(&self, out: &mut DrawList) {
        self.lines.build(out);
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The map for one frame of `feed`, into the strokes `lines` and the other parts `parts`.
pub fn compose(
    feed: &CustomFeed,
    memory: &mut MapMemory,
    cx: &LayoutCx<'_>,
    (lines, parts): (&mut Polylines, &mut Vec<Box<dyn Component>>),
) {
    let (hud, settings) = (&*feed.hud, &feed.settings);
    if !hud.visible {
        return;
    }
    let motion = Motion::for_settings(settings);
    memory.mode.update(settings.map_mode, feed.elapsed_s);
    let progress = memory.mode.progress(feed.elapsed_s, motion.map_mode);
    let from = Overlay::of(memory.mode.previous(), settings.map_opacity);
    let to = Overlay::of(memory.mode.current(), settings.map_opacity);
    let overlay = transition(from, to, progress, settings.motion_reduction);
    let frame = HudBox::new(feed.screen, settings.scale);
    if overlay.chip > 0.0
        && let Some(route) = &hud.route
    {
        parts.extend(chip::chip(route, feed.units, (&frame, settings), overlay.chip, cx).into_iter().flatten());
    }
    if overlay.shown <= 0.0 {
        return;
    }
    let Some(car) = hud.map_car.or(hud.minimap) else { return };
    let heading_up = hud.minimap.is_some_and(|map| map.orientation == Orientation::Heading);
    let reduced = settings.motion_reduction;
    let zoom = memory.zoom.step(settings.map_zoom, feed.delta_s, feed.elapsed_s, motion.map_zoom, reduced);
    let alpha = overlay.shown * memory.zoom.crossfade(feed.elapsed_s, motion.map_zoom, reduced);
    let content = frame.content();
    let radii = Vec2::new(overlay.radii[0], overlay.radii[1]) * content.h;
    let centre = Vec2::new(content.x, content.y);
    let view_radius = overlay.base_m / zoom.max(0.01);
    let view = MapView::new(car.position, car.heading, heading_up, radii.x / view_radius, centre);
    let (clip, band) = ellipse(centre, radii);
    let stage = Stage { view: &view, clip, band, radii, scale: frame.scale() };
    let roads_opacity = overlay.road_opacity * alpha;
    if let Some(roads) = &hud.map_roads {
        memory.local.refresh(roads, car.position, view_radius);
        let look = road_look(cx, settings.high_contrast, (roads_opacity, overlay.detail), stage.scale);
        road_lines(roads, &memory.local, &view, (clip, band), &look, ROAD_Z, lines);
    }
    if let Some(route) = &hud.route {
        let nav = colour(cx, keys::NAV);
        let ahead = fade(nav, (overlay.road_opacity + ROUTE_GAIN).min(1.0) * alpha);
        let behind = fade(ahead, DRIVEN_SHARE * overlay.detail);
        route_lines(route, car.position, &stage, (ahead, behind), ROUTE_Z, lines);
    }
    let marks_alpha = settings.opacity * alpha;
    draw_marks(&marks::marks(hud), &stage, (marks_alpha, MARK_Z), cx, parts);
    let direction = view.direction(car.heading);
    let tones = (fade(colour(cx, keys::TEXT), marks_alpha), fade(colour(cx, keys::SURFACE), marks_alpha));
    for line in arrow(centre, direction, stage.scale, tones, ARROW_Z) {
        parts.push(Box::new(line));
    }
}

/// The road colours for `opacity` (the roads' opacity times the overlay's fade) and `detail` (the Expanded map's
/// paths), from the skin's tokens.
fn road_look(cx: &LayoutCx<'_>, high_contrast: bool, (opacity, detail): (f32, f32), scale: f32) -> RoadLook {
    RoadLook {
        main: fade(colour(cx, keys::TEXT), opacity),
        side: fade(colour(cx, keys::TEXT_SECONDARY), opacity * SIDE_SHARE),
        path: fade(colour(cx, keys::RULE), opacity * detail),
        under: high_contrast.then(|| fade(colour(cx, keys::SURFACE), opacity)),
        scale,
    }
}
