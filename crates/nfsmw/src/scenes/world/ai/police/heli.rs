//! The police helicopter in the chase: when it joins, how it is thought each frame and when it leaves. The model and its
//! rules are in `blackbox_pursuit::helicopter`; its tuning per heat and race level is in `settings::helicopter`. Child of
//! `police`, so it reads the chase's state directly.

use blackbox_pursuit::helicopter::control::{Flat, Heights};
use blackbox_pursuit::helicopter::{Helicopter, Mode, SearchInput, THINK_PERIOD, Target, Tuning};
use glam::Vec3;

use super::helpers::{SheetFloor, horizontal, to_body};
use super::{PlayerView, Police, Sensed};

/// Heat levels with a helicopter in their cop mix (`pursuitlevels`, [verified]: heat 4 to 6 and 8 to 10).
const HELI_LEVELS: [u32; 6] = [4, 5, 6, 8, 9, 10];
/// Heat levels whose `pursuitsupport` has a skid-hit air entry (`SKID_HIT`, [verified]).
const SKID_LEVELS: [u32; 5] = [5, 6, 7, 9, 10];
/// How high above the player it spawns.
const SPAWN_HEIGHT: f32 = 20.0;
/// Turns the search circles per search point (0.125 of a turn, the spec's step).
const SEARCH_STEP_TURNS: f32 = 0.125;
/// The race table is not used yet: no race code calls the police, so the chase always reads the heat table.
const RACING: bool = false;

impl Police {
    /// Whether a helicopter is in the chase, flying or leaving.
    pub fn helicopter_present(&self) -> bool {
        self.heli.is_some()
    }

    /// Replaces the helicopter tuning (read from the file when the track opens).
    pub fn set_heli_settings(&mut self, settings: crate::settings::helicopter::HeliSettings) {
        self.heli_settings = settings;
    }

    pub(super) fn think_helicopter(
        &mut self,
        dt: f32,
        pursuing: bool,
        player: &PlayerView,
        sensed: Sensed,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) {
        if self.heli.is_none() && pursuing && sensed.seen {
            self.try_join(player);
        }
        let Some(mut heli) = self.heli.take() else { return };
        let from = heli.body.position;
        let line = |to: Vec3| clear(from, to + Vec3::Y);
        let body = to_body(player);
        let flat = Flat(body.position.y - 2.0);
        let sheet = SheetFloor(self.heli_sheet.as_ref());
        let heights: &dyn Heights = match self.heli_settings.heli_sheet && self.heli_sheet.is_some() {
            true => &sheet,
            false => &flat,
        };
        let target = Target { hidden: sensed.hidden_from_heli, in_tunnel: sensed.in_tunnel };
        self.heli_clock += dt;
        let mut gone = false;
        while self.heli_clock >= THINK_PERIOD && !gone {
            self.heli_clock -= THINK_PERIOD;
            self.search_turns = (self.search_turns + SEARCH_STEP_TURNS).fract();
            let search = SearchInput { angle_turns: self.search_turns, unit_random: sensed.unit_random };
            gone = heli.think(THINK_PERIOD, &body, target, &line, heights, search);
        }
        if gone {
            self.heli_wait = self.heli_settings.respawn_secs(self.level().unwrap_or(1), RACING);
            return;
        }
        if heli.too_far(&body) && !matches!(heli.mode, Mode::Exit(_)) {
            heli.leave();
        }
        self.heli = Some(heli);
    }

    /// Sends a helicopter in when the level has one, its wait is over and the chase is on. It spawns `spawn_distance`
    /// metres behind the player on the road.
    pub(super) fn try_join(&mut self, player: &PlayerView) {
        let Some(level) = self.level() else { return };
        if !self.heli_available || self.heli_wait > 0.0 || !HELI_LEVELS.contains(&level) {
            return;
        }
        let tuning = Tuning { skid_hit: SKID_LEVELS.contains(&level), ..self.heli_settings.tuning(level, RACING) };
        let body = to_body(player);
        let behind = body.position - horizontal(body.forward).normalize_or_zero() * tuning.spawn_distance
            + Vec3::new(0.0, SPAWN_HEIGHT, 0.0);
        let heading = body.forward.z.atan2(body.forward.x);
        self.heli = Some(Helicopter::spawn(behind, heading, tuning));
        self.on_helicopter();
    }
}
