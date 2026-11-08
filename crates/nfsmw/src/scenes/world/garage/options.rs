//! The developer tools' view of the car being driven: what it can change, and a change applied through the garage's own path.

use blackbox_gfx::RenderBackend;

use super::WorldScene;
use crate::garage::options::{Change, Edit, VehicleOptions};

impl WorldScene {
    /// The options of the car the player drives. `None` before a car is driven.
    pub(in crate::scenes::world) fn driven_vehicle_options(&mut self) -> Option<VehicleOptions> {
        if self.players.is_empty() {
            return None;
        }
        let car = self.garage.applied.car.clone();
        if car.is_empty() {
            return None;
        }
        self.discover_garage_parts(&car).ok()?;
        VehicleOptions::of_applied(&self.garage)
    }

    /// Applies `change` to the driven car: the choices of the options are edited in the garage's selection, which is applied as
    /// the F6 garage applies one (same car: the model is dressed again where the player stands, with the speed kept).
    pub(in crate::scenes::world) fn change_driven_vehicle(
        &mut self,
        renderer: &mut dyn RenderBackend,
        change: &Change,
    ) -> Result<String, String> {
        let options = self.driven_vehicle_options().ok_or("get into a car first (the drive command)")?;
        let catalog =
            self.garage.cars.iter().find(|car| car.folder == options.folder).and_then(|car| car.parts.clone());
        let mut selection = self.garage.applied.clone();
        let message = options.apply(
            catalog.as_deref(),
            &mut Edit {
                parts: &mut selection.parts,
                color: &mut selection.color,
                performance: &mut selection.performance,
            },
            change,
        )?;
        self.apply_garage_selection(renderer, selection).map_err(|error| format!("{error:#}"))?;
        Ok(message)
    }
}

impl crate::viewer::VehicleScene for WorldScene {
    fn options(&mut self) -> Option<VehicleOptions> {
        self.driven_vehicle_options()
    }

    fn change(&mut self, renderer: &mut dyn RenderBackend, change: &Change) -> Result<String, String> {
        self.change_driven_vehicle(renderer, change)
    }

    fn install_dir(&self) -> Option<&game_install::GameDir> {
        Some(&self.dir)
    }
}
