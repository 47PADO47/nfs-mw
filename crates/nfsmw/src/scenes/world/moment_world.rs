//! The world's side of the cinematic moment: what it tells the moment when a breaker asks, and what the moment asks of
//! the rest of the world while it plays. The rules are in `moment.rs`; spec `docs/specs/pursuit-breaker-moment.md`.

use glam::Vec3;

use super::moment::Facts;
use super::{WorldScene, space};

impl WorldScene {
    /// The console line about the cinematic moment.
    pub(super) fn moment_status(&self) -> String {
        self.moment.status()
    }

    /// The breaker sequences asked for these scenes this step. The first the rules allow plays.
    pub(super) fn play_moments(&mut self, scenes: Vec<String>) {
        if scenes.is_empty() {
            return;
        }
        self.moment.prepare(&self.dir);
        let Some(drive) = self.drive.as_ref() else { return };
        let view = drive.player_view();
        let pursuers = self.traffic.as_ref().map(|traffic| traffic.pursuer_positions()).unwrap_or_default();
        let facts = Facts {
            touch_age: drive.prop_touch_age(),
            player: view.position,
            forward: view.forward,
            pursuers: &pursuers,
            arrested: self.traffic.as_ref().is_some_and(|traffic| traffic.arrest_active()),
        };
        let (collision, _) = self.residency.world_parts();
        let ground = |at: Vec3| space::ground_below(collision, at.x, at.y, at.z + 10.0, at.z - 10.0);
        for scene in scenes {
            if self.moment.request(&scene, &facts, ground).is_ok() {
                break;
            }
        }
    }

    /// Plays `sim_dt` seconds of the cut and applies what it asks of the world: the camera jump and the busted hold.
    pub(super) fn step_moment(&mut self, sim_dt: f32) {
        self.moment.advance(sim_dt);
        if self.moment.take_cut() {
            self.cut.arm();
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.hold_busted(self.moment.holds_busted());
        }
    }
}
