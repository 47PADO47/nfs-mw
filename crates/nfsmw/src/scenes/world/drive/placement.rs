//! Putting the car on the road: respawn requests, placing it, swapping the model.

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable};

use super::ground::WorldGround;
use super::road::Spawn;
use super::sim::CarSim;
use super::{CarRig, Drive, SpawnRequest};
use crate::scenes::world::vehicle_effects::VehicleEffects;

mod strict;
#[cfg(test)]
mod tests;

pub use strict::Strict;

impl Drive {
    /// Ask for the car to be put on the road nearest `near`, keeping `heading` if given.
    pub fn respawn_near(&mut self, near: [f32; 2], heading: Option<f32>) {
        self.tail_trails.clear();
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request = Some(SpawnRequest { near, heading, exact: None, strict: false });
    }

    /// A garage exit uses its authored marker, waiting for the ground instead of moving to a different road.
    pub fn respawn_at(&mut self, spawn: Spawn) {
        self.respawn_near([spawn.position.x, spawn.position.y], Some(spawn.heading));
        if let Some(request) = &mut self.request {
            request.exact = Some(spawn);
            request.strict = true;
        }
    }

    /// The last place the car was on a road.
    pub fn last_good(&self) -> Option<Spawn> {
        self.last_good
    }

    /// Put the car back where it last stood on a road, if it ever did.
    pub fn restore_last_good(&mut self) -> bool {
        let Some(good) = self.last_good else { return false };
        self.tail_trails.clear();
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request = Some(SpawnRequest {
            near: [good.position.x, good.position.y],
            heading: None,
            exact: Some(good),
            strict: false,
        });
        true
    }

    /// Give up waiting for a road (none was found).
    pub fn cancel_request(&mut self) {
        self.request = None;
    }

    /// Put the car on `spawn`, standing still. False when `ground` has no road there.
    pub fn spawn(&mut self, spawn: Spawn, collision: &CollisionWorld, surfaces: &SurfaceTable) -> bool {
        let ground = WorldGround { collision, surfaces };
        let request = self.request.take();
        let spawn = match request {
            // Keep facing the way the car did: the road runs both ways.
            Some(SpawnRequest { heading: Some(h), exact: None, .. }) if (spawn.heading - h).cos() < 0.0 => {
                Spawn { heading: spawn.heading + std::f32::consts::PI, ..spawn }
            }
            _ => spawn,
        };
        let rest = self.rig.rest_heights();
        let replacing = self.sim.is_some() && self.last_good.is_some();
        let sim = self.sim.get_or_insert_with(|| CarSim::new(self.physics.clone(), rest));
        sim.set_tunings(self.tunings);
        sim.set_visual_tires(self.rig.visual_tires());
        for (zone, severity, glazed) in self.queued_hits.drain(..) {
            sim.inflict(zone, severity, glazed);
        }
        let strict = request.filter(|r| r.strict);
        if !sim.place_moving(&ground, spawn, self.pending_speed) {
            // An exact placement (a race start) keeps its request and its speed: it waits for its ground.
            self.request = strict;
            if strict.is_none() {
                self.pending_speed = 0.0;
            }
            return false;
        }
        self.pending_speed = 0.0;
        self.strict_since = None;
        if replacing {
            sim.begin_reset_collision_grace();
        }
        self.effects.disconnect();
        let pose = sim.pose();
        self.tail_trails.clear();
        self.vehicle_effects.clear();
        (self.previous, self.current) = (pose, pose);
        self.telemetry = sim.telemetry();
        self.alpha = 0.0;
        self.chase.snap();
        // Placement can complete with the garage open, before any driving ticks.
        self.follow(collision, (0.0, 0.0), 0.0);
        self.cut.arm();
        self.steps = 0;
        self.fall.reset();
        self.last_check = 0;
        self.last_good = Some(spawn);
        true
    }

    /// Swap the car model; the car is put back on the road where it stands.
    pub fn set_car(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        name: String,
        rig: CarRig,
        physics: CarPhysics,
        visuals: nfsmw_data::vehicle_effects::VisualEffectsData,
    ) {
        std::mem::replace(&mut self.rig, rig).release(renderer);
        self.tail_trails.clear();
        self.vehicle_effects = VehicleEffects::new(visuals);
        self.physics = physics;
        self.performance = Default::default();
        self.tuning = Default::default();
        self.tunings = Default::default();
        self.flames.set_performance(self.performance);
        self.flames.unload();
        self.effects.clear();
        self.car_name = name;
        if self.sim.take().is_some() {
            let at = self.current.position;
            self.request =
                Some(SpawnRequest { near: [at.x, at.y], heading: Some(self.heading()), exact: None, strict: false });
        }
    }

    /// Whether the camera jumped since the last call (the car was placed on the road).
    pub fn take_camera_cut(&mut self) -> bool {
        self.cut.take()
    }
}
