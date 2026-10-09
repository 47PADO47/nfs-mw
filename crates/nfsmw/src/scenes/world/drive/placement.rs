//! Putting the car on the road: respawn requests, placing it, swapping the model.

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable};

use super::ground::WorldGround;
use super::road::Spawn;
use super::sim::CarSim;
use super::{CarRig, Drive, SpawnRequest};
use crate::scenes::world::vehicle_effects::VehicleEffects;

impl Drive {
    /// Ask for the car to be put on the road nearest `near`, keeping `heading` if given.
    pub fn respawn_near(&mut self, near: [f32; 2], heading: Option<f32>) {
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request = Some(SpawnRequest { near, heading, exact: None });
    }

    /// The last place the car was on a road.
    pub fn last_good(&self) -> Option<Spawn> {
        self.last_good
    }

    /// Put the car back where it last stood on a road, if it ever did.
    pub fn restore_last_good(&mut self) -> bool {
        let Some(good) = self.last_good else { return false };
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request =
            Some(SpawnRequest { near: [good.position.x, good.position.y], heading: None, exact: Some(good) });
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
        let sim = self.sim.get_or_insert_with(|| CarSim::new(self.physics.clone(), rest));
        sim.set_visual_tires(self.rig.visual_tires());
        if !sim.place(&ground, spawn) {
            return false;
        }
        self.effects.disconnect();
        let pose = sim.pose();
        self.vehicle_effects.clear();
        (self.previous, self.current) = (pose, pose);
        self.telemetry = sim.telemetry();
        self.clock.reset();
        self.chase.snap();
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
        self.vehicle_effects = VehicleEffects::new(visuals);
        self.physics = physics;
        self.effects.clear();
        self.car_name = name;
        if self.sim.take().is_some() {
            let at = self.current.position;
            self.request = Some(SpawnRequest { near: [at.x, at.y], heading: Some(self.heading()), exact: None });
        }
    }

    /// Whether the camera jumped since the last call (the car was placed on the road).
    pub fn take_camera_cut(&mut self) -> bool {
        self.cut.take()
    }
}
