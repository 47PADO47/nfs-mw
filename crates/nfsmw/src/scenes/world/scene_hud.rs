//! Visibility and composition of the world HUD.
use nfsmw_data::world::DEFAULT_TRACK;

use super::WorldScene;

impl WorldScene {
    pub(super) fn note_police_speech_now(&mut self, phase: Option<blackbox_pursuit::siren::Phase>) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.note_police_speech(phase);
        }
    }

    pub(super) fn scene_hud(&self) -> Option<crate::hud::HudState> {
        if self.career.garage.is_some() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        // A race's message card is the HUD's own package, drawn alone over the frame.
        if self.race_card_text().is_some() {
            return Some(crate::hud::HudState { visible: false, message_card: true, ..Default::default() });
        }
        // `None` would show the viewer's idle HUD, so a playing scene answers with a hidden one.
        if self.cutscene_playing() {
            // An arrest keeps the busted overlay up for the whole scene, and its overlay events cue the package.
            // A camera key with overlay 3 puts the same police-camera overlay up in another scene: every key of every
            // arrest's track has it, and the D-Day start's first shot (install data); the original shows the overlay
            // over exactly that shot (reference frames).
            let arrest = self.nis.arrest_playing();
            let camera = self.nis.camera_overlay() == Some(POLICE_CAMERA_OVERLAY);
            let cue = self.nis.screen().overlay_cue(crate::hud::OVERLAY_PACKAGE).filter(|_| arrest || camera);
            let summary = self.traffic.as_ref().filter(|_| arrest).map(super::hud_pursuit::chase_summary);
            return Some(crate::hud::HudState {
                visible: false,
                busted_overlay: arrest || camera,
                busted_overlay_cue: cue,
                busted_summary: summary,
                busted_overlay_text: (camera && !arrest).then_some(POLICE_CAMERA_CRAWL),
                ..Default::default()
            });
        }
        if self.moment.hides_hud() || self.moment_shots.cams.hides_hud() {
            return None;
        }
        let looking_back = self.players.lead().is_some_and(|d| d.camera_looking_back(self.camera_mode));
        if looking_back && !self.tuning.show_hud_on_look_back {
            return None;
        }
        let mut state = self.hud_frame()?;
        // The minimap is the city's picture: a second world has none (`docs/specs/second-world.md`).
        if self.track != DEFAULT_TRACK {
            state.minimap = None;
            state.map_car = None;
            state.second_world = true;
        }
        Some(state)
    }
    pub(super) fn scene_ui(&mut self, ctx: &egui::Context) -> f32 {
        if self.career.garage.is_some() {
            return 0.0;
        }
        // Under the Custom HUD the theme draws these cards from the HUD state (`hud/cards.rs`); a mod's line still
        // counts its time down here.
        match crate::hud::cards_routed() {
            true => crate::hud::advance_mod_message(ctx.input(|input| input.stable_dt)),
            false => {
                crate::hud::show_interact_prompt(ctx, &self.entry.popup);
                crate::hud::show_mod_message(ctx);
                self.show_pursuit_end(ctx);
                self.show_race_card(ctx);
            }
        }
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
        let at = glam::Vec3::from_array(feed.speech.player.position);
        feed.speech.player.speech_map_id = self
            .audio_environment
            .as_ref()
            .map_or(crate::audio::environment_context::NO_SPEECH_PLACE, |map| map.speech_place(at));
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

/// The camera key overlay byte of the police-camera look (`BUSTED_OVERLAY.fng`).
const POLICE_CAMERA_OVERLAY: u8 = 3;
/// The crawl of the police camera outside an arrest: "STATUS: CODE 3 PURSUIT – DISTRICT: ROCKPORT POLICE DEPARTMENT –
/// CONDITION: 3 – SUSPECTS: 2 VEHICLES", the string after the five arrest crawls (`hud/busted_line.rs`); the original's
/// D-Day start shows it (reference frames).
const POLICE_CAMERA_CRAWL: u32 = 0x626A_C047;
