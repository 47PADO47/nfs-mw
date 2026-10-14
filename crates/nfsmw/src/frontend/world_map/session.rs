//! One open map: the filter tab, the camera, the blip the cursor is attached to and the "Activate GPS?" prompt.
//! [`WorldMap::frame`] takes one frame of input and says whether the map stays, closes, or sets a GPS route.

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
    /// Enter: prompts to set GPS for the attached blip, or confirms the prompt.
    pub accept: bool,
    /// Right click: prompts to set GPS for the attached blip (the same as Enter).
    pub secondary: bool,
    /// Escape or the back button: cancels the prompt, or closes the map.
    pub back: bool,
}

/// What the map does after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep the map open.
    Stay,
    /// Close the map (back, with no prompt open).
    Close,
    /// The player confirmed GPS for the blip at this index into the slice given to [`WorldMap::frame`]. The caller
    /// sets the route (`activate_gps`, a stub) and closes the map.
    ActivateGps(usize),
}

/// The state of an open map. A new one is [`WorldMap::new`]: the Locations tab, the whole picture, nothing attached.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldMap {
    pub filter: Filter,
    pub camera: Camera,
    /// The blip the cursor is on (an index into the blips given to [`WorldMap::frame`]). Frozen while the prompt is
    /// open, so the prompt keeps naming the blip it asked about.
    pub attached: Option<usize>,
    /// The "Activate GPS?" prompt is open.
    pub prompt: bool,
}

impl Default for WorldMap {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldMap {
    pub fn new() -> Self {
        Self { filter: Filter::DEFAULT, camera: Camera::zoom_all(), attached: None, prompt: false }
    }

    /// The indices of the blips the current filter shows, in the order given. This is the right-side list.
    pub fn listed(&self, blips: &[PlacedBlip]) -> Vec<usize> {
        blips.iter().enumerate().filter(|(_, b)| self.filter.contains(b.style)).map(|(i, _)| i).collect()
    }

    /// One frame: zoom and pan, switch the tab, attach the cursor, and handle the prompt and back.
    pub fn frame(&mut self, input: &MapInput, blips: &[PlacedBlip]) -> Outcome {
        self.move_camera(input);
        self.switch_filter(input);
        if let Some(cursor) = input.cursor.filter(|_| !self.prompt) {
            self.attached = self.pick(cursor, blips);
        }
        if input.back {
            return self.back();
        }
        if input.accept || input.secondary {
            return self.confirm(input.accept);
        }
        Outcome::Stay
    }

    fn move_camera(&mut self, input: &MapInput) {
        if input.zoom_all {
            self.camera = Camera::zoom_all();
            return;
        }
        self.camera.step(input.zoom_lines);
        self.camera.pan(input.drag);
    }

    /// A new tab drops the prompt: it named a blip the tab may no longer show.
    fn switch_filter(&mut self, input: &MapInput) {
        let next = if input.next_filter {
            self.filter.next()
        } else if input.previous_filter {
            self.filter.previous()
        } else {
            return;
        };
        self.filter = next;
        self.prompt = false;
        self.attached = None;
    }

    /// The nearest blip of the filter to the cursor, as an index into `blips`.
    fn pick(&self, cursor: [f32; 2], blips: &[PlacedBlip]) -> Option<usize> {
        let shown = self.listed(blips);
        let views: Vec<[f32; 2]> = shown.iter().map(|&i| self.camera.to_view(blips[i].map)).collect();
        nearest(&views, cursor, PICK_RADIUS).map(|at| shown[at])
    }

    fn back(&mut self) -> Outcome {
        if self.prompt {
            self.prompt = false;
            return Outcome::Stay;
        }
        Outcome::Close
    }

    /// Enter or right click opens the prompt for the attached blip; Enter with the prompt open confirms it.
    fn confirm(&mut self, accept: bool) -> Outcome {
        if self.prompt {
            return match (accept, self.attached) {
                (true, Some(index)) => Outcome::ActivateGps(index),
                _ => Outcome::Stay,
            };
        }
        if self.attached.is_none() {
            return Outcome::Stay;
        }
        self.prompt = true;
        Outcome::Stay
    }
}

/// The distance in metres from the player to `blip`, both in render space.
pub fn distance_metres(player: [f32; 2], blip: &PlacedBlip) -> f32 {
    let dx = blip.world[0] - player[0];
    let dy = blip.world[1] - player[1];
    (dx * dx + dy * dy).sqrt()
}
