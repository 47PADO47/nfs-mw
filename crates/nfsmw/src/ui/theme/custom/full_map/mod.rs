//! The Custom full-screen world map (the `WorldMapChrome` slot, `docs/specs/custom-hud/menus.md` section 3): the whole
//! map drawn by the theme, as the Remastered theme does, so the FEng map screen runs underneath for its logic and is not
//! presented (`frontend/plugin.rs`). The map itself is the game's: the picture of the career stage's map (or the running
//! race's own) and the game's icon textures for the blips, the cars and the checkpoint, the same files the Remastered map
//! draws ([`art`]). Custom brings the chrome round them and the GPS route's strokes.
//!
//! The frontend fills [`FullMapCell`] each frame the map is open with a [`FullMapSnapshot`]: the frontend's map frame
//! (the tab, the camera, the focused blip, the cursor, the blips, the cars, and the GPS route and the city's calibration
//! it read at open), the footer hints with the player's bindings, and the window's aspect. The component draws nothing
//! without it.
//!
//! Back to front: the backdrop over the frozen world (a scrim over the whole window and a strong plate under the map's
//! picture, faded in as the map opens), the picture with its veil, the GPS route, the current tab's blips, the cars and
//! the route's end as icons, the cursor, then the chrome (the title with the tabs and the zoom, the legend, the focused
//! blip's card and the footer).
//!
//! - [`art`]: the picture and the icons.
//! - [`fit`]: the projection from the world through the frontend's camera to the screen, for the route.
//! - [`places`]: the focused blip and the legend's rows as data, under the frontend's filter.
//! - [`chrome`]: the backdrop, the title and tabs, the legend.
//! - [`card`]: the cursor and the focused blip's card.

pub mod art;
#[cfg(test)]
mod backdrop_tests;
pub mod card;
pub mod chrome;
pub mod fit;
pub mod places;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::{Clip, Component, DrawList, LayoutCx, Polylines, Rect, token_keys};
use glam::Vec2;

use super::map::draw::{Stage, colour, route_lines};
use super::map::modes::ease;
use super::map::{DRIVEN_SHARE, ROUTE_Z};
use super::menu::footer_parts;
use super::menu::layout::MenuFrame;
use super::menu::look::MenuLook;
use super::tokens::{Motion, fade, ramp};
use crate::frontend::world_map::WorldMapFrame;
use crate::ui::theme::MenuHint;

/// The cursor and the route are drawn this much larger than the menus' scale.
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

/// What the map keeps between frames: when it was first drawn since it opened, which the backdrop and the art fade in
/// from.
#[derive(Debug, Default)]
pub struct FullMapMemory {
    /// When the map was first drawn since it opened, on the layout clock (`LayoutCx::time_s`).
    opened_at: Option<f32>,
}

impl FullMapMemory {
    /// How open the map is at `now`, 0 to 1: the first frame is 0, and it is 1 once `duration` seconds have passed, eased
    /// like the HUD map's mode change. The backdrop and the art fade in with it.
    pub fn open(&mut self, now: f32, duration: f32) -> f32 {
        let since = *self.opened_at.get_or_insert(now);
        ease(ramp(now - since, duration))
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

    /// Marks the map closed, so the component draws nothing, and lets the opening it kept go.
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
    let look = MenuLook::from_skin(cx.skin);
    let frame = MenuFrame::new(snapshot.aspect, look.text_size());
    let open = memory.open(cx.time_s, Motion::of(look.reduce_motion()).mode_change);
    layers.back.extend(chrome::backdrop(frame.window(), open, cx));
    art::picture(map, open, cx, layers.back);
    route(map, &frame, open, cx, layers.lines);
    let icons = (frame.scale(), open);
    art::blips(map, icons, frame.window(), cx, layers.front);
    art::live(map, icons, frame.window(), cx, layers.front);
    art::route_end(map, icons, cx, layers.front);
    art::cars(map, icons, frame.window(), cx, layers.front);
    card::cursor(map, frame.scale() * MARK_SCALE, cx, layers.front);
    chrome::title(map, &frame, cx, layers.front);
    if map.session.legend_open {
        chrome::legend(map, &frame, cx, layers.front);
    }
    card::card(map, &frame, cx, layers.front);
    layers.front.extend(footer_parts(&snapshot.hints, frame, cx));
}

/// The GPS route's strokes on the picture, in the guide colour, the stretch behind the car dimmer. Nothing without the
/// route or the city's calibration, which places it.
fn route(map: &WorldMapFrame, frame: &MenuFrame, open: f32, cx: &LayoutCx<'_>, lines: &mut Polylines) {
    let (Some(route), Some(calibration)) = (&map.route, map.calibration) else { return };
    let view = fit::view(calibration, map.session.camera);
    let window = frame.window();
    let radii = Vec2::new(window.w, window.h) / 2.0;
    let stage = Stage { view: &view, clip: Clip::Rect(window), band: 0.0, radii, scale: frame.scale() * MARK_SCALE };
    let ahead = fade(colour(cx, token_keys::GUIDE), open);
    // Without the player's place the whole route is ahead: it is split at its start.
    let car = match map.player_mark {
        Some(_) => map.player,
        None => route.path.first().copied().unwrap_or(route.target),
    };
    route_lines(route, car, &stage, (ahead, fade(ahead, DRIVEN_SHARE)), ROUTE_Z, lines);
}
