//! A custom race in the flow: the transmission the player chose is kept as the setting (as the challenge series does), the car
//! and the options go to the world, and leaving the race returns to the Quick Race menu.

use super::*;
use crate::frontend::logic::CustomStart;

impl Frontend {
    pub(in crate::frontend) fn start_custom_race(&mut self, start: CustomStart, host: &mut Host, env: &mut Env) {
        env.settings.transmission = start.transmission;
        env.changed.transmission = Some(start.transmission);
        self.career.end_play();
        self.origin = QuitTo::QuickRace;
        self.race_options = Some(start.options);
        self.quick_race = Some(start.race);
        self.quick_car = Some(match start.preset {
            Some(preset) => format!("{}:{preset}", start.car),
            None => start.car,
        });
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }
}
