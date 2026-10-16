//! The world's answers to the mods (`crate::modding`): the player's car, the camera, the track, and the calls that move the
//! car or change the traffic. `mod.rs` forwards the `Scene` methods here, so the world's own file keeps its size.

use blackbox_damage::{Damage, MAX_LEVEL};

use super::{View, WorldScene};
use crate::settings::{Percent, TrafficSettings};
use crate::viewer::camera::{FlyCamera, direction};
use crate::viewer::{CameraReadout, PlayerCar, WorldInfo};

/// The damage of a car as a fraction: the panel, paint and glass levels of every zone over the most they can take.
fn damage_fraction(damage: &Damage) -> f32 {
    let zones = damage.zone_count();
    if zones == 0 {
        return 0.0;
    }
    let total: f32 = (0..zones)
        .map(|zone| {
            let level = damage.zone(zone);
            f32::from(level.panel) + f32::from(level.paint) + f32::from(level.glass)
        })
        .sum();
    total / (zones as f32 * 3.0 * f32::from(MAX_LEVEL))
}

impl WorldScene {
    /// The player's car, when one is driven and simulated (the HUD's numbers and the physics' velocity and damage).
    pub(super) fn player_car_state(&mut self) -> Option<PlayerCar> {
        let drive = self.drive.as_mut()?;
        let hud = drive.hud_state()?;
        let (position, heading, speed) = (drive.position(), drive.heading(), drive.speed());
        let sim = drive.sim_mut()?;
        let velocity = sim.effect_velocity();
        let damage = sim.events.as_ref().map_or(0.0, |events| damage_fraction(&events.damage));
        Some(PlayerCar {
            position: position.to_array(),
            velocity: velocity.to_array(),
            heading,
            speed,
            rpm: hud.rpm,
            gear: hud.gear,
            nos: match hud.has_nos {
                true => hud.nos,
                false => 0.0,
            },
            damage,
        })
    }

    /// The camera the player sees: the chase camera's while the car is driven and the view is the chase, else the free
    /// camera's.
    pub(super) fn camera_now(&self) -> CameraReadout {
        let mode = self.camera_mode;
        match (&self.drive, self.view) {
            (Some(drive), View::Chase) => {
                let view = drive.camera();
                let forward = (view.target() - view.position).normalize_or_zero();
                CameraReadout {
                    mode,
                    position: view.position.to_array(),
                    forward: forward.to_array(),
                    fov: view.fov_degrees(),
                }
            }
            _ => CameraReadout {
                mode,
                position: self.camera.position.to_array(),
                forward: direction(self.camera.yaw, self.camera.pitch).to_array(),
                fov: FlyCamera::FOV_Y_DEGREES,
            },
        }
    }

    /// The track and the district at the player.
    pub(super) fn world_now(&self) -> WorldInfo {
        WorldInfo { track: self.track.clone(), section: self.residency.zone() }
    }

    /// Put the car on the road nearest `at`, facing `heading`, and show the chase view. Refused without a car.
    pub(super) fn move_player_to(&mut self, at: [f32; 2], heading: f32) -> bool {
        let Some(drive) = self.drive.as_mut() else {
            return false;
        };
        drive.respawn_near(at, Some(heading));
        self.view = View::Chase;
        true
    }

    /// Clear the car's visible damage and punctures. Refused without a car.
    pub(super) fn repair_player_car(&mut self) -> bool {
        let Some(drive) = self.drive.as_mut() else {
            return false;
        };
        drive.repair();
        true
    }

    /// Set the traffic count and the cop share, keeping the lights and avoidance settings as they are.
    pub(super) fn set_traffic_density_to(&mut self, cars: u32, cop_share: u8) {
        let setting = TrafficSettings { cars, cop_share: Percent(cop_share), ..self.traffic_setting };
        self.apply_traffic_setting(setting);
    }
}
