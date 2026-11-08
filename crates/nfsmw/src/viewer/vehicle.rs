//! What a scene that shows a vehicle offers the developer tools: the vehicle's options and a way to change them.

use blackbox_gfx::RenderBackend;

use crate::garage::options::{Change, VehicleOptions};

/// A scene with a vehicle on show (`Scene::vehicle`): the world's driven car, the car viewer's car.
pub trait VehicleScene {
    /// What the vehicle on show can change (`garage::options`), for the vehicle panel and the `vehicle` console command. `None`
    /// while there is none.
    fn options(&mut self) -> Option<VehicleOptions>;

    /// Applies a change of the options to the vehicle on show, which is dressed again at once. Returns what was done.
    fn change(&mut self, renderer: &mut dyn RenderBackend, change: &Change) -> Result<String, String>;

    /// The install the scene reads, where the tools can read the rider's gear catalogue from before a rider has loaded.
    fn install_dir(&self) -> Option<&game_install::GameDir> {
        None
    }
}
