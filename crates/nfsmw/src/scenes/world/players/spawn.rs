//! Bringing a co-op player's car into the world, and taking it out again. The lead is started by `start.rs`;
//! every other player comes and goes through here.

use anyhow::Result;
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::CarModel;

use super::super::{
    WorldScene,
    drive::{CarRig, Drive, SpawnRequest},
    keys, sun,
};
use super::{LEAD, Player, PlayerId};

impl WorldScene {
    /// Bring a second (or later) player into the world in `model`, near `near` (map x, y), heading if given.
    ///
    /// The car takes a fresh render key from `keys::next_car`, the allocator traffic uses. The lead keeps
    /// `keys::PLAYER_CAR` (0), and the allocator starts at 1, so the two never share a key.
    pub(in crate::scenes::world) fn add_player(
        &mut self,
        renderer: &mut dyn RenderBackend,
        id: PlayerId,
        name: String,
        model: CarModel,
        near: [f32; 2],
        heading: Option<f32>,
    ) -> Result<()> {
        if id == LEAD {
            anyhow::bail!("the lead player is started by start_driving");
        }
        let physics = self.physics_of(&model)?;
        let visuals = nfsmw_data::vehicle_effects::VisualEffectsData::read(
            self.physics.database(),
            model.car_type.as_deref().unwrap_or(&name),
        );
        if let Some(previous) = self.players.remove(id) {
            previous.release(renderer);
        }
        let rig = CarRig::upload(renderer, keys::next_car(), model, self.car_shading, sun::to_sun());
        let request = SpawnRequest { near, heading, exact: None };
        let mut drive = Drive::new(name, rig, physics, request, None, visuals);
        let car_trails = self.car_trails();
        drive.effects.set_enabled(self.tire_effects[0], self.tire_effects[1]);
        drive.effects.set_quality(self.smoke_quality);
        drive.vehicle_effects.set_enabled(self.vehicle_effects[0], car_trails);
        drive.vehicle_effects.set_style(self.spark_style);
        drive.flames.set_enabled(self.exhaust_flames);
        self.players.insert(Player::new(id, drive));
        Ok(())
    }

    /// Take a player out of the world and free its GPU resources. The lead is not removed here; returns whether a
    /// player was removed.
    pub(in crate::scenes::world) fn remove_player(&mut self, renderer: &mut dyn RenderBackend, id: PlayerId) -> bool {
        if id == LEAD {
            return false;
        }
        let Some(drive) = self.players.remove(id) else {
            return false;
        };
        drive.release(renderer);
        true
    }
}
