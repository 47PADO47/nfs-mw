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
        let Some(drive) = self.players.lead() else { return };
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
        let ground = |at: Vec3| space::ground_below(collision, at.x, at.y, at.z + space::CAR_GROUND_ABOVE, at.z - 10.0);
        for scene in scenes {
            if self.moment.request(&scene, &facts, ground).is_ok() {
                break;
            }
        }
        self.sync_moment_ownership();
    }

    /// Plays `sim_dt` seconds of the cut and applies what it asks of the world: the camera jump and the busted hold.
    pub(super) fn step_moment(&mut self, sim_dt: f32) {
        self.moment.advance(sim_dt);
        self.sync_moment_ownership();
        if self.moment.take_cut() {
            self.cut.arm();
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.hold_busted(self.moment.holds_busted());
        }
    }

    /// Apply camera ownership at each physics tick, reusing the normal race driver on the existing player body.
    pub(super) fn update_moment_control_with_delta(&mut self, dt: f32) {
        self.sync_moment_ownership();
        if !self.moment.hides_hud() {
            return;
        }
        let Some(drive) = self.players.lead() else { return };
        let body = drive.ai_body();
        let Some(view) = drive.sim().map(|sim| sim.driver_view()) else { return };
        let performance = drive.ai_performance();
        let ai = nfsmw_data::pursuit::ai_vehicle(self.physics.database(), &drive.car_name);
        let mass = drive.physical_mass();
        let input = self
            .traffic
            .as_mut()
            .map(|traffic| {
                traffic.moment_input_with_delta(dt, &mut self.moment.driver, body, view, performance, ai, mass)
            })
            .unwrap_or_default();
        if let Some(drive) = self.players.lead_mut() {
            drive.set_cinematic_input(Some(input));
        }
    }

    fn sync_moment_ownership(&mut self) {
        let active = self.moment.hides_hud();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.hold_world_moment(active);
        }
        if active {
            if self.moment.driver.is_none()
                && let Some(drive) = self.players.lead_mut()
            {
                drive.set_cinematic_input(Some(Default::default()));
            }
            return;
        }
        self.moment.driver = None;
        if let Some(drive) = self.players.lead_mut() {
            drive.set_cinematic_input(None);
        }
    }
}
