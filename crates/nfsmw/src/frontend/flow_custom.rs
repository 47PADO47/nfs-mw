//! A custom race in the flow: the transmission the player chose is kept as the setting (as the challenge series does), the car
//! and the options go to the world, and leaving the race returns to the Quick Race menu.

use super::*;
use crate::frontend::logic::CustomStart;

impl Frontend {
    pub(super) fn apply_my_car(&mut self, host: &mut Host, id: &str) -> anyhow::Result<()> {
        let model = host.scene.garage().ok_or_else(|| anyhow::anyhow!("The world has no garage"))?;
        let folder = model.applied.car.clone();
        let selection = crate::garage::my_cars::load(&self.dir, model, id, &folder)?;
        let renderer = host.renderer.as_mut().ok_or_else(|| anyhow::anyhow!("Renderer is not ready"))?;
        host.scene.apply_garage(renderer.as_mut(), selection)?;
        if let Some(model) = host.scene.garage() {
            model.custom_slot = Some(id.to_owned());
        }
        Ok(())
    }

    pub(in crate::frontend) fn start_custom_race(&mut self, start: CustomStart, host: &mut Host, env: &mut Env) {
        env.settings.transmission = start.transmission;
        env.changed.transmission = Some(start.transmission);
        self.career.end_play();
        self.origin = QuitTo::QuickRace;
        self.race_options = Some(start.options);
        self.quick_race = Some(start.race);
        self.quick_custom = start.custom;
        self.quick_car = Some(match start.preset {
            Some(preset) => format!("{}:{preset}", start.car),
            None => start.car,
        });
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }
}
