//! The world scene's side of the traffic: where it spawns around, which settings it follows and when it runs.

use blackbox_gfx::RenderBackend;

use super::{Focus, LampMeshes, load_requested};
use crate::scenes::world::{View, WorldScene};
use crate::settings::TrafficSettings;

impl WorldScene {
    /// Where traffic spawns around: the car the police follow (the lead outside co-op), or the free camera.
    pub(in crate::scenes::world) fn traffic_focus(&self) -> Focus {
        match (self.players.get(self.chase_primary()), self.view) {
            (Some(drive), View::Chase) => {
                let (p, heading, speed) = (drive.position(), drive.heading(), drive.speed());
                let body = drive.ai_body();
                Focus { position: [p.x, p.y], heading, speed, body: Some(body) }
            }
            _ => Focus {
                position: [self.camera.position.x, self.camera.position.y],
                heading: self.camera.yaw,
                speed: 0.0,
                body: None,
            },
        }
    }

    /// Every other player as the traffic sees it (all but the car the police follow): its car as a focus and its
    /// camera (`population/guests.rs`).
    fn guest_traffic_views(&self) -> Vec<(Focus, crate::viewer::camera::Lens)> {
        if self.view != View::Chase {
            return Vec::new();
        }
        let primary = self.chase_primary();
        self.players
            .iter()
            .filter(|guest| guest.id != primary && guest.drive.has_sim())
            .map(|guest| {
                let drive = &guest.drive;
                let p = drive.position();
                let focus = Focus {
                    position: [p.x, p.y],
                    heading: drive.heading(),
                    speed: drive.speed(),
                    body: Some(drive.ai_body()),
                };
                (focus, self.guest_rules_lens(drive))
            })
            .collect()
    }

    /// The traffic settings changed (or came in for the first time). A change of the car count or the cop share
    /// replaces a count typed in the console.
    pub(in crate::scenes::world) fn apply_traffic_setting(&mut self, setting: TrafficSettings) {
        if self.traffic_setting == setting {
            return;
        }
        let counts_changed =
            (self.traffic_setting.cars, self.traffic_setting.cop_share) != (setting.cars, setting.cop_share);
        self.traffic_setting = setting;
        if counts_changed {
            self.traffic_manual = None;
        }
    }

    /// How many cars to keep: the console's count if there is one, else the setting while a car is driven.
    fn traffic_wanted(&self) -> usize {
        match self.traffic_manual {
            Some(n) => n,
            None if self.players.has_lead() => self.traffic_setting.cars as usize,
            None => 0,
        }
    }

    pub(in crate::scenes::world) fn prepare_traffic_support(&mut self) {
        let focus = self.traffic_focus();
        let policy = self.traffic_setting.policy.sanitized();
        let range = match self.traffic_wanted() > 0 && policy.density_percent > 0 {
            true => policy.spawn_max as f32 + focus.speed.abs().min(200.0) + 100.0,
            false => 0.0,
        };
        let actors = self.traffic.as_ref().map_or_else(Vec::new, |t| t.collision_anchors());
        let ahead = (policy.spawn_min + policy.spawn_max) as f32 * 0.5 + focus.speed.max(0.0);
        let spawn = [focus.position[0] + ahead * focus.heading.cos(), focus.position[1] + ahead * focus.heading.sin()];
        self.residency.set_traffic_support(focus.position, spawn, range, actors);
    }

    /// Apply settings and load requested models before any physics tick.
    pub(in crate::scenes::world) fn prepare_traffic(&mut self, renderer: &mut dyn RenderBackend) {
        let (wanted, share) = (self.traffic_wanted(), u32::from(self.traffic_setting.cop_share.0));
        let barred = self.race_bars_cops();
        let guests = self.guest_traffic_views();
        let blocking_models = self.rules.blocking_models;
        let Some(traffic) = self.traffic.as_mut() else { return };
        traffic.set_guests(guests);
        traffic.set_pursuit_options(self.traffic_setting.pursuit);
        traffic.configure(wanted, share);
        traffic.bar_cops(barred);
        traffic.set_policy(self.traffic_setting.policy);
        traffic.set_lights(self.traffic_setting.lights);
        traffic.set_avoidance(self.traffic_setting.avoidance);
        // The models the manager asks for are read on the loader's worker. A warm-up run (`traffic warmup`) waits for
        // them in the frame they are asked for, so it sees the same cars every time; a playing scene does not wait, unless
        // a session's rules make the server's model choice independent of the loader's timing (`session_rules.rs`).
        let (submit, blocking) =
            (self.residency.complete() && traffic.wants_models(), !traffic.settled() || blocking_models);
        load_requested(self, renderer, submit, blocking);
        if self.traffic_setting.lights && self.lamp_meshes.is_none() {
            self.lamp_meshes = Some(LampMeshes::upload(renderer));
        }
    }
}
