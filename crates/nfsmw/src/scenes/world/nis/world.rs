//! The world's side of the NIS player: where a console-started scene stands, the skip and the bars, and the status line.
//! The player's own rules are in `player.rs`.

use glam::Vec3;

use super::player::NisKind;
use crate::input::{Action, ActionState};
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::scene_space::Anchor;
use crate::scenes::world::{WorldScene, space};

impl WorldScene {
    /// Starts the NIS scene `name` with its camera `track`, standing it where the player's car is, on the ground and
    /// facing the way the car faces. Refused without a car, or while another scene plays.
    pub(in crate::scenes::world) fn play_nis(&mut self, name: &str, track: u32) -> Result<(), String> {
        self.play_nis_as(name, track, NisKind::of_scene(name))
    }

    /// [`WorldScene::play_nis`] with the scene's `kind` given: the bust's arrest scene is an arrest whatever its name.
    fn play_nis_as(&mut self, name: &str, track: u32, kind: NisKind) -> Result<(), String> {
        let drive = self.drive.as_ref().ok_or("not driving (use the drive command)")?;
        let view = drive.player_view();
        let (collision, _) = self.residency.world_parts();
        let at = view.position;
        let ground = space::ground_below(collision, at.x, at.y, at.z + 10.0, at.z - 10.0).unwrap_or(at.z);
        let anchor = Anchor::facing(Vec3::new(at.x, at.y, ground), heading_angle(view.forward));
        self.nis.play(&self.dir, name, anchor, kind, track)
    }

    /// Plays the arrest scene the bust asks for (`busted-sequence.md` section 4), as an arrest NIS (not skippable).
    /// The spec's arrest-spot search is not in the port yet, so the scene is anchored at the player's pose instead: the
    /// player's car position on the ground, facing the car's heading (see [`WorldScene::play_nis`]). A scene that cannot
    /// play (no car, no bundle, another scene on) is answered at once, so the bust keeps its plain hold.
    pub(in crate::scenes::world) fn update_arrest_nis(&mut self) {
        let Some(name) = self.traffic.as_mut().and_then(|traffic| traffic.take_arrest_request()) else { return };
        let Err(error) = self.play_nis_as(name, 0, NisKind::Arrest) else { return };
        log::warn!("busted: the arrest scene {name} cannot play ({error}); the ticket follows the hold");
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.arrest_scene_over();
        }
    }

    /// Starts the NIS scene `name` with its camera `track`, standing at `anchor`. Refused while another scene plays.
    pub(in crate::scenes::world) fn play_nis_at(
        &mut self,
        name: &str,
        track: u32,
        anchor: Anchor,
    ) -> Result<(), String> {
        self.nis.play(&self.dir, name, anchor, NisKind::of_scene(name), track)
    }

    /// Ends the scene that plays. Returns whether one did.
    pub(in crate::scenes::world) fn stop_nis(&mut self) -> bool {
        self.nis.stop()
    }

    /// The console line about the NIS player.
    pub(in crate::scenes::world) fn nis_status(&self) -> String {
        self.nis.status()
    }

    /// Reads the player's skip and plays the scene on real frame time. Runs before the world reads the frame's input,
    /// so the world can freeze while a scene plays.
    pub(in crate::scenes::world) fn update_nis(&mut self, input: &ActionState, dt: f32) {
        if input.just_pressed(Action::MenuAccept) || input.just_pressed(Action::MenuStart) {
            self.nis.skip();
        }
        let events = self.nis.advance(dt);
        self.apply_nis_events(events);
        if let Some(outcome) = self.nis.take_outcome() {
            log::info!("NIS scene ended: {outcome:?}");
            // A bust waiting on its arrest scene goes on; the bust ignores any other scene's end.
            if let Some(traffic) = self.traffic.as_mut() {
                traffic.arrest_scene_over();
            }
            self.finish_nis_cars();
        }
        if self.nis.is_playing() {
            self.link_nis_cars();
            self.drive_nis_cars();
        }
        if self.nis.take_cut() {
            self.cut.arm();
        }
    }

    /// Draws the letterbox bars of the scene's camera over the frame.
    pub(in crate::scenes::world) fn show_nis_bars(&self, ctx: &egui::Context) {
        let bar = self.nis.letterbox();
        if bar <= 0.0 {
            return;
        }
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
