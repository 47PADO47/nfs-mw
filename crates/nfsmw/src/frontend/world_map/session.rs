//! One open map: the filter tab, the camera and the blip the cursor is focused on. [`WorldMap::frame`] takes one
//! frame of input and says whether the map stays, closes, or asks for a GPS route on the focused blip.

use super::legend::VISIBLE_ROWS;
use super::{Camera, Filter, PICK_RADIUS, nearest};

/// A blip on the map. `world` is metres in render space (x forward, y left); `map` is its place on the picture,
/// 0..1 on both axes (`blackbox_minimap::Calibration::to_map`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedBlip {
    pub style: &'static str,
    pub world: [f32; 2],
    pub map: [f32; 2],
}

/// One frame of player input, already mapped to the map's controls. Callers fill only what they read.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapInput {
    /// The cursor in view units (0..1, top left origin), `None` while it is outside the map.
    pub cursor: Option<[f32; 2]>,
    /// The pan this frame, in view units (a drag moves the picture with the pointer).
    pub drag: [f32; 2],
    /// Scroll lines this frame: positive zooms in.
    pub zoom_lines: f32,
    /// "Zoom All" was pressed: back to the whole picture.
    pub zoom_all: bool,
    pub next_filter: bool,
    pub previous_filter: bool,
    /// A left click: asks for GPS on the blip the cursor is on (the hovered/attached blip).
    pub click: bool,
    /// Enter: asks for GPS on the attached blip.
    pub accept: bool,
    /// Right click: the same as Enter.
    pub secondary: bool,
    /// Escape or the back button: unfocuses the blip, or closes the map.
    pub back: bool,
}

/// What the map does after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep the map open.
    Stay,
    /// Close the map.
    Close,
    /// The player asked for GPS on the focused blip at this index into the slice given to [`WorldMap::frame`]. The
    /// caller opens the game's `Dialog.fng` Yes/No confirm; Yes closes the map (setting the route is a later step).
    AskGps(usize),
}

/// The state of an open map. A new one is [`WorldMap::new`]: the Locations tab, the whole picture, nothing attached.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldMap {
    pub filter: Filter,
    pub camera: Camera,
    /// The blip the cursor is on (an index into the blips given to [`WorldMap::frame`]). Hovering the map over a blip
    /// of the filter focuses it; moving away unfocuses. The cursor snaps onto it (the "square"); the GPS confirm names it.
    pub attached: Option<usize>,
    /// The mouse's place in the view (0..1 on each axis over the picture), for drawing the free cross cursor. `None`
    /// off the window.
    pub cursor: Option<[f32; 2]>,
    /// The first row the themed legend shows: the scroll offset into the current filter's blips. Scrolling while the
    /// cursor is off the picture moves it; switching the tab resets it.
    pub list_scroll: usize,
}

impl Default for WorldMap {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldMap {
    pub fn new() -> Self {
        Self { filter: Filter::DEFAULT, camera: Camera::zoom_all(), attached: None, cursor: None, list_scroll: 0 }
    }

    /// The indices of the blips the current filter shows, in the order given. This is the right-side list.
    pub fn listed(&self, blips: &[PlacedBlip]) -> Vec<usize> {
        blips.iter().enumerate().filter(|(_, b)| self.filter.contains(b.style)).map(|(i, _)| i).collect()
    }

    /// One frame: zoom/pan or scroll the list, switch the tab, focus the blip under the cursor, and handle the GPS
    /// press and back. Hovering the picture over a blip of the filter focuses it (the cursor snaps onto it); a left
    /// click, Enter or a right click on the focused blip asks for GPS (the caller opens the Yes/No dialog).
    pub fn frame(&mut self, input: &MapInput, blips: &[PlacedBlip]) -> Outcome {
        self.cursor = input.cursor;
        let on_map = input.cursor.is_some_and(on_picture);
        self.move_or_scroll(input, on_map, blips);
        self.switch_filter(input);
        // Hovering the picture focuses the nearest blip of the filter; moving off a blip (still on the picture)
        // unfocuses. Off the picture (over the legend) the focus is left as it was.
        if let (true, Some(cursor)) = (on_map, input.cursor) {
            self.attached = self.pick(cursor, blips);
        }
        if input.back {
            return self.back();
        }
        if (input.click || input.accept || input.secondary)
            && let Some(index) = self.attached
        {
            return Outcome::AskGps(index);
        }
        Outcome::Stay
    }

    /// The scroll wheel zooms the map when the cursor is over the picture, and scrolls the themed legend's list when
    /// it is off it. A drag pans the picture. Zoom All (the Z key) always resets the camera.
    fn move_or_scroll(&mut self, input: &MapInput, on_map: bool, blips: &[PlacedBlip]) {
        if input.zoom_all {
            self.camera = Camera::zoom_all();
            return;
        }
        if on_map {
            self.camera.step(input.zoom_lines);
            self.camera.pan(input.drag);
            return;
        }
        self.scroll_list(input.zoom_lines, blips);
    }

    /// Scrolls the themed legend's list (the map's own FEng list is never scrolled). Wheel up shows earlier rows.
    fn scroll_list(&mut self, lines: f32, blips: &[PlacedBlip]) {
        let step = lines.round() as i32;
        if step == 0 {
            return;
        }
        let max = self.listed(blips).len().saturating_sub(VISIBLE_ROWS) as i32;
        self.list_scroll = (self.list_scroll as i32 - step).clamp(0, max) as usize;
    }

    /// A new tab unfocuses (it named a blip the tab may no longer show) and resets the list scroll.
    fn switch_filter(&mut self, input: &MapInput) {
        let next = if input.next_filter {
            self.filter.next()
        } else if input.previous_filter {
            self.filter.previous()
        } else {
            return;
        };
        self.filter = next;
        self.attached = None;
        self.list_scroll = 0;
    }

    /// The nearest blip of the filter to the cursor, as an index into `blips`.
    fn pick(&self, cursor: [f32; 2], blips: &[PlacedBlip]) -> Option<usize> {
        let shown = self.listed(blips);
        let views: Vec<[f32; 2]> = shown.iter().map(|&i| self.camera.to_view(blips[i].map)).collect();
        nearest(&views, cursor, PICK_RADIUS).map(|at| shown[at])
    }

    /// Back: unfocuses the blip, then closes the map.
    fn back(&mut self) -> Outcome {
        if self.attached.is_some() {
            self.attached = None;
            return Outcome::Stay;
        }
        Outcome::Close
    }
}

/// Whether a cursor (view units) is over the picture (0..1 on both axes); off it, it is over the chrome/legend.
fn on_picture(cursor: [f32; 2]) -> bool {
    cursor.iter().all(|v| (0.0..=1.0).contains(v))
}

/// The distance in metres from the player to `blip`, both in render space.
pub fn distance_metres(player: [f32; 2], blip: &PlacedBlip) -> f32 {
    let dx = blip.world[0] - player[0];
    let dy = blip.world[1] - player[1];
    (dx * dx + dy * dy).sqrt()
}
