//! The cop manager's spawning gate: the lockout timer and the track's no-cop-spawn zones.
//! Spec: `docs/specs/ai-pursuit-cops.md` (sections 1 and 3.2), `docs/formats/pursuit-data.md` section 5.
//!
//! While the lockout runs no cop is spawned and every active one is removed. It is 60 s when a session starts and
//! 120 s (the free-roam `TimeInactiveFor911`) after the player evades or pays a ticket. Starting a chase clears it.

use blackbox_roads::{TrackZone, to_zone_space};
use glam::Vec3;

pub(super) mod requests;
pub(super) mod roam;

/// The lockout when the session starts (`kCopLockoutTime`).
pub(super) const LOCKOUT_START: f32 = 60.0;
/// The lockout after an evade or a restart in free roam (`TimeInactiveFor911` of the `default` pursuit level row).
pub(super) const LOCKOUT_FREE_ROAM: f32 = 120.0;
/// The track zone kind that bars a cop spawn point (`NO_COP_SPAWN`, `docs/formats/pursuit-data.md` section 5).
pub(super) const NO_COP_SPAWN_ZONE: u32 = 13;

/// The lockout timer and the zones that bar a cop spawn point.
#[derive(Default)]
pub(super) struct CopManager {
    lockout: f32,
    no_spawn: Vec<TrackZone>,
    /// The free-roam 911 call (`roam.rs`).
    pub roam: roam::Roam911,
    /// The camera eye the spawn window is measured from, physics space (`None` until the world gives it).
    eye: Option<Vec3>,
    /// The cop spawn requests that wait to be placed (`requests.rs`).
    pub requests: requests::Queue,
}

impl CopManager {
    /// Starts the session's lockout (the game's load path; a bare world used in tests has none).
    pub fn start_session(&mut self) {
        self.lockout = LOCKOUT_START;
    }

    /// Advances the timer by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if dt > 0.0 && dt.is_finite() {
            self.lockout = (self.lockout - dt).max(0.0);
        }
    }

    /// Whether cops may spawn and stay (`VehicleSpawningEnabled`): false during the lockout.
    pub fn spawning_enabled(&self) -> bool {
        self.lockout <= 0.0
    }

    /// Locks the cops out for the free-roam time after an evade, or a ticket paid (`LockoutCops(true)`).
    pub fn lock_out(&mut self) {
        self.lockout = LOCKOUT_FREE_ROAM;
        self.roam.restart();
    }

    /// Clears the lockout: a chase starts (`LockoutCops(false)`).
    pub fn unlock(&mut self) {
        self.lockout = 0.0;
    }

    /// Gives the camera eye the world's view has this tick (physics space): the spawn window of `CheckSpawnPosition` and
    /// `RespawnAvailable` is measured from the camera (`AISpawnManager::GetBasePosition`), not from the player.
    pub fn set_eye(&mut self, eye: Vec3) {
        self.eye = Some(eye).filter(|eye| eye.is_finite());
    }

    /// The point the spawn window is measured from: the camera eye, or `fallback` (the player) before the world gives one.
    pub fn base(&self, fallback: Vec3) -> Vec3 {
        self.eye.unwrap_or(fallback)
    }

    /// The horizontal distance from the spawn window's base to `at`.
    pub fn window_gap(&self, at: Vec3, fallback: Vec3) -> f32 {
        let base = self.base(fallback);
        (at.x - base.x).hypot(at.z - base.z)
    }

    /// Seconds of lockout left.
    pub fn lockout_left(&self) -> f32 {
        self.lockout
    }

    /// The track's no-cop-spawn zones, read once per track.
    pub fn set_no_spawn_zones(&mut self, zones: Vec<TrackZone>) {
        self.no_spawn = zones;
    }

    /// Whether a physics-space point lies in a zone that bars a cop spawn (`CheckSpawnPosition`). The zones have no
    /// height test: a zone is its outline.
    pub fn bars_spawn(&self, at: Vec3) -> bool {
        let point = to_zone_space(at);
        self.no_spawn.iter().any(|zone| zone.contains(point))
    }
}

/// A square no-cop-spawn zone of half side `half` around the zone-space origin, for the tests that bar a spawn.
#[cfg(test)]
pub(super) fn square_zone(half: f32) -> TrackZone {
    use glam::Vec2;
    TrackZone {
        kind: NO_COP_SPAWN_ZONE,
        position: Vec2::ZERO,
        direction: Vec2::ZERO,
        elevation: 0.0,
        visit_info: 0,
        data: [0; 4],
        bbox_min: Vec2::splat(-half),
        bbox_max: Vec2::splat(half),
        polygon: vec![Vec2::new(-half, -half), Vec2::new(half, -half), Vec2::new(half, half), Vec2::new(-half, half)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_around_zone_origin() -> TrackZone {
        square_zone(10.0)
    }

    #[test]
    fn a_bare_manager_spawns_and_a_session_starts_locked_out() {
        let mut cops = CopManager::default();
        assert!(cops.spawning_enabled());
        cops.start_session();
        assert!(!cops.spawning_enabled());
        cops.tick(LOCKOUT_START - 0.5);
        assert!(!cops.spawning_enabled(), "not yet");
        cops.tick(0.6);
        assert!(cops.spawning_enabled(), "the 60 s are over");
    }

    #[test]
    fn an_evade_locks_out_for_the_free_roam_time_and_a_chase_clears_it() {
        let mut cops = CopManager::default();
        cops.lock_out();
        cops.tick(119.0);
        assert!(!cops.spawning_enabled());
        cops.tick(1.1);
        assert!(cops.spawning_enabled());
        cops.lock_out();
        cops.unlock();
        assert!(cops.spawning_enabled(), "a chase that starts clears the lockout");
    }

    #[test]
    fn a_bad_step_changes_nothing() {
        let mut cops = CopManager::default();
        cops.lock_out();
        cops.tick(f32::NAN);
        cops.tick(-5.0);
        assert_eq!(cops.lockout_left(), LOCKOUT_FREE_ROAM);
    }

    #[test]
    fn a_no_spawn_zone_bars_the_points_inside_it() {
        let mut cops = CopManager::default();
        assert!(!cops.bars_spawn(Vec3::ZERO), "no zones, no bar");
        cops.set_no_spawn_zones(vec![square_around_zone_origin()]);
        assert!(cops.bars_spawn(Vec3::ZERO));
        assert!(!cops.bars_spawn(Vec3::new(500.0, 0.0, 500.0)));
    }
}
