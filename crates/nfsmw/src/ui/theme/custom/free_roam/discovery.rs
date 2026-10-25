//! The discovery chips' memory (`docs/specs/custom-hud/modes.md` section 2): which places near the car are announced,
//! and which were announced already, so a place is never announced twice.
//!
//! A place is a candidate when the player can go to it (`NavMarker::discovered`: an open race, a shop, a safe house)
//! and the car is within [`RADIUS_M`] of it. The nearest candidates not announced before come up as chips, at most
//! [`MAX_CHIPS`] at a time. A chip stays while the car stays near, its distance updated each frame, and leaves (fading
//! out) when the car moves away or reaches the place (the entry card takes over then). Once a place has had a chip it
//! is remembered for the session and never comes up again.

use std::collections::HashSet;

use crate::hud::{NavMarker, NavMarkerKind};
use crate::ui::theme::custom::tokens::{Motion, ramp};

/// How near a place has to be for its chip, metres.
pub const RADIUS_M: f32 = 300.0;
/// The most chips up at once: the nearest one or two places.
pub const MAX_CHIPS: usize = 2;

/// A place, as the memory knows it: its spot to the metre and its name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PlaceKey {
    at: [i32; 2],
    name: Option<String>,
}

impl PlaceKey {
    fn of(marker: &NavMarker) -> Self {
        Self { at: marker.position.map(|v| v.round() as i32), name: marker.name.clone() }
    }
}

/// One announced place.
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    pub kind: NavMarkerKind,
    /// The career event id of a race; `None` for a place its kind names.
    pub name: Option<String>,
    /// Metres from the car now (as of the frame it last stayed).
    pub distance_m: f32,
    /// When the chip came up, elapsed seconds.
    pub since: f32,
    /// When the chip began to leave; `None` while it stays.
    pub left: Option<f32>,
    key: PlaceKey,
}

impl Chip {
    /// The chip's fade at `now`: in over `fade_in`, out over `fade_out` once it leaves.
    pub fn opacity(&self, now: f32, motion: &Motion) -> f32 {
        let shown = ramp(now - self.since, motion.fade_in);
        let leaving = self.left.map_or(1.0, |left| 1.0 - ramp(now - left, motion.fade_out));
        shown.min(leaving)
    }
}

/// The chips up and the places announced so far.
#[derive(Clone, Debug, Default)]
pub struct Discovery {
    chips: Vec<Chip>,
    announced: HashSet<PlaceKey>,
}

impl Discovery {
    /// Takes the frame at `now`: the places on the map, the car at `player`, and whether the car has reached a place
    /// (the entry card is up). Outside free roam the caller passes no places, so every chip leaves. A chip that left is
    /// dropped `fade_out` seconds later.
    pub fn update(&mut self, markers: &[NavMarker], player: Option<[f32; 2]>, reached: bool, now: f32, fade_out: f32) {
        let near = nearest(markers, player);
        self.chips.retain(|chip| chip.left.is_none_or(|left| now - left < fade_out));
        for chip in self.chips.iter_mut().filter(|chip| chip.left.is_none()) {
            let stays = near.iter().find(|(_, marker)| PlaceKey::of(marker) == chip.key).map(|(d, _)| *d);
            match stays.filter(|_| !reached) {
                Some(distance) => chip.distance_m = distance,
                None => chip.left = Some(now),
            }
        }
        if reached {
            return;
        }
        let room = MAX_CHIPS.saturating_sub(self.chips.iter().filter(|chip| chip.left.is_none()).count());
        let fresh: Vec<(f32, &NavMarker)> =
            near.into_iter().filter(|(_, marker)| !self.announced.contains(&PlaceKey::of(marker))).take(room).collect();
        for (distance, marker) in fresh {
            let key = PlaceKey::of(marker);
            self.announced.insert(key.clone());
            let chip = Chip {
                kind: marker.kind,
                name: marker.name.clone(),
                distance_m: distance,
                since: now,
                left: None,
                key,
            };
            self.chips.push(chip);
        }
    }

    /// The chips up, staying or leaving, in the order they came up.
    pub fn chips(&self) -> &[Chip] {
        &self.chips
    }
}

/// The places the player can go to within [`RADIUS_M`] of `player`, nearest first, with their distances.
fn nearest(markers: &[NavMarker], player: Option<[f32; 2]>) -> Vec<(f32, &NavMarker)> {
    let Some(at) = player else { return Vec::new() };
    let mut near: Vec<(f32, &NavMarker)> = markers
        .iter()
        .filter(|marker| marker.discovered)
        .map(|marker| ((marker.position[0] - at[0]).hypot(marker.position[1] - at[1]), marker))
        .filter(|(distance, _)| *distance <= RADIUS_M)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near
}
