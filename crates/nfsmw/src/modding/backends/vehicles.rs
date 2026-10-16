//! `bb.vehicles`: the player's car and traffic. The state is the bridge's copy of the car this frame; a move, a repair and a
//! traffic spawn are queued as a [`WorldRequest`] and made on the scene after the mods have run. A spawn is placed on the
//! road nearest the point; the handle it answers is the mod's id for it (the car is not tracked yet, so there is no despawn).

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::{BackendResult, VehicleState, VehiclesBackend};

use super::bridge::Bridge;
use super::world::WorldRequest;

/// The `bb.vehicles` port of the game: one per mod table, sharing the bridge.
pub struct VehiclesPort(pub(super) Rc<Bridge>);

impl VehiclesPort {
    /// The car, or `NotReady` while no car is driven.
    fn car(&self) -> BackendResult<()> {
        if self.0.vehicle.get().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        Ok(())
    }

    /// The world, or `NotReady` while none is loaded.
    fn world(&self) -> BackendResult<()> {
        if self.0.world.borrow().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        Ok(())
    }
}

impl VehiclesBackend for VehiclesPort {
    fn state(&self, _owner: &str) -> BackendResult<VehicleState> {
        let car = self.0.vehicle.get().ok_or(BB_ERR_NOT_READY)?;
        Ok(VehicleState {
            gear: car.gear,
            position: car.position,
            velocity: car.velocity,
            heading: car.heading,
            speed: car.speed,
            rpm: car.rpm,
            nos: car.nos,
            damage: car.damage,
        })
    }

    fn teleport(&self, _owner: &str, position: [f32; 3], heading: f32) -> BackendResult<()> {
        self.car()?;
        // The road is found from the horizontal position; the height is not used.
        let at = [position[0], position[1]];
        self.0.world_requests.borrow_mut().push(WorldRequest::Move { at, heading });
        Ok(())
    }

    fn repair(&self, _owner: &str) -> BackendResult<()> {
        self.car()?;
        self.0.world_requests.borrow_mut().push(WorldRequest::Repair);
        Ok(())
    }

    fn spawn_traffic(&self, _owner: &str, position: [f32; 3], heading: f32) -> BackendResult<u64> {
        self.world()?;
        // The road is found from the horizontal position, as for a teleport; the height is not used.
        let at = [position[0], position[1]];
        self.0.world_requests.borrow_mut().push(WorldRequest::SpawnTraffic { at, heading });
        Ok(self.0.next_handle())
    }
}
