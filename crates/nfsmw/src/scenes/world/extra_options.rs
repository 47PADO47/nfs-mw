//! F5 actions reuse the world's validated placement and pursuit paths.
use super::{View, WorldScene};
use crate::extra_options::{Action, Bookmark, Location};
use crate::viewer::camera::{Camera, Lens};
use anyhow::{Context, Result, bail, ensure};
use blackbox_gfx::RenderBackend;
use glam::Vec3;

impl WorldScene {
    pub(super) fn extra_action(&mut self, renderer: &mut dyn RenderBackend, action: Action) -> Result<()> {
        ensure!(
            self.rules.time_scale_allowed && !self.players.has_guests(),
            "Extra options require local single player"
        );
        ensure!(
            !self.garage_open && !self.nis.is_playing() && !self.story_movie.is_playing(),
            "Close the shop or cinematic first"
        );
        if !matches!(
            action,
            Action::Center
                | Action::Freecam
                | Action::SaveBookmark
                | Action::LoadBookmark(_)
                | Action::DeleteBookmark(_)
                | Action::Reload
                | Action::Clear
                | Action::SaveLocation
                | Action::DeleteLocation(_)
        ) {
            ensure!(
                !crate::career::CareerHandle::shared().is_active(),
                "Use a free-roam test session outside career for gameplay overrides"
            );
        }
        match action {
            Action::Environment(value) => {
                if self.extra.restore_environment.is_none() {
                    self.extra.restore_environment = Some(self.environment_state());
                }
                self.configure_environment(value);
            }

            Action::Center => {
                if self.extra.freecam {
                    self.view = View::Chase;
                }
                self.extra.freecam = false;
                self.cut.arm();
                if let Some(drive) = self.players.lead_mut() {
                    drive.center_camera();
                }
                self.extra.selected_bookmark = None;
            }
            Action::Freecam => {
                if self.extra.freecam {
                    self.extra.freecam = false;
                    self.extra.selected_bookmark = None;
                    self.view = View::Chase;
                } else {
                    let lens = self.gameplay_lens(renderer.aspect_ratio());

                    self.extra.free_fov = Some(lens.fov_y_degrees);
                    let right = lens.forward.cross(Vec3::Z).normalize_or_zero();
                    let up = right.cross(lens.forward);
                    let actual_up = lens.view.inverse().y_axis.truncate();
                    self.extra.free_roll = actual_up.dot(right).atan2(actual_up.dot(up));
                    self.camera.position = lens.position;
                    self.camera.yaw = lens.forward.y.atan2(lens.forward.x);
                    self.camera.pitch = lens.forward.z.clamp(-1.0, 1.0).asin();
                    self.extra.selected_bookmark = None;
                    self.extra.freecam = true;
                    self.view = View::Fly;
                }
                self.cut.arm();
            }
            Action::Refill => {
                self.players.lead_mut().context("No player car")?.refill_extra(true, true);
            }
            Action::EndPursuit => {
                if let Some(t) = self.traffic.as_mut() {
                    t.stop_pursuit();
                }
            }
            Action::SetHeat => {
                ensure!(!self.extra.testing.police_off, "Enable police first");
                let traffic = self.traffic.as_mut().context("No road network")?;
                ensure!(traffic.set_extra_heat_once(self.extra.testing.heat), "Start a pursuit first");
            }
            Action::StartPursuit => {
                ensure!(!self.extra.testing.police_off, "Enable police first");
                let heat = self.extra.testing.heat.to_string();
                super::ai::pursuit_command(self, renderer, &[&heat]).map_err(anyhow::Error::msg)?;
            }
            Action::Recover => {
                super::commands::run(self, renderer, "reset", &[])
                    .context("No reset command")?
                    .map_err(anyhow::Error::msg)?;
            }
            Action::Repair => {
                super::commands::run(self, renderer, "repair", &[])
                    .context("No repair command")?
                    .map_err(anyhow::Error::msg)?;
            }
            Action::Teleport(at, heading) => self.extra_teleport(at, heading)?,
            Action::Marker => {
                let at = self.route_blip().context("Set a GPS destination on the world map first")?.position;
                self.extra_teleport(at, None)?;
            }
            Action::Return => {
                let at = self.extra.previous_location.clone().context("No previous teleport position")?;
                self.extra_teleport(at.position, Some(at.heading))?;
            }
            Action::SaveLocation => {
                let location = self.extra_location()?;
                ensure!(self.extra.locations.len() < 128, "Location limit reached");
                self.extra.locations.retain(|v| v.name != location.name);
                self.extra.locations.push(location);
                crate::extra_options::storage::save(&self.extra)?;
            }
            Action::DeleteLocation(index) => {
                ensure!(index < self.extra.locations.len(), "Location no longer exists");
                self.extra.locations.remove(index);
                crate::extra_options::storage::save(&self.extra)?;
            }
            Action::SaveBookmark => {
                let name = self.extra_name()?;
                let lens = self.gameplay_lens(renderer.aspect_ratio());
                let relative = self.extra.relative;
                let transform = self.players.lead().context("No player car")?.pose().transform();
                let inverse = transform.inverse();
                let (eye, forward, up) = (lens.position, lens.forward, lens.view.inverse().y_axis.truncate());
                let (eye, forward, up) = match relative {
                    true => (
                        inverse.transform_point3(eye),
                        inverse.transform_vector3(forward),
                        inverse.transform_vector3(up),
                    ),
                    false => (eye, forward, up),
                };
                ensure!(self.extra.bookmarks.len() < 128, "Bookmark limit reached");
                self.extra.bookmarks.retain(|v| v.name != name);
                self.extra.bookmarks.push(Bookmark {
                    name,
                    relative,
                    eye: eye.to_array(),
                    forward: forward.to_array(),
                    up: up.to_array(),
                    fov: lens.fov_y_degrees,
                });
                crate::extra_options::storage::save(&self.extra)?;
            }
            Action::LoadBookmark(index) => {
                let bookmark = self.extra.bookmarks.get(index).context("Bookmark no longer exists")?.clone();
                self.extra.freecam = false;
                self.extra.selected_bookmark = Some(bookmark);
                self.cut.arm();
            }
            Action::DeleteBookmark(index) => {
                ensure!(index < self.extra.bookmarks.len(), "Bookmark no longer exists");
                self.extra.bookmarks.remove(index);
                crate::extra_options::storage::save(&self.extra)?;
            }
            Action::Reload => crate::extra_options::storage::reload(&mut self.extra)?,
            Action::Clear => {
                self.extra.testing = Default::default();
                if let Some(env) = self.extra.restore_environment.take() {
                    self.configure_environment(env);
                }
                self.extra.selected_bookmark = None;
                self.extra.hide_ui = false;
            }
        }
        Ok(())
    }
    fn extra_name(&self) -> Result<String> {
        let name = self.extra.name.trim();
        ensure!(
            !name.is_empty() && name.chars().count() <= 64 && !name.chars().any(char::is_control),
            "Enter a name (1–64 characters)"
        );
        Ok(name.into())
    }
    fn extra_location(&self) -> Result<Location> {
        let drive = self.players.lead().context("No player car")?;
        let p = drive.position();
        Ok(Location { name: self.extra_name()?, position: [p.x, p.y], heading: drive.heading() })
    }
    fn extra_teleport(&mut self, at: [f32; 2], heading: Option<f32>) -> Result<()> {
        ensure!(at.iter().all(|v| v.is_finite() && v.abs() < 1_000_000.0), "Invalid map coordinates");
        if self.race.is_some() {
            bail!("End the current race before teleporting");
        }
        let drive = self.players.lead_mut().context("No player car")?;
        let p = drive.position();
        self.extra.previous_location =
            Some(Location { name: "Previous position".into(), position: [p.x, p.y], heading: drive.heading() });
        drive.respawn_near(at, heading);
        self.extra.teleport_heading = heading;
        self.view = View::Chase;
        self.extra.freecam = false;
        self.extra.selected_bookmark = None;
        self.cut.arm();
        Ok(())
    }
    pub(super) fn extra_lens(&self, aspect: f32) -> Option<Lens> {
        if let Some(b) = &self.extra.selected_bookmark {
            let (mut eye, mut forward, mut up) =
                (Vec3::from_array(b.eye), Vec3::from_array(b.forward), Vec3::from_array(b.up));
            if b.relative {
                let transform = self.players.lead()?.pose().transform();
                eye = transform.transform_point3(eye);
                forward = transform.transform_vector3(forward);
                up = transform.transform_vector3(up);
            }
            let mut lens = Lens::look_at(eye, eye + forward, b.fov, 0.1, aspect);
            lens.view = glam::camera::rh::view::look_at_mat4(eye, eye + forward, up);
            return Some(lens);
        }
        if self.extra.freecam {
            let mut lens = self.camera.lens(aspect);
            let fov = self.extra.free_fov.unwrap_or(70.0);
            lens.fov_y_degrees = fov;

            lens.projection = crate::viewer::camera::projection(fov, aspect, 0.1);
            let canonical_up = lens.forward.cross(Vec3::Z).normalize_or_zero().cross(lens.forward);
            let up = glam::Quat::from_axis_angle(lens.forward, self.extra.free_roll) * canonical_up;
            lens.view = glam::camera::rh::view::look_at_mat4(lens.position, lens.position + lens.forward, up);
            return Some(lens);
        }
        None
    }
    pub(super) fn prepare_extra(&mut self) {
        self.extra.local_allowed = self.rules.time_scale_allowed && !self.players.has_guests();
        if !self.extra.local_allowed {
            if let Some(env) = self.extra.restore_environment.take() {
                self.configure_environment(env);
            }
            self.extra.testing = Default::default();
            self.extra.selected_bookmark = None;
            if self.extra.freecam {
                self.view = View::Chase;
            }
            self.extra.freecam = false;
        }
        let mut gameplay = self.extra.testing.clone();
        gameplay.paused = false;
        if (!self.rules.time_scale_allowed
            || self.players.has_guests()
            || crate::career::CareerHandle::shared().is_active())
            && gameplay.active()
        {
            let paused = self.extra.testing.paused;
            self.extra.testing = Default::default();
            self.extra.testing.paused = paused;
            self.extra.message =
                Some("Gameplay overrides require a local free-roam test session outside career".into());
        }
        let locked = self.extra.testing.lock_heat.then_some(self.extra.testing.heat);
        if locked != self.extra.applied_heat {
            if let Some(t) = self.traffic.as_mut() {
                t.set_extra_heat(locked);
            }
            self.extra.applied_heat = locked;
        }
        self.extra.environment = Some(self.environment_state());
        for drive in self.players.drives_mut() {
            if self.extra.local_allowed {
                drive.configure_extra(&self.extra.camera);
                continue;
            }
            drive.configure_extra(&Default::default());
        }
        if let Some(t) = self.traffic.as_mut() {
            t.set_extra_traffic(self.extra.testing.traffic_off);
            if self.extra.testing.police_off {
                t.stop_pursuit();
            }
        }
    }
}

#[cfg(test)]
mod tests;

mod command;
pub(super) use command::run as command;
