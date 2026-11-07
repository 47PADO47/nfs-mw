//! A story race can temporarily require its own player car and equipment.
use anyhow::{Context, Result};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::{CarTables, physics::performance::preset};

use crate::scenes::world::{WorldScene, car_loader};

impl WorldScene {
    pub(in crate::scenes::world) fn queue_career_event(&mut self, event: &str) -> bool {
        if self.race.is_some() || self.gameplay.as_ref().is_none_or(|data| super::start::resolve(data, event).is_err())
        {
            return false;
        }
        self.queue_race(event.to_owned(), super::RaceOptions { career: true, ..Default::default() });
        true
    }

    pub(super) fn prepare_career_car(&mut self, renderer: &mut dyn RenderBackend, query: &str) -> Result<()> {
        let data = self.gameplay.as_ref().context("race data unavailable")?;
        let race = super::start::resolve(data, query).map_err(anyhow::Error::msg)?;
        let Some(preset_name) = race.story.player_car_type.clone() else { return Ok(()) };
        let tables = CarTables::load(&self.dir)?;
        let ride = tables.preset(&preset_name).context("forced player preset missing")?;
        let vehicle = preset::vehicle_name(self.physics.database(), ride).to_owned();
        let wanted = format!("{}:{}", ride.car_type_name, ride.preset_name);
        let (folder, model) = car_loader::load_named_car(&self.dir, &wanted)?;
        self.start_driving_setup(renderer, folder.clone(), model, None, false)?;
        let package = preset::read(self.physics.database(), &vehicle, &preset_name)?.unwrap_or_default();
        let physics = self.physics.car_with_performance(&vehicle, package)?;
        if let Some(drive) = self.players.lead_mut() {
            drive.set_performance(physics, package);
            drive.sound_car = Some(vehicle);
        }
        Ok(())
    }
}
