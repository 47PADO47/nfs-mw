//! The world's side of the NIS player: where a console-started scene stands, the skip and the bars, and the status line.
//! The player's own rules are in `player.rs`.

use blackbox_gfx::RenderBackend;
use glam::Vec3;

use super::player::NisKind;
use crate::input::{Action, ActionState};
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::scene_space::Anchor;
use crate::scenes::world::{WorldScene, space};

/// The career-flow trace's line for a scene that was asked to start: `nis_start`, or `nis_failed` with the reason.
fn trace_nis_start(name: &str, track: u32, played: &Result<(), String>) {
    let line = serde_json::json!({"scene": name, "track": track, "error": played.as_ref().err()});
    let kind = if played.is_ok() { "nis_start" } else { "nis_failed" };
    crate::parity::career_trace::record(kind, line);
}

/// How far below its probe point a character's ground is looked for, in metres [ours]: the probe starts 2 m above the
/// pelvis, so this reaches a few metres under the feet.
const CHARACTER_GROUND_BELOW: f32 = 6.0;

impl WorldScene {
    /// Starts the NIS scene `name` with its camera `track`, standing it where the player's car is, on the ground and
    /// facing the way the car faces. Refused without a car, or while another scene plays.
    pub(in crate::scenes::world) fn play_nis(&mut self, name: &str, track: u32) -> Result<(), String> {
        self.play_nis_as(name, track, NisKind::of_scene(name))
    }

    /// [`WorldScene::play_nis`] with the scene's `kind` given: the bust's arrest scene is an arrest whatever its name.
    fn play_nis_as(&mut self, name: &str, track: u32, kind: NisKind) -> Result<(), String> {
        let drive = self.players.lead().ok_or("not driving (use the drive command)")?;
        let view = drive.player_view();
        let (collision, _) = self.residency.world_parts();
        let at = view.position;
        let ground =
            space::ground_below(collision, at.x, at.y, at.z + space::CAR_GROUND_ABOVE, at.z - 10.0).unwrap_or(at.z);
        let anchor = Anchor::facing(Vec3::new(at.x, at.y, ground), heading_angle(view.forward));
        let played = self.nis.play(&self.dir, name, anchor, kind, track);
        trace_nis_start(name, track, &played);
        played
    }

    /// Plays the arrest scene the bust asks for (`busted-sequence.md` section 4), as an arrest NIS (not skippable).
    /// The scene uses the queried road cross-section midpoint, facing the player's car. An unavailable road or scene
    /// is answered immediately, so the existing no-scene payment/race outcome can continue.
    pub(in crate::scenes::world) fn update_arrest_nis(&mut self) {
        let Some(name) = self.traffic.as_mut().and_then(|traffic| traffic.take_arrest_request()) else { return };
        if self.coop.is_some() {
            // Co-op has no arrest scene: the pursuit camera's arrest shot shows in the busted player's half instead.
            log::info!("co-op: the arrest scene {name} is not played; the bust is paid without a ticket screen");
            if let Some(traffic) = self.traffic.as_mut() {
                traffic.arrest_scene_over();
            }
            return;
        }
        let anchor = self
            .players
            .lead()
            .zip(self.traffic.as_ref())
            .and_then(|(drive, traffic)| traffic.arrest_anchor(drive.ai_body()))
            .map(|anchor| self.grounded_anchor(anchor));
        let result = anchor
            .ok_or_else(|| "no arrest road location".to_owned())
            .and_then(|anchor| self.nis.play(&self.dir, name, anchor, NisKind::Arrest, 0));
        let Err(error) = result else { return };
        log::warn!("busted: the arrest scene {name} cannot play ({error}); the ticket follows the hold");
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.arrest_scene_over();
        }
    }

    /// `anchor` with its height set to the ground under its origin, as the original stands a scene's origin on the
    /// ground face under the marker (`nis-player.md` section 5.2) [decomp]. The road's cross-section height is the road
    /// network's, which need not be the height of the road's faces. Without a ground the height stays.
    fn grounded_anchor(&mut self, anchor: Anchor) -> Anchor {
        let (collision, _) = self.residency.world_parts();
        let at = anchor.origin;
        let ground = space::ground_below(collision, at.x, at.y, at.z + space::CAR_GROUND_ABOVE, at.z - 10.0);
        Anchor { origin: Vec3::new(at.x, at.y, ground.unwrap_or(at.z)), ..anchor }
    }

    /// Starts the NIS scene `name` with its camera `track`, standing at `anchor`. Refused while another scene plays.
    pub(in crate::scenes::world) fn play_nis_at(
        &mut self,
        name: &str,
        track: u32,
        anchor: Anchor,
    ) -> Result<(), String> {
        let played = self.nis.play(&self.dir, name, anchor, NisKind::of_scene(name), track);
        trace_nis_start(name, track, &played);
        played
    }

    /// Ends the scene that plays. Returns whether one did.
    pub(in crate::scenes::world) fn stop_nis(&mut self) -> bool {
        self.nis.stop()
    }

    /// The console line about the NIS player.
    pub(in crate::scenes::world) fn nis_status(&self) -> String {
        self.nis.status()
    }

    /// Uploads the playing scene's characters, with the world's shared textures, and frees the ones of scenes that
    /// ended. Runs every frame, before the frame is built.
    pub(in crate::scenes::world) fn update_nis_characters(&mut self, renderer: &mut dyn RenderBackend) {
        self.nis.prepare_characters(renderer, &self.residency.shared.materials);
    }

    /// Loads the police car model a playing scene's cop channels wait for. The world freezes while a scene plays, so the
    /// traffic's own loading (`prepare_traffic`) does not run then; this loads the models the scene asked for. Runs every
    /// frame, before the frame is built.
    pub(in crate::scenes::world) fn update_nis_models(&mut self, renderer: &mut dyn RenderBackend) {
        if self.nis.cop_waiting().is_empty() {
            return;
        }
        let submit = self.residency.complete();
        crate::scenes::world::ai::load_requested(self, renderer, submit, false);
    }

    /// Reads the player's skip and plays the scene on real frame time. Runs before the world reads the frame's input,
    /// so the world can freeze while a scene plays.
    pub(in crate::scenes::world) fn update_nis(&mut self, input: &ActionState, dt: f32) {
        let skip = input.just_pressed(Action::MenuAccept) || input.just_pressed(Action::MenuStart);
        if skip || crate::parity::career_trace::skip_cinematics() {
            self.nis.skip();
        }
        let ending = self.nis.playing_name().map(str::to_owned);
        let events = self.nis.advance(self.nis_capture_step(dt));
        self.apply_nis_events(events);
        if let Some(outcome) = self.nis.take_outcome() {
            log::info!("NIS scene ended: {outcome:?}");
            crate::parity::career_trace::record("nis_end", serde_json::json!({"outcome": format!("{outcome:?}")}));
            // A bust waiting on its arrest scene goes on; the bust ignores any other scene's end.
            if let Some(traffic) = self.traffic.as_mut() {
                traffic.arrest_scene_over();
            }
            // A scene that faded itself to black leaves the screen black until the next race or movie takes it.
            if self.nis.screen_mut().take_held_black() {
                self.screen_fade.set_black();
            }
            self.finish_nis_cars();
            if let Some(name) = ending.filter(|_| crate::parity::nis_capture::enabled()) {
                self.note_nis_capture_end(&name);
            }
        }
        if self.nis.is_playing() {
            self.link_nis_cars();
            self.retry_nis_cops(dt);
            self.drive_nis_cars();
            self.ground_nis_characters();
        }
        if self.nis.take_cut() {
            self.cut.arm();
        }
    }

    /// Stands the playing scene's characters on the ground under their pelvis (`nis-player-2.md` section 9.3): the first
    /// drivable face going down from the probe point to [`CHARACTER_GROUND_BELOW`] below it.
    fn ground_nis_characters(&mut self) {
        let (collision, _) = self.residency.world_parts();
        self.nis.keep_characters_on_ground(|at| {
            space::ground_below(collision, at.x, at.y, at.z, at.z - CHARACTER_GROUND_BELOW)
        });
    }

    /// Whether the scene's camera has letterbox bars up.
    pub(in crate::scenes::world) fn nis_bars_up(&self) -> bool {
        self.nis.letterbox() > 0.0
    }

    /// Draws the letterbox bars of the scene's camera over the frame.
    pub(in crate::scenes::world) fn show_nis_bars(&self, ctx: &egui::Context) {
        if !self.nis_bars_up() {
            return;
        }
        let bar = self.nis.letterbox();
        let screen = ctx.content_rect();
        let height = screen.height() * bar;
        let top = egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, screen.min.y + height));
        let bottom = egui::Rect::from_min_max(egui::pos2(screen.min.x, screen.max.y - height), screen.max);
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("nis_letterbox")));
        for bars in [top, bottom] {
            painter.rect_filled(bars, 0.0, egui::Color32::BLACK);
        }
    }
}
