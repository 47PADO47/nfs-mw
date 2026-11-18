//! The unattended driver of the drive to the first safe house: with `--autopilot` on an unattended career run
//! (`docs/testing.md`, "Unattended career run") the race autopilot's driver takes the car to the safe house's respawn
//! marker, along a route built from where the drive starts to the marker as a race route is built to its finish. The GPS is
//! not read: the route is the driver's own, so the trace can compare the GPS arrow with where the car goes.

use std::sync::Arc;

use blackbox_race::Gate;
use blackbox_roads::{Goal, Route};
use glam::Vec3;
use nfsmw_data::gameplay::{Placement, world_heading};

use super::drive_to_target::SAFE_HOUSE_ZONE;
use crate::scenes::world::ai::RaceAim;
use crate::scenes::world::{WorldScene, space};

/// The size of the gate at the respawn marker the route is built to: its depth, width and height, metres, as a finish
/// line's.
const GATE_SIZE: [f32; 3] = [2.0, 40.0, 10.0];

impl WorldScene {
    /// Where the autopilot drives on the drive to the first safe house, with `skill`; `None` while no drive runs or the
    /// data has no safe house marker.
    pub(in crate::scenes::world) fn drive_pilot_aim(&mut self, skill: f32) -> Option<RaceAim> {
        self.career.safe_house_drive.machine.as_ref()?;
        let target = self.gameplay.as_ref()?.zone_respawn(SAFE_HOUSE_ZONE)?;
        if self.career.safe_house_drive.pilot_route.is_none() {
            let route = self.drive_pilot_route(target)?;
            self.career.safe_house_drive.pilot_route = Some(route);
        }
        let route = self.career.safe_house_drive.pilot_route.clone();
        Some(RaceAim { goal: Some(target.position), leg: Some(0), route, skill })
    }

    /// The route from the player's car to `target`, through a gate at the marker that faces as the marker does.
    fn drive_pilot_route(&mut self, target: Placement) -> Option<Arc<Route>> {
        let pose = self.players.lead()?.physics_pose();
        let start = Goal {
            position: Vec3::from(space::to_physics(pose.position)),
            forward: Vec3::from(space::to_physics(pose.rotation * Vec3::X)),
        };
        let gate = Gate::new(target.position, world_heading(target.heading_deg), Vec3::Y, GATE_SIZE);
        let traffic = self.traffic.as_mut()?;
        let lines = self.scenery_groups.active_barriers(traffic.track_barriers());
        log::info!("drive: the autopilot drives to the safe house marker at {:?}", target.position);
        Some(Arc::new(traffic.build_race_route(start, &[gate], &lines)))
    }
}
