//! The Custom full-screen world map (the `WorldMapChrome` slot, `docs/specs/custom-hud/menus.md` section 3): the whole
//! map drawn by the theme, as the Remastered theme does, so the FEng map screen runs underneath for its logic and is not
//! presented (`frontend/plugin.rs`). It reuses the centred map's drawing (`custom/map`): the road polylines of
//! `hud/map_roads`, the road look, the route, the mark shapes and the player's arrow.
//!
//! The frontend fills [`FullMapCell`] each frame the map is open with a [`FullMapSnapshot`]: the frontend's map frame
//! (the tab, the camera, the focused blip, the cursor, the blips, the player, and the roads, the GPS route and the
//! city's calibration it read at open), the footer hints with the player's bindings, and the window's aspect. The
//! component draws nothing without it.
//!
//! Back to front: the ground over the frozen world, the roads, the GPS route, the current tab's marks and the route's
//! end, the players' arrows, the cursor, then the chrome (the title with the tabs and the zoom, the legend, the focused
//! blip's card and the footer).
//!
//! - [`fit`]: the projection from the world through the frontend's camera to the screen.
//! - [`places`]: the marks and the legend's rows as data, under the frontend's filter.
//! - [`chrome`]: the ground, the title and tabs, the legend.
//! - [`card`]: the cursor and the focused blip's card.

pub mod card;
pub mod chrome;
pub mod fit;
pub mod places;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_minimap::Calibration;
use blackbox_ui::{Clip, Component, DrawList, LayoutCx, Polylines, Rect};
use glam::Vec2;

use super::map::draw::{Stage, arrow, colour, draw_marks, route_lines};
use super::map::roads::{LocalSet, road_lines};
use super::map::view::MapView;
use super::map::{ARROW_Z, DRIVEN_SHARE, MARK_Z, ROAD_Z, ROUTE_Z, road_look};
use super::menu::footer_parts;
use super::menu::layout::MenuFrame;
use super::menu::look::MenuLook;
use super::tokens::{fade, keys};
use crate::frontend::world_map::{PlayerMark, WorldMapFrame};
use crate::hud::MapRoadsHandle;
use crate::ui::theme::MenuHint;

/// The roads' opacity on the full map, and their detail (1 draws the paths too).
pub const ROAD_OPACITY: f32 = 0.85;
pub const ROAD_DETAIL: f32 = 1.0;
/// The marks, the route and the arrows are drawn this much larger than the menus' scale; the roads keep it.
pub const MARK_SCALE: f32 = 1.6;

/// One frame of the open map, as the Custom theme draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct FullMapSnapshot {
    pub frame: WorldMapFrame,
    /// The footer hints, with the player's bindings (`frontend/world_map/control.rs` `hints`).
    pub hints: Vec<MenuHint>,
    /// The window's width over its height: where the screen's edges lie in canvas units.
    pub aspect: f32,
}

/// What the map keeps between frames: the roads in view (queried again only when the view moves or zooms enough, by
/// the centred map's cache rule), and the calibration fitted to a network when the map has none of its own.
#[derive(Debug, Default)]
pub struct FullMapMemory {
    local: LocalSet,
    fit: Option<(MapRoadsHandle, Option<Calibration>)>,
}

impl FullMapMemory {
    /// The calibration that places the world on the map: the city's, else one fitted to the roads (computed once per
    /// network), else `None`.
    fn calibration(&mut self, map: &WorldMapFrame) -> Option<Calibration> {
        if map.calibration.is_some() {
            return map.calibration;
        }
        let roads = map.roads.as_ref()?;
        if let Some((source, known)) = &self.fit
            && source.same(roads)
        {
            return *known;
        }
        let fitted = fit::fitted(roads.roads());
        self.fit = Some((roads.clone(), fitted));
        fitted
    }
}

/// The shared cell. Clones share one snapshot and one memory.
#[derive(Clone, Debug, Default)]
pub struct FullMapCell {
    snapshot: Arc<Mutex<Option<FullMapSnapshot>>>,
    memory: Arc<Mutex<FullMapMemory>>,
}

impl FullMapCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores this frame's map.
    pub(crate) fn set(&self, snapshot: FullMapSnapshot) {
        *lock(&self.snapshot) = Some(snapshot);
    }

    /// Marks the map closed, so the component draws nothing, and lets the roads it kept go.
    pub(crate) fn clear(&self) {
        *lock(&self.snapshot) = None;
        *lock(&self.memory) = FullMapMemory::default();
    }

    pub(in crate::ui::theme) fn get(&self) -> Option<FullMapSnapshot> {
        lock(&self.snapshot).clone()
    }

    fn with_memory<R>(&self, f: impl FnOnce(&mut FullMapMemory) -> R) -> R {
        f(&mut lock(&self.memory))
    }
}

/// A lock whose poison is ignored: each write replaces the whole value.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The Custom full-screen map component. Built per resolve from the shared cell; its state between frames is the
/// cell's memory.
pub struct FullMap {
    cell: FullMapCell,
    back: Vec<Box<dyn Component>>,
    lines: Polylines,
    front: Vec<Box<dyn Component>>,
}

impl FullMap {
    pub fn new(cell: FullMapCell) -> Self {
        Self { cell, back: Vec::new(), lines: Polylines::new(), front: Vec::new() }
    }
}

impl Component for FullMap {
    /// The map does not use `bounds`: it fills the window, whose shape comes from the snapshot.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.back.clear();
        self.lines.clear();
        self.front.clear();
        let Some(snapshot) = self.cell.get() else { return };
        let layers = Layers { back: &mut self.back, lines: &mut self.lines, front: &mut self.front };
        self.cell.with_memory(|memory| compose(&snapshot, memory, cx, layers));
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.back {
            part.build(out);
        }
        self.lines.build(out);
        for part in &self.front {
            part.build(out);
        }
    }
}

/// Where one frame's parts go: behind the strokes, the strokes, and in front of them.
struct Layers<'a> {
    back: &'a mut Vec<Box<dyn Component>>,
    lines: &'a mut Polylines,
    front: &'a mut Vec<Box<dyn Component>>,
}

/// The map for one frame of `snapshot`.
fn compose(snapshot: &FullMapSnapshot, memory: &mut FullMapMemory, cx: &LayoutCx<'_>, layers: Layers<'_>) {
    let map = &snapshot.frame;
    let frame = MenuFrame::new(snapshot.aspect, MenuLook::from_skin(cx.skin).text_size());
    layers.back.push(chrome::ground(frame.window(), cx));
    if let Some(calibration) = memory.calibration(map) {
        let view = fit::view(calibration, map.session.camera);
        world(map, &view, &frame, memory, cx, (&mut *layers.lines, &mut *layers.front));
    }
    card::cursor(map, frame.scale() * MARK_SCALE, cx, layers.front);
    chrome::title(map, &frame, cx, layers.front);
    if map.session.legend_open {
        chrome::legend(map, &frame, cx, layers.front);
    }
    card::card(map, &frame, cx, layers.front);
    layers.front.extend(footer_parts(&snapshot.hints, frame, cx));
}

/// What lies on the map under `view`: the roads in the window, the GPS route, the current tab's marks and the route's
/// end, and the players' arrows.
fn world(
    map: &WorldMapFrame,
    view: &MapView,
    frame: &MenuFrame,
    memory: &mut FullMapMemory,
    cx: &LayoutCx<'_>,
    (lines, front): (&mut Polylines, &mut Vec<Box<dyn Component>>),
) {
    let window = frame.window();
    let clip = Clip::Rect(window);
    let radii = Vec2::new(window.w, window.h) / 2.0;
    let scale = frame.scale() * MARK_SCALE;
    let stage = Stage { view, clip, band: 0.0, radii, scale };
    if let Some(roads) = &map.roads {
        // The window's half diagonal, in metres: every road in the window is inside it.
        let reach = radii.length() / view.scale.max(f32::EPSILON);
        memory.local.refresh(roads, view.car, reach);
        let high_contrast = cx.skin.colour(keys::OUTLINE).is_some();
        let look = road_look(cx, high_contrast, (ROAD_OPACITY, ROAD_DETAIL), frame.scale());
        road_lines(roads, &memory.local, view, (clip, 0.0), &look, ROAD_Z, lines);
    }
    if let Some(route) = &map.route {
        let ahead = colour(cx, keys::NAV);
        // Without the player's place the whole route is ahead: it is split at its start.
        let car = match map.player_mark {
            Some(_) => map.player,
            None => route.path.first().copied().unwrap_or(route.target),
        };
        route_lines(route, car, &stage, (ahead, fade(ahead, DRIVEN_SHARE)), ROUTE_Z, lines);
    }
    draw_marks(&places::marks(map), &stage, (1.0, MARK_Z), None, cx, front);
    let second = (colour(cx, keys::TEXT_SECONDARY), colour(cx, keys::SURFACE));
    let first = (colour(cx, keys::TEXT), colour(cx, keys::SURFACE));
    for (mark, tones) in [(map.second_mark, second), (map.player_mark, first)] {
        let Some(mark) = mark else { continue };
        let at = fit::on_screen(map.session.camera, mark.map);
        let strokes = arrow(at, bearing_direction(mark), scale, tones, ARROW_Z);
        front.extend(strokes.into_iter().map(|line| Box::new(line) as Box<dyn Component>));
    }
}

/// The screen direction of a car mark's bearing (degrees clockwise from up the picture).
fn bearing_direction(mark: PlayerMark) -> Vec2 {
    let (sin, cos) = mark.bearing.to_radians().sin_cos();
    Vec2::new(sin, -cos)
}
