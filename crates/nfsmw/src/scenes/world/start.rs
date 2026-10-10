//! Getting into a car: its physics, its effects and the settings the effects follow.

use anyhow::{Context, Result};
use blackbox_render::Renderer;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::{CarRig, Drive, DriveScript, SpawnRequest};
use super::{View, WorldScene};

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
        let visuals = nfsmw_data::vehicle_effects::VisualEffectsData::read(
            self.physics.database(),
            model.car_type.as_deref().unwrap_or(&name),
        );
        let rig = CarRig::upload(renderer, model, self.car_shading, super::sun::to_sun());
        let [x, y] = self.focus();
        match self.drive.as_mut() {
            Some(drive) => {
                drive.set_car(renderer, name.clone(), rig, physics, visuals);
                drive.respawn_near([x, y], None);
            }
            None => {
                let request =
                    SpawnRequest { near: [self.camera.position.x, self.camera.position.y], heading: None, exact: None };
                self.drive = Some(Drive::new(name.clone(), rig, physics, request, script, visuals));
            }
        }
        self.last_car = name;
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_enabled(self.tire_effects[0], self.tire_effects[1]);
            drive.effects.set_quality(self.smoke_quality);
            drive.vehicle_effects.set_enabled(self.vehicle_effects[0], self.vehicle_effects[1]);
            drive.vehicle_effects.set_style(self.spark_style);
            drive.flames.set_enabled(self.exhaust_flames);
        }
        self.view = View::Chase;
        Ok(())
    }
}
