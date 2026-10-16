//! Where the support cars stand (spec `ai-pursuit-support.md` sections 6 and 7; the same search in the formations spec
//! section 6). A heavy car is placed `400 - (60 + 10 k)` metres ahead of the target and a leader car `150 + 10 k` metres
//! behind, for the try `k` of the search. The try counter is kept across the cars of one request.
//! Distances are signed along the target's road: positive ahead, negative behind.

use glam::Vec2;

/// Spawn tries per car (the binary's loop bound).
pub const TRIES: u32 = 8;
/// A heavy car's first distance is this far ahead less the first step (400 m, the binary constant).
pub const HEAVY_FAR: f32 = 400.0;
/// A leader car's first distance is this far behind (150 m, the binary constant).
pub const LEADER_NEAR: f32 = 150.0;
/// The first try's step back from the heavy distance (60 m).
const HEAVY_FIRST_STEP: f32 = 60.0;
/// The step per try (10 m).
const STEP: f32 = 10.0;
/// The largest try index. The spec gives `k` as `0..7`; the cap is the reading of the spec's range and is unconfirmed
/// past the first car.
pub const LAST_K: u32 = 7;

/// The search for one request's cars: the try counter `k`. It is not reset between the cars, so a car starts its search
/// at the try the car before it was placed at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Search {
    k: u32,
}

impl Search {
    /// The signed distance of a heavy car's try `k`: `400 - (60 + 10 k)`, ahead of the target.
    pub fn heavy_distance(k: u32) -> f32 {
        HEAVY_FAR - (HEAVY_FIRST_STEP + STEP * k as f32)
    }

    /// The signed distance of a leader car's try `k`: `-(150 + 10 k)`, behind the target.
    pub fn leader_distance(k: u32) -> f32 {
        -(LEADER_NEAR + STEP * k as f32)
    }

    /// The try counter as it stands (the next car starts here).
    pub fn k(&self) -> u32 {
        self.k
    }

    /// Runs one car's search: `distance` gives a try's signed distance from `k`, and `at` tries a spot at that distance,
    /// returning what it placed or `None` when the spot is refused. Up to [`TRIES`] tries; a refused try moves `k` on by
    /// one (capped at [`LAST_K`]), and a placed one leaves `k` where it was.
    pub fn place<T>(&mut self, distance: fn(u32) -> f32, mut at: impl FnMut(f32) -> Option<T>) -> Option<T> {
        for _ in 0..TRIES {
            if let Some(placed) = at(distance(self.k)) {
                return Some(placed);
            }
            self.k = (self.k + 1).min(LAST_K);
        }
        None
    }
}

/// The pursuit offset of the leader car at `index` in the target's frame (`x` to the side, `y` ahead, metres): the cross
/// car sits 20 m behind the target's centre; the two V cars sit 24 m behind and 4 m to either side (spec section 5).
pub fn leader_offset(index: u32) -> Vec2 {
    match index {
        0 => Vec2::new(0.0, -20.0),
        1 => Vec2::new(-4.0, -24.0),
        _ => Vec2::new(4.0, -24.0),
    }
}

/// The point `distance` metres along `forward` from `target` (both in the map plane). A negative distance is behind.
pub fn point_along(target: Vec2, forward: Vec2, distance: f32) -> Vec2 {
    target + forward.normalize_or_zero() * distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_heavy_try_is_340_metres_ahead() {
        assert_eq!(Search::heavy_distance(0), 340.0);
    }

    #[test]
    fn the_heavy_tries_step_back_ten_metres_to_270() {
        let distances: Vec<f32> = (0..8).map(Search::heavy_distance).collect();
        assert_eq!(distances, vec![340.0, 330.0, 320.0, 310.0, 300.0, 290.0, 280.0, 270.0]);
    }

    #[test]
    fn the_leader_tries_step_out_behind_from_150_to_220() {
        assert_eq!(Search::leader_distance(0), -150.0);
        assert_eq!(Search::leader_distance(7), -220.0);
    }

    #[test]
    fn the_leader_offsets_are_the_pursuit_slots() {
        assert_eq!(leader_offset(0), Vec2::new(0.0, -20.0));
        assert_eq!(leader_offset(1), Vec2::new(-4.0, -24.0));
        assert_eq!(leader_offset(2), Vec2::new(4.0, -24.0));
    }

    #[test]
    fn a_placed_try_keeps_the_counter_for_the_next_car() {
        let mut search = Search::default();
        // The first car is placed on the third try (k = 2), so the second car starts at 2 too.
        let placed = search.place(Search::heavy_distance, |d| (d < 330.0).then_some(d));
        assert_eq!(placed, Some(320.0));
        assert_eq!(search.k(), 2);
        let second = search.place(Search::heavy_distance, |d| Some(d));
        assert_eq!(second, Some(320.0), "the second car's first try is where the first ended");
    }

    #[test]
    fn a_search_that_finds_nothing_takes_all_tries_and_stops_the_counter_at_seven() {
        let mut search = Search::default();
        let mut asked = Vec::new();
        let found: Option<()> = search.place(Search::leader_distance, |d| {
            asked.push(d);
            None
        });
        assert!(found.is_none());
        assert_eq!(asked.len(), TRIES as usize);
        assert_eq!(asked[0], -150.0);
        assert_eq!(asked[7], -220.0);
        assert_eq!(search.k(), LAST_K);
    }

    #[test]
    fn a_point_along_the_road_is_ahead_for_positive_and_behind_for_negative() {
        let target = Vec2::new(10.0, 20.0);
        assert_eq!(point_along(target, Vec2::Y, 340.0), Vec2::new(10.0, 360.0));
        assert_eq!(point_along(target, Vec2::Y, -150.0), Vec2::new(10.0, -130.0));
    }
}
