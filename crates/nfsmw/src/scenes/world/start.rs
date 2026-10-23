//! Getting into a car: its physics, its effects and the settings the effects follow.

use anyhow::{Context, Result};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::{CarRig, Drive, DriveScript, SpawnRequest};
use super::{View, WorldScene, keys, sun};

impl WorldScene {
    /// The physics of the car `model` was assembled as.
    pub(super) fn physics_of(&mut self, model: &CarModel) -> Result<CarPhysics> {
        let type_name = model.car_type.as_deref().context("this car is not in the car tables, so it has no physics")?;
        self.physics.car(type_name)
    }

    /// Start (or restart) driving `model` from the camera's place.
    pub(super) fn start_driving(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: String,
        model: CarModel,
        script: Option<DriveScript>,
    ) -> Result<()> {
        let physics = self.physics_of(&model)?;
        self.discover_garage_parts(&name)?;
        let visuals = nfsmw_data::vehicle_effects::VisualEffectsData::read(
            self.physics.database(),
            model.car_type.as_deref().unwrap_or(&name),
        );
        self.camera_environment.views = nfsmw_data::camera::load_car(self.physics.database(), &name)
            .or_else(|| nfsmw_data::camera::load(self.physics.database()));
        self.original_camera.reset();
        self.original_player_step = None;
        let rig = CarRig::upload(renderer, keys::PLAYER_CAR, model, self.car_shading, sun::to_sun());
        let [x, y] = self.focus();
        match self.players.lead_mut() {
            Some(drive) => {
                drive.set_car(renderer, name.clone(), rig, physics, visuals);
                drive.respawn_near([x, y], None);
            }
            None => {
                let request = SpawnRequest {
                    near: [self.camera.position.x, self.camera.position.y],
                    heading: None,
                    exact: None,
                    strict: false,
                };
                self.players.set_lead(Drive::new(name.clone(), rig, physics, request, script, visuals));
            }
        }
        self.last_car = name;
        self.garage.applied =
            crate::garage::Selection { car: self.last_car.clone(), color: None, ..Default::default() };
        self.garage.draft = self.garage.applied.clone();
        if let Some(car) = self.garage.selected() {
            self.garage.category = car.category;
        }
        let car_trails = self.car_trails();
        if let Some(drive) = self.players.lead_mut() {
            drive.effects.set_enabled(self.tire_effects[0], self.tire_effects[1]);
            drive.effects.set_quality(self.smoke_quality);
            drive.vehicle_effects.set_enabled(self.vehicle_effects[0], car_trails);
            drive.vehicle_effects.set_style(self.spark_style);
            drive.flames.set_enabled(self.exhaust_flames);
        }
        self.view = View::Chase;
        Ok(())
    }
}
