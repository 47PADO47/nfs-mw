//! The live screen of a bike's instrument cluster (`docs/modding/ride6-bike-dash.md`): the importer stores the cluster's picture as an
//! uncompressed texture of the car's pack and names the display in it (`[bike.dash]`); a few times a second the speed, the gear and the
//! revs are painted into that window on the CPU and only that region is written to the texture (`RenderBackend::update_texture`).
//! Nothing is painted while nothing changed, and a backend that cannot write a texture in place leaves the baked picture.

use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::mods::bike_dash::{self, Colors, Dash};

use super::CarRig;

/// How often the screen may be repainted: the cluster's picture does not need the frame rate.
pub const MIN_INTERVAL: Duration = Duration::from_millis(66);
/// The segments of the rev bar.
const SEGMENTS: u32 = 24;

/// What the screen shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashState {
    pub speed_kmh: f32,
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    pub rpm: f32,
    /// The rpm of the full bar.
    pub rpm_max: f32,
}

impl DashState {
    /// The bike at rest: neutral, nothing on the bar.
    pub const REST: Self = Self { speed_kmh: 0.0, gear: 0, rpm: 0.0, rpm_max: 10_000.0 };

    /// What a repaint would draw: the speed in whole km/h, the gear and the lit segments. A state with the same key paints the same picture.
    fn key(&self) -> (u32, i32, u32) {
        let share = (self.rpm / self.rpm_max.max(1.0)).clamp(0.0, 1.0);
        (self.speed_kmh.abs().round() as u32, self.gear, (share * SEGMENTS as f32).ceil() as u32)
    }
}

/// The screen of one rig: which texture, and what was painted last.
pub(super) struct DashFeed {
    dash: Arc<Dash>,
    last_time: Cell<Option<Instant>>,
    last_key: Cell<Option<(u32, i32, u32)>>,
    /// The backend refused a write (it cannot, or the texture is not 32-bit colour): the baked picture stays.
    refused: Cell<bool>,
}

impl DashFeed {
    /// The feed of the bike in `folder`, if its manifest has a `[bike.dash]`.
    pub(super) fn of(folder: &str) -> Option<Self> {
        let dash = bike_dash::of(folder)?;
        Some(Self { dash, last_time: Cell::new(None), last_key: Cell::new(None), refused: Cell::new(false) })
    }
}

impl CarRig {
    /// Paint `state` into the cluster's screen if the bike has one, the screen is due and something changed. Returns the microseconds the
    /// painting and the upload took when it repainted (`None` otherwise), for the measurement of its cost.
    pub fn feed_dash(&self, renderer: &mut dyn RenderBackend, state: &DashState, now: Instant) -> Option<u128> {
        let feed = self.dash.as_ref()?;
        if feed.refused.get() || feed.last_time.get().is_some_and(|last| now.duration_since(last) < MIN_INTERVAL) {
            return None;
        }
        let key = state.key();
        if feed.last_key.get() == Some(key) {
            return None;
        }
        let texture = self.appearance.borrow().materials.texture(feed.dash.texture_hash)?;
        let [x, y, width, height] = feed.dash.window;
        let started = Instant::now();
        let picture = paint(width, height, state, &feed.dash.colors);
        if !renderer.update_texture(texture, [x, y, width, height], &picture) {
            feed.refused.set(true);
            log::debug!("the cluster of {} cannot be written: the baked picture stays", self.model.name);
            return None;
        }
        feed.last_time.set(Some(now));
        feed.last_key.set(Some(key));
        Some(started.elapsed().as_micros())
    }

    /// Put the screen back to a bike at rest (the live screen switched off): the next repaint draws it.
    pub fn rest_dash(&self, renderer: &mut dyn RenderBackend, now: Instant) {
        let Some(feed) = self.dash.as_ref() else { return };
        if feed.last_key.get().is_none() {
            return;
        }
        feed.last_time.set(None);
        self.feed_dash(renderer, &DashState::REST, now);
        feed.last_key.set(None);
    }
}

/// The glyphs of the panel: five columns by seven rows, a bit per column from the left (bit 4).
fn glyph(c: char) -> Option<[u8; 7]> {
    let rows = match c {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        _ => return None,
    };
    Some(rows)
}

/// An RGBA buffer of `width` by `height` texels.
struct Canvas {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Canvas {
    fn fill(&mut self, (x, y, width, height): (u32, u32, u32, u32), [r, g, b]: [u8; 3]) {
        for row in y..(y + height).min(self.height) {
            for column in x..(x + width).min(self.width) {
                let at = ((row * self.width + column) * 4) as usize;
                self.rgba[at..at + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }

    /// `text` with its top left corner at `(x, y)`, the glyphs `unit` texels per dot.
    fn text(&mut self, (x, y): (u32, u32), unit: u32, colour: [u8; 3], text: &str) {
        let mut at = x;
        for c in text.chars() {
            let Some(rows) = glyph(c) else { continue };
            for (row, bits) in rows.iter().enumerate() {
                for column in 0..5u32 {
                    if (*bits >> (4 - column)) & 1 == 1 {
                        self.fill((at + column * unit, y + row as u32 * unit, unit, unit), colour);
                    }
                }
            }
            at += 6 * unit;
        }
    }
}

/// The width of `text` in dots of `unit` texels, without the gap after the last glyph.
fn text_width(unit: u32, text: &str) -> u32 {
    let glyphs = text.chars().filter(|&c| glyph(c).is_some()).count() as u32;
    (glyphs * 6).saturating_sub(1) * unit
}

/// The generic panel: the rev bar along the top, the gear on the left and the speed on the right with its unit under it. The whole
/// buffer, the size of the display, is painted.
fn paint(width: u32, height: u32, state: &DashState, colors: &Colors) -> Vec<u8> {
    let mut canvas = Canvas { width, height, rgba: vec![255; (width * height * 4) as usize] };
    canvas.fill((0, 0, width, height), colors.background);
    let lit = state.key().2;
    let (bar_y, bar_h) = (height / 12, height / 9);
    let pitch = (width * 9 / 10 / SEGMENTS).max(1);
    let bar_x = width / 20;
    for segment in 0..SEGMENTS {
        let colour = if segment < lit { colors.lit } else { colors.dim };
        canvas.fill((bar_x + segment * pitch, bar_y, (pitch * 3 / 4).max(1), bar_h), colour);
    }
    let unit = (height / 3 / 7).max(1);
    let top = height * 3 / 10;
    let gear = match state.gear {
        -1 => "R".to_owned(),
        0 => "N".to_owned(),
        n => n.to_string(),
    };
    canvas.text((width / 12, top), unit, colors.gear, &gear);
    let speed = (state.speed_kmh.abs().round().min(999.0) as u32).to_string();
    let speed_x = (width - width / 12).saturating_sub(text_width(unit, &speed));
    canvas.text((speed_x, top), unit, colors.text, &speed);
    let small = (unit / 3).max(1);
    let label_x = (width - width / 12).saturating_sub(text_width(small, "KM/H"));
    canvas.text((label_x, top + 7 * unit + 2 * small), small, colors.label, "KM/H");
    canvas.rgba
}

#[cfg(test)]
#[path = "dash_tests.rs"]
mod tests;
