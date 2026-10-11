//! Loading and starting the player car.

use anyhow::{Context, Result};
use blackbox_render::Renderer;
use game_install::GameDir;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::{CarRig, Drive, DriveScript, SpawnRequest};
use super::{View, WorldScene};

/// Load a car for driving (highest detail, stock parts). `name` may be a unique prefix.
pub(super) fn load_car(dir: &GameDir, name: &str) -> Result<(String, CarModel)> {
    let cars = nfsmw_data::car::list(dir);
    let folder = nfsmw_data::car::pick_folder(&cars, name)
        .with_context(|| format!("no car {name:?}, or several start with it (`nfsmw list-cars`)"))?
        .to_owned();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(dir, &folder, &options)?;
    Ok((folder, model))
}

impl WorldScene {
    /// The physics of the car `model` was assembled as.
    pub(super) fn physics_of(&mut self, model: &CarModel) -> Result<CarPhysics> {
        let type_name = model.car_type.as_deref().context("this car is not in the car tables, so it has no physics")?;
        self.physics.car(type_name)
    }

    /// Start (or restart) driving `model` from the camera's place.
    pub(super) fn start_driving(
        &mut self,
        renderer: &mut Renderer,
        name: String,
        model: CarModel,
        script: Option<DriveScript>,
    ) -> Result<()> {
        let physics = self.physics_of(&model)?;
        let rig = CarRig::upload(renderer, model);
        let [x, y] = self.focus();
        match self.drive.as_mut() {
            Some(drive) => {
                drive.set_car(renderer, name.clone(), rig, physics);
                drive.respawn_near([x, y], None);
            }
            None => {
                let request =
                    SpawnRequest { near: [self.camera.position.x, self.camera.position.y], heading: None, exact: None };
                self.drive = Some(Drive::new(name.clone(), rig, physics, request, script));
            }
        }
        self.last_car = name;
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_enabled(self.tire_effects[0], self.tire_effects[1]);
            drive.effects.set_quality(self.smoke_quality);
        }
        self.view = View::Chase;
        Ok(())
    }
}
