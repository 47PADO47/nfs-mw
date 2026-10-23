//! The HUD of a client world: the server's global block (heat, busted and cooldown meters, the chase's state, the race
//! place and lap) over the car's own dials, the session's gameplay lines in the message group, and the remote players
//! on the minimap. The car's dials stay the predicted car's (`Drive::hud_state`).

use nfsmw_net::client_session::GlobalView;
use nfsmw_net::frames::PursuitStatus;
use nfsmw_net::message::Message;
use nfsmw_sim::EntityKind;

use super::super::WorldScene;
use crate::hud::{HudState, PursuitHud, RaceHud};

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
        Message::RaceStart(start) => Some(format!("GO! {} racers", start.order.len())),
        Message::RaceFinish(finish) => Some(format!("Car {} finished in place {}", finish.entity_id, finish.place)),
        Message::PursuitStart(start) => Some(format!("Pursuit! Heat {}", start.heat)),
        Message::PursuitEnd(end) => Some(format!("Pursuit over: {:?}", end.reason)),
        Message::PlayerJoined(joined) => Some(format!("{} joined", joined.name.as_str())),
        Message::PlayerLeft(left) => Some(format!("Player {} left", u16::from(left.slot) + 1)),
        _ => None,
    }
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
