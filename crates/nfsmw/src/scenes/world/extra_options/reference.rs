//! The F5 pursuit reference in the world: the bounty places of the install, the rap sheet's jump to one of them, the
//! pursuit started once the car stands there, and the heat keys.
use super::*;
use crate::extra_options::reference;

impl WorldScene {
    /// Jumps to bounty place `number` as the rap sheet does (`start_at_point`: on the place's marker, facing its
    /// `Rotation`, the car stopped). With `pursuit`, a chase at the testing heat starts once the car stands there.
    pub(super) fn extra_jump_to_spot(&mut self, number: i32, pursuit: bool) -> Result<()> {
        self.load_bounty_spots();
        let spot = self.extra.bounty_spots.iter().find(|spot| spot.number == number).cloned();
        let spot = spot.context("That bounty place is not in the install's data")?;
        if self.race.is_some() {
            bail!("End the current race before jumping");
        }
        if pursuit {
            ensure!(!self.extra.testing.police_off, "Enable police first");
        }
        // A reference run starts clean: the chase running now ends before the jump.
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.stop_pursuit();
        }
        let drive = self.players.lead().context("No player car")?;
        let p = drive.position();
        let previous = Location { name: "Previous position".into(), position: [p.x, p.y], heading: drive.heading() };
        ensure!(self.start_at_point(spot.jump), "No player car");
        self.extra.previous_location = Some(previous);
        self.extra.teleport_heading = None;
        self.view = View::Chase;
        self.extra.freecam = false;
        self.extra.selected_bookmark = None;
        self.cut.arm();
        self.extra.landed = None;
        self.extra.pending_jump = Some((number, pursuit.then_some(self.extra.testing.heat)));
        log::info!(
            "pursuit reference: jumping to bounty place {number:02} ({}) at ({:.0}, {:.0}){}",
            spot.name,
            spot.at[0],
            spot.at[1],
            if pursuit { ", the pursuit starts on landing" } else { "" }
        );
        Ok(())
    }

    /// Page Up / Page Down: the testing heat moves one level and a running chase takes it at once.
    pub(super) fn extra_heat_step(&mut self, delta: i8) {
        let heat = reference::step_heat(self.extra.testing.heat, delta);
        self.extra.testing.heat = heat;
        let police = !self.extra.testing.police_off;
        let applied = police && self.traffic.as_mut().is_some_and(|traffic| traffic.set_extra_heat_once(heat));
        let text = reference::heat_toast(heat, applied);
        log::info!("pursuit reference: {text}");
        self.extra.toast = Some(text);
    }

    /// Reads the install's bounty places once per world.
    pub(super) fn load_bounty_spots(&mut self) {
        if self.extra.bounty_loaded {
            return;
        }
        self.extra.bounty_loaded = true;
        let data = match nfsmw_data::gameplay::cached(&self.dir) {
            Ok(data) => data,
            Err(e) => return log::warn!("pursuit reference: no bounty places ({e:#})"),
        };
        let name = |number: i32| crate::i18n::game(blackbox_text::label_hash(&reference::label(number)));
        self.extra.bounty_spots = reference::spots(&data.bounty_jumps, name);
    }

    /// Each frame: once the jumped car stands on its road, records where and how it faces and starts the pursuit.
    pub(in crate::scenes::world) fn update_pursuit_reference(&mut self, renderer: &mut dyn RenderBackend) {
        self.load_bounty_spots();
        let Some((number, heat)) = self.extra.pending_jump else { return };
        let Some(drive) = self.players.lead() else { return };
        if !drive.has_sim() || drive.waiting_for_road().is_some() {
            return;
        }
        self.extra.pending_jump = None;
        let p = drive.position();
        let heading = drive.heading().to_degrees().rem_euclid(360.0);
        self.extra.landed = Some((number, [p.x, p.y], heading));
        log::info!(
            "pursuit reference: landed at bounty place {number:02}, ({:.1}, {:.1}, {:.1}), heading {heading:.1} deg",
            p.x,
            p.y,
            p.z
        );
        self.pursuit_trace_landed(number, [p.x, p.y, p.z], heading, heat);
        let Some(heat) = heat else { return };
        if self.extra.testing.police_off {
            self.extra.message = Some("Police are disabled; the pursuit was not started".into());
            return;
        }
        let level = heat.to_string();
        // The original's chase after a jump puts its first cop 80 m ahead, facing the car (`ai/pursuit/opening.rs`).
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_jump_start(true);
        }
        let result = super::super::ai::pursuit_command(self, renderer, &[&level]);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_jump_start(false);
        }
        self.extra.message = Some(match result {
            Ok(_) => {
                format!("Pursuit started at heat {heat} at {}", reference::row_text(&self.extra.bounty_spots, number))
            }
            Err(e) => format!("The pursuit did not start: {e}"),
        });
    }
}
