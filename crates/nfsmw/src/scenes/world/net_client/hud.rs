//! The HUD of a client world: the server's global block (heat, busted and cooldown meters, the chase's state, the race
//! place and lap) over the car's own dials, the session's gameplay lines in the message group, and the remote players
//! on the minimap. The car's dials stay the predicted car's (`Drive::hud_state`).

use nfsmw_net::client_session::GlobalView;
use nfsmw_net::frames::PursuitStatus;
use nfsmw_net::message::{Message, PursuitEndReason};
use nfsmw_sim::EntityKind;

use super::super::WorldScene;
use crate::hud::{HudState, PursuitHud, RaceHud};
use crate::i18n::{self, Args, t};

/// Minutes in a day (`GlobalBlock::time_of_day` is under this).
const DAY_MINUTES: f32 = 1_440.0;

/// The serial of the client's message lines: one line at a time, a new text restarts the fade.
const SERIAL_BASE: u32 = 0x4E45_5400;

impl WorldScene {
    /// Writes the client's view of the session into `state`; nothing outside client mode.
    pub(in crate::scenes::world) fn apply_net_hud(&self, state: &mut HudState) {
        let Some(mode) = self.net_client.as_ref() else { return };
        if let Some(global) = mode.global.as_ref() {
            apply_global(global, state);
        }
        if let Some((line, _)) = mode.banner.as_ref() {
            let serial = SERIAL_BASE ^ (line.len() as u32);
            state.message = Some(crate::hud::Message { serial, text: line.clone() });
        }
        state.second_player =
            mode.display().iter().find(|(_, drawn)| drawn.view.kind == EntityKind::PlayerCar).map(|(_, drawn)| {
                let pose = super::frame::car_pose(drawn.view.position, drawn.view.orientation);
                crate::scenes::world::split::map_position(pose.position, pose.rotation * glam::Vec3::X)
            });
    }
}

/// How far the client's hour may stray from the server's before it is set to it, in minutes of the day. Both clocks run
/// at the same rate, so only a join, a pause or a stall shows as a difference.
const TIME_TOLERANCE_MINUTES: f32 = 5.0;

impl WorldScene {
    /// Sets the day clock to the server's hour of day when it has strayed from it (the lobby's or the world's time of
    /// day, `GlobalBlock::time_of_day`); in between the clock runs on its own.
    pub(in crate::scenes::world) fn follow_server_time(&mut self) {
        let Some(server) = self.net_client.as_ref().and_then(|mode| mode.global.as_ref()).map(|g| g.time_of_day) else {
            return;
        };
        if !server.is_finite() {
            return;
        }
        if !strayed((self.day.hour() * 60.0).rem_euclid(DAY_MINUTES), server) {
            return;
        }
        let environment = crate::viewer::Environment { hour: server / 60.0, ..self.day.environment() };
        self.configure_day(environment);
    }
}

/// Whether two minutes of the day are further apart than the tolerance, across midnight too.
pub(super) fn strayed(ours: f32, server: f32) -> bool {
    let apart = (server - ours).abs();
    apart.min(DAY_MINUTES - apart) > TIME_TOLERANCE_MINUTES
}

/// The global block's pursuit and race fields as the HUD shows them.
pub fn apply_global(global: &GlobalView, state: &mut HudState) {
    state.vehicle_heat = f32::from(global.heat);
    state.busted = global.busted;
    let chased = matches!(global.pursuit, PursuitStatus::Chased | PursuitStatus::Cooldown);
    state.pursuit = chased.then(|| PursuitHud {
        evade: global.cooldown,
        busted: match global.pursuit {
            PursuitStatus::Cooldown => -global.cooldown,
            _ => global.busted,
        },
        cooldown: global.pursuit == PursuitStatus::Cooldown,
        ..PursuitHud::default()
    });
    state.busted_overlay = global.pursuit == PursuitStatus::Busted;
    if global.place == 0 {
        return;
    }
    let race =
        state.race.get_or_insert(RaceHud { place: 0, racers: 0, lap: None, clock: 0.0, progress: 0.0, count: None });
    race.place = u32::from(global.place);
    race.racers = race.racers.max(race.place);
    if global.lap > 0 {
        let laps = race.lap.map_or(0, |(_, laps)| laps);
        race.lap = Some((u32::from(global.lap), laps.max(u32::from(global.lap))));
    }
}

/// The line a gameplay event shows, if it shows one.
pub fn game_line(message: &Message) -> Option<String> {
    match message {
        Message::RaceStart(start) => Some(t!("lan-net-race-start", racers = start.order.len())),
        Message::RaceFinish(finish) => Some(t!("lan-net-race-finish", car = finish.entity_id, place = finish.place)),
        Message::PursuitStart(start) => Some(t!("lan-net-pursuit-start", heat = start.heat)),
        Message::PursuitEnd(end) => Some(pursuit_end_line(end.reason)),
        Message::PlayerJoined(joined) => Some(t!("lan-notice-join", who = joined.name.as_str())),
        Message::PlayerLeft(left) => Some(t!("lan-net-player-left", player = u16::from(left.slot) + 1)),
        _ => None,
    }
}

/// How a pursuit ended, in words of its own for each way.
fn pursuit_end_line(reason: PursuitEndReason) -> String {
    let id = match reason {
        PursuitEndReason::Busted => "lan-net-pursuit-busted",
        PursuitEndReason::Evaded => "lan-net-pursuit-evaded",
        PursuitEndReason::Abandoned => "lan-net-pursuit-abandoned",
    };
    i18n::text(id, &Args::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chase_shows_heat_the_meters_and_the_board() {
        let global = GlobalView { heat: 3, busted: 0.5, pursuit: PursuitStatus::Chased, ..GlobalView::default() };
        let mut state = HudState::default();
        apply_global(&global, &mut state);
        assert_eq!((state.vehicle_heat, state.busted), (3.0, 0.5));
        let board = state.pursuit.expect("a chase shows the board");
        assert!(!board.cooldown && board.busted == 0.5);
        assert!(state.race.is_none(), "no race without a place");
    }

    #[test]
    fn cooldown_counts_down_and_free_roam_has_no_board() {
        let global = GlobalView { cooldown: 0.25, pursuit: PursuitStatus::Cooldown, ..GlobalView::default() };
        let mut state = HudState::default();
        apply_global(&global, &mut state);
        assert!(state.pursuit.is_some_and(|board| board.cooldown && board.busted == -0.25));
        apply_global(&GlobalView::default(), &mut state);
        assert!(state.pursuit.is_none());
    }

    #[test]
    fn a_race_shows_the_place_and_the_lap() {
        let global = GlobalView { place: 2, lap: 3, ..GlobalView::default() };
        let mut state = HudState::default();
        apply_global(&global, &mut state);
        let race = state.race.expect("a race");
        assert_eq!((race.place, race.lap), (2, Some((3, 3))));
    }
}
