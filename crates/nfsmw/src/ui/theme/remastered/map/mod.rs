//! The Remastered world map: the whole screen is the map. The career stage's picture fills the window behind a dark
//! ground, the blips of the current tab sit on it as the game's own icons, the cursor is drawn here (a cross, or square
//! brackets that snap onto the blip it is over), the title and compass are top left and top right, a legend of the tab's
//! icons is at the right, and the footer hints are the menus' footer. The FEng screen runs underneath for its logic and is
//! not presented (`frontend/plugin.rs`); the pointer, zoom, pan, tabs and GPS prompt are the same session as every map's.
//!
//! The frontend fills [`WorldMapViewCell`] each frame the map is open; the component draws nothing without it.
//!
//! - [`draw`]: the picture, the blips, the player and the cursor.
//! - [`panels`]: the title, the compass, the legend and the dark bands.

mod draw;
mod panels;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect};

use super::group::Group;
use super::menu::Parts;
use super::skin::keys;
use crate::frontend::world_map::WorldMapFrame;
use crate::ui::theme::MenuHint;

/// How long the map takes to come in, in seconds.
const ENTRANCE: f32 = 0.3;

/// One frame of the open map, as the theme draws it: the map's frame, the footer hints with the player's bindings, and
/// the window's aspect (width over height), which says where the screen's edges lie in FEng units.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSnapshot {
    pub frame: WorldMapFrame,
    pub hints: Vec<MenuHint>,
    pub aspect: f32,
}

/// The shared cell. Clones share one snapshot.
#[derive(Clone, Debug, Default)]
pub struct WorldMapViewCell {
    snapshot: Arc<Mutex<Option<MapSnapshot>>>,
}

impl WorldMapViewCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores this frame's map.
    pub(crate) fn set(&self, snapshot: MapSnapshot) {
        *self.lock() = Some(snapshot);
    }

    /// Marks the map closed, so the component draws nothing.
    pub(crate) fn clear(&self) {
        *self.lock() = None;
    }

    pub(in crate::ui::theme) fn get(&self) -> Option<MapSnapshot> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Option<MapSnapshot>> {
        self.snapshot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The Remastered map component. Built per resolve from the shared cell; it holds no state of its own.
pub struct WorldMapView {
    cell: WorldMapViewCell,
    parts: Vec<Box<dyn Component>>,
}

impl WorldMapView {
    pub fn new(cell: WorldMapViewCell) -> Self {
        Self { cell, parts: Vec::new() }
    }
}

impl Component for WorldMapView {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.get() {
            Some(snapshot) => assemble(&snapshot, cx),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// `t` eased out: fast at first, settling at 1.
pub(super) fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// The map as parts, back to front, the whole of it fading in as the map opens.
fn assemble(snapshot: &MapSnapshot, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let frame = &snapshot.frame;
    let half_width = snapshot.aspect * 240.0;
    let mut parts = Parts::new(cx);
    let ground = Rect::new(0.0, 0.0, half_width * 2.0 + 4.0, 484.0);
    parts.push(Panel::new(keys::MAP_GROUND), ground);
    draw::picture(&mut parts, frame);
    draw::blips(&mut parts, frame);
    draw::player(&mut parts, frame);
    panels::bands(&mut parts, half_width);
    panels::title(&mut parts, frame, -half_width);
    panels::compass(&mut parts, frame, half_width);
    panels::legend(&mut parts, frame, half_width);
    parts.footer(&snapshot.hints, -half_width);
    draw::cursor(&mut parts, snapshot);
    let entrance = ease_out(frame.clock / ENTRANCE);
    vec![Box::new(Group::new(parts.finish(), entrance))]
}
