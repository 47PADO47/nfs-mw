//! Visibility and composition of the world HUD.
use super::WorldScene;

impl WorldScene {
    pub(super) fn scene_hud(&self) -> Option<crate::hud::HudState> {
        if self.career.garage.is_some() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        // `None` would show the viewer's idle HUD, so a playing scene answers with a hidden one.
        if self.cutscene_playing() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        if self.moment.hides_hud() {
            return None;
        }
        let looking_back = self.players.lead().is_some_and(|d| d.camera_looking_back(self.camera_mode));
        if looking_back && !self.tuning.show_hud_on_look_back {
            return None;
        }
        self.hud_frame()
    }
    pub(super) fn scene_ui(&mut self, ctx: &egui::Context) -> f32 {
        if self.career.garage.is_some() {
            return 0.0;
        }
        crate::hud::show_interact_prompt(ctx, &self.entry.popup);
        crate::hud::show_mod_message(ctx);
        self.show_pursuit_end(ctx);
        self.show_race_card(ctx);
        self.show_race_message_card(ctx);
        self.show_nis_bars(ctx);
        self.show_story_movie(ctx);
        self.nis.screen().draw(ctx);
        self.show_screen_fade(ctx);
        0.0
    }

    pub(super) fn scene_pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        if self.garage_backdrop.is_some() {
            return None;
        }
        let rain = self.rain.intensity();
        let look = self.player_look(&self.speech_look).unwrap_or_default();
        let mut feed = self.traffic.as_ref()?.pursuit_feed()?;
        feed.rain_intensity = rain;
        feed.speech.player.car_type = look.car_type;
        feed.speech.player.car_colour = look.colour;
        Some(feed)
    }

    pub(super) fn scene_roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        if self.garage_backdrop.is_some() {
            return None;
        }
        let look = self.player_look(&self.speech_look).unwrap_or_default();
        let traffic = self.traffic.as_mut()?;
        let tuning = traffic.roam_tuning();
        let mut feed = traffic.roam_feed(&tuning)?;
        feed.player.car_type = look.car_type;
        feed.player.car_colour = look.colour;
        Some(feed)
    }
}
