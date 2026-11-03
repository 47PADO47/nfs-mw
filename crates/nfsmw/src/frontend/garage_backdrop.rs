//! The normal garage uses the same installed rooms, lighting and camera mover as the other FEng menus.
//! The transaction owner supplies the assembled car; this module never changes its save or physics.

use anyhow::{Context, Result, ensure};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions};

use super::super::{camera_rig::CameraRig, look::CarLook, room::RoomData, stage::Stage};
use super::{Look, MenuScene, Room};
use crate::input::ActionState;
use crate::settings::CarShading;
use crate::viewer::Scene;

#[path = "garage_inspection.rs"]
mod inspection;
use inspection::Inspection;
#[path = "garage_preview_cache.rs"]
mod previews;

/// The menu's car-selection room, the normal shop and its marker-upgrade backroom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GarageStage {
    #[default]
    Cars,
    Customization,
    Backroom,
}

impl GarageStage {
    /// The look of this view in a garage opened on `home`. The original keeps the platform it has when Customize opens:
    /// My Cars customizes in the crib, and only an auto shop entered from the world stands in the shop
    /// (`menu-backdrop-looks.md`, "Which room the garage uses"). The marker backroom is its own platform from either.
    fn look(self, home: Stage) -> Look {
        let car = CarLook::default_car();
        match (self, home) {
            (Self::Cars, _) | (Self::Customization, Stage::Crib) => Look::Main,
            (Self::Customization, _) => Look::Customization(car),
            (Self::Backroom, _) => Look::Backroom(car),
        }
    }
}

/// A garage-owned backdrop. A world can keep this beside its driving scene and resume the same world afterwards.
pub(crate) struct GarageBackdrop {
    scene: MenuScene,
    loaded_stage: Option<Stage>,
    /// The platform the garage was opened on: the crib for My Cars, the shop for an auto shop.
    home: Stage,
    inspection: Inspection,
    camera: &'static str,
    previews: previews::Previews,
}

impl GarageBackdrop {
    pub(crate) fn preview_pending(&mut self, pending: bool) {
        if self.scene.hide_car != pending {
            self.scene.still = None;
        }
        self.scene.hide_car = pending;
    }

    pub(crate) fn new(dir: GameDir) -> Self {
        Self {
            scene: MenuScene { room_first: true, ..MenuScene::with_look(Some(dir), Look::Main) },
            loaded_stage: None,
            home: Stage::Crib,
            inspection: Inspection::default(),
            camera: "",
            previews: previews::Previews::default(),
        }
    }

    /// A garage opened in an auto shop: Customize stands in the shop's own room.
    pub(crate) fn shop(dir: GameDir) -> Self {
        Self { home: Stage::Customization, ..Self::new(dir) }
    }

    /// Share the front end's mover, retaining its current pose across the car-selection and shop screens.
    pub(crate) fn with_rig(mut self, rig: CameraRig) -> Self {
        self.scene.rig = rig;
        self
    }

    pub(crate) fn set_stage(&mut self, renderer: &mut dyn RenderBackend, stage: GarageStage) -> Result<()> {
        let look = stage.look(self.home);
        let target = look.stage();
        if self.loaded_stage == Some(target) {
            return Ok(());
        }
        let dir = self.scene.dir.clone().context("Garage install is unavailable")?;
        let dir = &dir;
        // Read before releasing the visible room, so a bad replacement leaves the current preview intact.
        let data = RoomData::read(dir, target)?;
        let room = Room::upload(renderer, &data);
        if let Some(previous) = self.scene.room.replace(room) {
            previous.destroy(renderer);
        }
        let fx = crate::frontend::room_fx::RoomFx::new(dir.clone(), data.placements().to_vec());
        if let Some(previous) = std::mem::replace(&mut self.scene.fx, fx) {
            previous.release(renderer);
        }
        let returning = (stage == GarageStage::Cars || target == self.home) && self.loaded_stage.is_some();
        if !returning || !self.scene.rig.resume_stage(target) {
            self.scene.rig.begin(dir, target, look.camera_intro().expect("garage stages have intros"));
        }
        self.scene.look = look;
        if let Some(car) = &mut self.scene.car {
            car.set_frontend_backroom(target == Stage::Backroom);
        }
        self.scene.reflection = None;
        self.scene.still = None;
        self.inspection = Inspection::default();
        self.camera = "";
        self.loaded_stage = Some(target);
        Ok(())
    }

    /// Upload a validated preview without changing the transaction's selected parts.
    pub(crate) fn show_model(
        &mut self,
        renderer: &mut dyn RenderBackend,
        model: CarModel,
        options: LoadOptions,
    ) -> Result<()> {
        if self.loaded_stage.is_none() {
            self.set_stage(renderer, GarageStage::Cars)?;
        }
        if let Some(car) = &mut self.scene.car {
            car.replace_model(renderer, model);
            self.scene.reflection = None;
            self.scene.still = None;
            return Ok(());
        }
        self.scene.upload_car(renderer, model, options);
        ensure!(self.scene.car.is_some(), "Garage preview could not be uploaded");
        self.scene
            .car
            .as_mut()
            .expect("car uploaded")
            .set_frontend_backroom(self.loaded_stage == Some(Stage::Backroom));
        Ok(())
    }

    pub(crate) fn show_camera(&mut self, screen: &'static str) {
        if self.camera == screen {
            return;
        }
        let showcase = matches!(screen, "showcase" | "customize_showcase");
        self.scene.rig.show(screen);
        if let Some(pose) = self.inspection.take_pose() {
            self.scene.rig.resume_from(pose);
        }
        self.inspection.set_showcase(showcase);
        self.camera = screen;
    }
}

impl Scene for GarageBackdrop {
    fn title(&self) -> String {
        "nfsmw — Garage".into()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.set_stage(renderer, GarageStage::Cars)
    }

    fn ready(&self) -> bool {
        self.loaded_stage.is_some() && self.scene.car.is_some()
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.scene.advance(renderer);
        self.scene.update_reflections(renderer);
        self.scene.update_car(renderer, input, dt);
        self.scene.update_fx(renderer, dt);
        self.inspection.set_enabled(self.scene.rig.user_rotate());
        self.inspection.set_shape(self.scene.rig.screen_angle());
        if let Some(pose) = self.inspection.update(input, self.scene.rig.pose(), dt, self.scene.rig.moving()) {
            self.scene.rig.resume_from(pose);
        }
        if self.scene.hide_car {
            self.scene.update_menu_casters(renderer);
            renderer.set_planar_reflection_scene(None);
            return;
        }
        if self.scene.still_unchanged_from(renderer, self.inspection.pose()) {
            return;
        }
        self.scene.update_menu_casters(renderer);
        self.scene.update_floor_reflection_from(renderer, self.inspection.pose());
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let pose = self.inspection.pose();
        let (mut frame, instances) = self.scene.frame(aspect);
        if let Some(pose) = pose {
            let lens = pose.lens(aspect);
            (frame.view, frame.projection, frame.camera_position) = (lens.view, lens.projection, lens.position);
        }
        (frame, instances)
    }

    fn prepare_frame(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        self.scene.prepare_frame(renderer, aspect);
    }

    fn set_car_shading(&mut self, shading: CarShading) {
        self.scene.set_car_shading(shading);
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        self.scene.refresh_effects(renderer);
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.clear_previews(renderer);
        self.scene.close(renderer);
        self.loaded_stage = None;
    }

    fn reattached(&mut self, renderer: &mut dyn RenderBackend) {
        self.scene.reattached(renderer);
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        self.scene.hud_state()
    }

    fn radio_on_menus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_cars_customizes_in_the_crib_and_an_auto_shop_in_the_shop() {
        assert_eq!(GarageStage::Customization.look(Stage::Crib).stage(), Stage::Crib);
        assert_eq!(GarageStage::Cars.look(Stage::Crib).stage(), Stage::Crib);
        assert_eq!(GarageStage::Customization.look(Stage::Customization).stage(), Stage::Customization);
        for home in [Stage::Crib, Stage::Customization] {
            assert_eq!(
                GarageStage::Backroom.look(home).stage(),
                Stage::Backroom,
                "the marker backroom is its own room"
            );
        }
    }
}
