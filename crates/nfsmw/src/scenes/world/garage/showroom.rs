//! The world keeps its driving state while the original shop platform is in the window.

use anyhow::Result;
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::LoadOptions;

use super::WorldScene;
use crate::frontend::{GarageBackdrop, GarageStage};
use crate::garage::{Selection, View};
use crate::viewer::Scene;

impl WorldScene {
    pub(in crate::scenes::world) fn garage_entry_allowed(&self) -> bool {
        nfsmw_career::safehouse::EntryPermissions::compute(self.race.is_some(), self.safe_house_phase())
            .allows(nfsmw_career::safehouse::ZoneKind::CustomShop)
    }

    pub(in crate::scenes::world) fn remember_garage_exit(&mut self, zone: &str) {
        self.garage_exit = self.gameplay.as_ref().and_then(|gameplay| gameplay.zone_respawn(zone));
    }

    pub(in crate::scenes::world) fn leave_garage_at_exit(&mut self) {
        let Some(marker) = self.garage_exit.take() else { return };
        if let Some(drive) = self.players.lead_mut() {
            drive.respawn_at(super::super::road::Spawn {
                position: super::super::space::to_render(marker.position.to_array()),
                heading: marker.heading_deg.to_radians(),
            });
        }
        self.career.respawn.guests_behind = true;
    }

    pub(in crate::scenes::world) fn show_garage_view(
        &mut self,
        renderer: &mut dyn RenderBackend,
        stage: View,
        camera: &'static str,
    ) -> Result<()> {
        let stage = match stage {
            View::Cars => GarageStage::Cars,
            View::Customization => GarageStage::Customization,
            View::Backroom => GarageStage::Backroom,
        };
        let new = self.garage_backdrop.is_none();
        let model = match new {
            true => Some(load_model(&self.dir, &self.garage.draft)?),
            false => None,
        };
        let mut backdrop = self.garage_backdrop.take().unwrap_or_else(|| GarageBackdrop::new(self.dir.clone()));
        backdrop.set_car_shading(self.car_shading);
        let result = backdrop.set_stage(renderer, stage).and_then(|()| {
            if let Some(model) = model {
                backdrop.show_model(renderer, model, options())?;
            }
            Ok(())
        });
        if let Err(error) = result {
            if !new {
                self.garage_backdrop = Some(backdrop);
                return Err(error);
            }
            backdrop.close(renderer);
            renderer.set_colour_treatment(self.colour_treatment.as_ref(), self.gain_map.as_ref());
            self.update_sky(renderer);
            self.refresh_effects(renderer);
            return Err(error);
        }
        backdrop.show_camera(camera);
        self.garage_backdrop = Some(backdrop);
        Ok(())
    }

    pub(in crate::scenes::world) fn update_garage_backdrop(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: &Selection,
    ) -> Result<()> {
        let Some(backdrop) = &mut self.garage_backdrop else { return Ok(()) };
        let model = load_model(&self.dir, selection)?;
        backdrop.show_model(renderer, model, options())
    }

    pub(in crate::scenes::world) fn close_garage_backdrop(&mut self, renderer: &mut dyn RenderBackend) {
        let Some(mut backdrop) = self.garage_backdrop.take() else { return };
        backdrop.close(renderer);
        self.reflection_last = None;
        renderer.set_colour_treatment(self.colour_treatment.as_ref(), self.gain_map.as_ref());
        self.update_sky(renderer);
        self.refresh_effects(renderer);
    }
}

fn options() -> LoadOptions {
    LoadOptions { lod: 'A', all_parts: false, preset: None }
}

fn load_model(dir: &game_install::GameDir, selection: &Selection) -> Result<nfsmw_data::car::CarModel> {
    let mut model = nfsmw_data::car::load_customized(dir, &selection.car, &options(), &selection.parts)?;
    if let Some(color) = selection.color {
        model.set_body_color(color)?;
    }
    Ok(model)
}
