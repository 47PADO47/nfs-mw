//! The selection's glide (`docs/specs/custom-hud/menus.md` section 2): when the selection moves, its plate eases from
//! where it was to the new row over 150 ms. With motion reduction it jumps.
//!
//! A theme builds a new component every frame, so where each menu's plate is going is kept here, by slot, in a cell the
//! theme holds. A different list on the slot (another title) starts where it is, with no glide from the last screen.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::SlotId;

use crate::ui::theme::custom::tokens::ramp;

/// How long the plate takes to reach a new row, in seconds.
pub const GLIDE_SECS: f32 = 0.150;

/// Where one slot's plate is going: the list it belongs to, the y it left from and the y it goes to, and when it left.
#[derive(Clone, Debug, PartialEq)]
struct Track {
    title: String,
    from: f32,
    to: f32,
    since: f32,
}

impl Track {
    /// The y `time` seconds into the glide, eased out.
    fn at(&self, time: f32, duration: f32) -> f32 {
        let t = ramp(time - self.since, duration);
        let eased = 1.0 - (1.0 - t).powi(3);
        self.from + (self.to - self.from) * eased
    }
}

/// The plates' tracks by slot. Clones share one map.
#[derive(Clone, Debug, Default)]
pub struct GlideCell {
    tracks: Arc<Mutex<HashMap<SlotId, Track>>>,
}

impl GlideCell {
    /// The y to draw `slot`'s selection plate at, at `time` on the host clock, when the selected row's middle is at
    /// `target` in the list titled `title`. A new target starts a glide of `duration` seconds from where the plate is
    /// now; a new list, or a duration of 0 (motion reduction), puts it at the target at once.
    pub fn follow(&self, slot: SlotId, title: &str, target: f32, time: f32, duration: f32) -> f32 {
        let mut tracks = self.lock();
        let still = Track { title: title.to_owned(), from: target, to: target, since: time };
        let Some(track) = tracks.get_mut(&slot) else {
            tracks.insert(slot, still);
            return target;
        };
        if track.title != title || duration <= 0.0 {
            *track = still;
            return target;
        }
        if track.to != target {
            let now = track.at(time, duration);
            *track = Track { from: now, ..still };
        }
        track.at(time, duration)
    }

    /// Forgets `slot`'s plate, so the next list on it starts in place.
    pub fn forget(&self, slot: SlotId) {
        self.lock().remove(&slot);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<SlotId, Track>> {
        self.tracks.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOT: SlotId = SlotId::new(20);

    #[test]
    fn the_first_sight_of_a_list_puts_the_plate_on_its_row() {
        let cell = GlideCell::default();
        assert_eq!(cell.follow(SLOT, "Main", 10.0, 0.0, GLIDE_SECS), 10.0);
        assert_eq!(cell.follow(SLOT, "Main", 10.0, 5.0, GLIDE_SECS), 10.0, "a still selection stays");
    }

    #[test]
    fn a_new_row_eases_over_the_glide_and_arrives() {
        let cell = GlideCell::default();
        cell.follow(SLOT, "Main", 0.0, 1.0, GLIDE_SECS);
        assert_eq!(cell.follow(SLOT, "Main", 30.0, 2.0, GLIDE_SECS), 0.0, "it leaves from the old row");
        let mid = cell.follow(SLOT, "Main", 30.0, 2.0 + GLIDE_SECS / 2.0, GLIDE_SECS);
        assert!(mid > 15.0 && mid < 30.0, "eased out: past halfway at half time ({mid})");
        assert_eq!(cell.follow(SLOT, "Main", 30.0, 2.0 + GLIDE_SECS, GLIDE_SECS), 30.0);
    }

    #[test]
    fn motion_reduction_and_a_new_list_jump_to_the_row() {
        let cell = GlideCell::default();
        cell.follow(SLOT, "Main", 0.0, 0.0, GLIDE_SECS);
        assert_eq!(cell.follow(SLOT, "Main", 30.0, 1.0, 0.0), 30.0, "no glide with motion reduction");
        assert_eq!(cell.follow(SLOT, "Options", 60.0, 2.0, GLIDE_SECS), 60.0, "another list starts in place");
        cell.forget(SLOT);
        assert_eq!(cell.follow(SLOT, "Options", 90.0, 3.0, GLIDE_SECS), 90.0, "a forgotten slot starts in place");
    }

    #[test]
    fn a_change_of_mind_starts_from_where_the_plate_is() {
        let cell = GlideCell::default();
        cell.follow(SLOT, "Main", 0.0, 0.0, GLIDE_SECS);
        cell.follow(SLOT, "Main", 30.0, 1.0, GLIDE_SECS);
        let there = cell.follow(SLOT, "Main", 30.0, 1.0 + GLIDE_SECS / 3.0, GLIDE_SECS);
        let back = cell.follow(SLOT, "Main", 0.0, 1.0 + GLIDE_SECS / 3.0, GLIDE_SECS);
        assert_eq!(back, there, "the new glide starts where the plate was, with no jump");
    }
}
