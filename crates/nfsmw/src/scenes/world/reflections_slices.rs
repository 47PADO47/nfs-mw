//! Which cube faces one update draws. A refresh is spread over a few updates, a slice of faces at a time, so no frame
//! pays for all six scene passes; a restart or a jump takes over the faces that matter first.
use blackbox_gfx::ReflectionQuality;
use blackbox_gfx::reflection::ALL_FACES;
use glam::Vec3;

use super::cadence;

const FACES: usize = 6;

/// A move this far since the refresh began is a teleport or a reset, not driving (a car covers a few metres in the
/// slowest refresh interval): the old cube shows another place, so every face is drawn at once.
const JUMP_DISTANCE: f32 = 30.0;

/// The faces still to draw in the refresh under way, and where the next slice starts. Restarting a refresh part way
/// keeps the cursor, so a refresh that keeps being restarted (streaming) still reaches every face in turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Rotation {
    pending: u8,
    cursor: u8,
}

impl Rotation {
    /// Whether no refresh is under way.
    pub(super) fn idle(&self) -> bool {
        self.pending == 0
    }

    /// Starts a refresh of all six faces.
    pub(super) fn restart(&mut self) {
        self.pending = ALL_FACES;
    }

    /// The next `count` pending faces, in rotation from the cursor, as a face mask. They are no longer pending.
    pub(super) fn take(&mut self, count: u32) -> u8 {
        let mut mask = 0u8;
        let start = usize::from(self.cursor);
        for step in 0..FACES {
            if mask.count_ones() >= count {
                break;
            }
            let face = (start + step) % FACES;
            let bit = 1u8 << face;
            if self.pending & bit == 0 {
                continue;
            }
            mask |= bit;
            self.cursor = ((face + 1) % FACES) as u8;
        }
        self.pending &= !mask;
        mask
    }
}

/// How many faces one update draws. The cached mode keeps its whole-capture semantics and rate.
pub(super) fn faces_per_update(cached: bool, quality: ReflectionQuality) -> u32 {
    match (cached, quality) {
        (true, _) | (_, ReflectionQuality::Off) => FACES as u32,
        (false, ReflectionQuality::High) => 2,
        (false, ReflectionQuality::Medium | ReflectionQuality::Low) => 3,
    }
}

/// Whether the car went somewhere else since the refresh began: the clock went back, or it moved too far.
pub(super) fn jumped(started: f32, now: f32, distance_squared: f32) -> bool {
    now < started || distance_squared >= JUMP_DISTANCE * JUMP_DISTANCE
}

/// Where the refresh under way began, which the cadence and the jump test measure from, and what is left of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Progress {
    pub seconds: f32,
    pub origin: Vec3,
    pub rotation: Rotation,
}

/// What an update knows about the cube and the car.
pub(super) struct Inputs {
    pub cached: bool,
    pub quality: ReflectionQuality,
    pub now: f32,
    pub origin: Vec3,
    /// The previous capture has this cube's size, quality and mode, so its faces are still usable.
    pub same_cube: bool,
    /// What the faces show changed (scenery, lighting, the reflection settings): every face is due again.
    pub content_changed: bool,
}

/// The progress after this update and the faces to draw in it, or `None` to keep the cube as it is.
pub(super) fn plan(previous: Option<Progress>, i: &Inputs) -> Option<(Progress, u8)> {
    let began = |rotation| Progress { seconds: i.now, origin: i.origin, rotation };
    let whole = (began(Rotation::default()), ALL_FACES);
    let Some(last) = previous.filter(|_| i.same_cube) else { return Some(whole) };
    let moved = last.origin.distance_squared(i.origin);
    if jumped(last.seconds, i.now, moved) {
        return Some(whole);
    }
    let mut progress = last;
    let due =
        i.content_changed || (last.rotation.idle() && !cadence::reuse(i.cached, i.quality, last.seconds, i.now, moved));
    if due {
        progress = began(last.rotation);
        progress.rotation.restart();
    }
    if progress.rotation.idle() {
        return None;
    }
    let faces = progress.rotation.take(faces_per_update(i.cached, i.quality));
    Some((progress, faces))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_cycle_covers_every_face_exactly_once() {
        for (cached, quality) in [
            (false, ReflectionQuality::High),
            (false, ReflectionQuality::Medium),
            (false, ReflectionQuality::Low),
            (true, ReflectionQuality::High),
        ] {
            let mut rotation = Rotation::default();
            assert!(rotation.idle());
            rotation.restart();
            let per_update = faces_per_update(cached, quality);
            let mut seen = 0u8;
            let mut updates = 0;
            while !rotation.idle() {
                let mask = rotation.take(per_update);
                assert_ne!(mask, 0);
                assert!(mask.count_ones() <= per_update);
                assert_eq!(seen & mask, 0, "{quality:?}: a face was drawn twice in one cycle");
                seen |= mask;
                updates += 1;
            }
            assert_eq!(seen, ALL_FACES, "{quality:?}: every face is drawn");
            assert_eq!(updates, FACES as u32 / per_update);
        }
    }

    #[test]
    fn high_quality_draws_two_faces_so_the_cube_completes_in_three_updates() {
        assert_eq!(faces_per_update(false, ReflectionQuality::High), 2);
        let mut rotation = Rotation::default();
        let mut covered = Vec::new();
        for _ in 0..6 {
            if rotation.idle() {
                rotation.restart();
            }
            covered.push(rotation.take(faces_per_update(false, ReflectionQuality::High)));
        }
        assert_eq!(covered[0] | covered[1] | covered[2], ALL_FACES);
        assert_eq!(covered[3] | covered[4] | covered[5], ALL_FACES);
        assert_eq!(covered[0].count_ones(), 2);
    }

    #[test]
    fn the_cached_mode_draws_the_whole_cube_at_once() {
        let mut rotation = Rotation::default();
        rotation.restart();
        assert_eq!(rotation.take(faces_per_update(true, ReflectionQuality::Medium)), ALL_FACES);
        assert!(rotation.idle());
    }

    #[test]
    fn a_refresh_that_keeps_restarting_still_reaches_every_face() {
        let mut rotation = Rotation::default();
        let mut seen = 0u8;
        for _ in 0..3 {
            rotation.restart();
            seen |= rotation.take(2);
        }
        assert_eq!(seen, ALL_FACES, "the cursor moves on instead of redrawing the first faces");
    }

    #[test]
    fn a_restart_part_way_through_hands_out_all_six_again() {
        let mut rotation = Rotation::default();
        rotation.restart();
        rotation.take(2);
        rotation.restart();
        let mut seen = 0u8;
        while !rotation.idle() {
            seen |= rotation.take(2);
        }
        assert_eq!(seen, ALL_FACES);
    }

    #[test]
    fn a_jump_or_a_clock_reset_is_not_driving() {
        assert!(!jumped(1.0, 1.05, 4.0), "a couple of metres in an update");
        assert!(!jumped(1.0, 1.0, 0.0), "paused");
        assert!(jumped(1.0, 1.05, 31.0 * 31.0), "a teleport");
        assert!(jumped(1.0, 0.5, 0.0), "the clock went back");
    }

    fn inputs(now: f32, origin: Vec3) -> Inputs {
        Inputs { cached: false, quality: ReflectionQuality::High, now, origin, same_cube: true, content_changed: false }
    }

    #[test]
    fn the_first_update_and_a_changed_cube_draw_every_face() {
        let (progress, faces) = plan(None, &inputs(0.0, Vec3::ZERO)).unwrap();
        assert_eq!(faces, ALL_FACES);
        assert!(progress.rotation.idle());
        let other = Inputs { same_cube: false, ..inputs(0.1, Vec3::ZERO) };
        assert_eq!(plan(Some(progress), &other).unwrap().1, ALL_FACES);
    }

    #[test]
    fn high_quality_rotates_two_faces_an_update_through_the_whole_cube() {
        let mut progress = plan(None, &inputs(0.0, Vec3::ZERO)).map(|p| p.0);
        let mut masks = Vec::new();
        for frame in 1..=12 {
            let now = frame as f32 / 60.0;
            let (next, faces) = plan(progress, &inputs(now, Vec3::new(frame as f32 * 0.5, 0.0, 0.0))).unwrap();
            assert_eq!(faces.count_ones(), 2);
            masks.push(faces);
            progress = Some(next);
        }
        for cycle in masks.chunks(3) {
            assert_eq!(cycle[0] & cycle[1] | cycle[0] & cycle[2] | cycle[1] & cycle[2], 0, "no face twice");
            assert_eq!(cycle[0] | cycle[1] | cycle[2], ALL_FACES, "every face once");
        }
    }

    #[test]
    fn a_jump_or_a_clock_reset_refreshes_all_six_part_way_through_a_cycle() {
        let (started, _) = plan(None, &inputs(1.0, Vec3::ZERO)).unwrap();
        let (mid, faces) = plan(Some(started), &inputs(1.02, Vec3::X)).unwrap();
        assert_eq!(faces.count_ones(), 2);
        assert!(!mid.rotation.idle());
        let (jumped, faces) = plan(Some(mid), &inputs(1.03, Vec3::new(200.0, 0.0, 0.0))).unwrap();
        assert_eq!(faces, ALL_FACES);
        assert!(jumped.rotation.idle(), "the old cycle is dropped");
        let (reset, faces) = plan(Some(mid), &inputs(0.2, Vec3::X)).unwrap();
        assert_eq!(faces, ALL_FACES);
        assert_eq!(reset.seconds, 0.2);
    }

    #[test]
    fn a_slow_cadence_spreads_a_refresh_then_waits_for_the_next() {
        let low = |now: f32| Inputs { quality: ReflectionQuality::Low, ..inputs(now, Vec3::ZERO) };
        let (whole, _) = plan(None, &low(0.0)).unwrap();
        assert!(plan(Some(whole), &low(0.03)).is_none(), "15 Hz: the cube is kept between refreshes");
        let (begun, faces) = plan(Some(whole), &low(0.07)).unwrap();
        assert_eq!(faces.count_ones(), 3, "a refresh begins with a slice");
        let (done, rest) = plan(Some(begun), &low(0.072)).unwrap();
        assert_eq!(faces | rest, ALL_FACES);
        assert_eq!(faces & rest, 0);
        assert!(done.rotation.idle());
        assert!(plan(Some(done), &low(0.1)).is_none(), "the next refresh is due a full interval after it began");
        assert!(plan(Some(done), &low(0.14)).is_some());
    }

    #[test]
    fn changed_content_restarts_the_refresh_from_the_next_face() {
        let (started, first) = plan(None, &inputs(0.0, Vec3::ZERO)).unwrap();
        assert_eq!(first, ALL_FACES);
        let changed = Inputs { content_changed: true, ..inputs(0.016, Vec3::ZERO) };
        let (progress, faces) = plan(Some(started), &changed).unwrap();
        assert_eq!(faces.count_ones(), 2);
        assert!(!progress.rotation.idle());
        let changed = Inputs { content_changed: true, ..inputs(0.032, Vec3::ZERO) };
        let (_, more) = plan(Some(progress), &changed).unwrap();
        assert_eq!(faces & more, 0, "a refresh that keeps restarting moves on to the other faces");
    }

    #[test]
    fn the_cached_mode_refreshes_whole_captures_at_its_own_rate() {
        let cached = |now: f32| Inputs { cached: true, ..inputs(now, Vec3::ZERO) };
        let (progress, _) = plan(None, &cached(0.0)).unwrap();
        assert!(plan(Some(progress), &cached(0.001)).is_none());
        let (progress, faces) = plan(Some(progress), &cached(0.02)).unwrap();
        assert_eq!(faces, ALL_FACES);
        assert!(progress.rotation.idle());
    }
}
