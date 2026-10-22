//! The pursuit readout of the HUD, shared by the lead's HUD (`hud.rs`) and a co-op guest's (`players/view.rs`): the
//! chase's board and each player's busted meter (`docs/specs/coop-career.md` section 5).

use super::WorldScene;
use super::ai::TrafficWorld;
use super::players::{PlayerId, coop::GUEST};
use crate::audio::PursuitFeed;
use crate::hud::{HudState, PursuitHud};

/// The chase as the pursuit board shows it: the timer and the wrecked cops. The evade share is what the feed gives (the
/// part of the timeout done); the board's bar takes the cool-down meter instead, see [`pursuit_board`].
fn board_from_feed(feed: PursuitFeed, cops_destroyed: u32) -> PursuitHud {
    PursuitHud {
        cops_destroyed,
        ..PursuitHud::from_evade_time(feed.secs, feed.secs_to_evade, blackbox_pursuit::evade::timeout(feed.heat))
    }
}

/// The chase's pursuit board, the same for every player it chases: the timer, the cool-down bar, the hiding mark and
/// the cops wrecked. `busted` is the meter this player's board shows. Once the arrest runs the board reads busted.
/// `None` while no cop chases.
pub(super) fn pursuit_board(traffic: &TrafficWorld, busted: f32) -> Option<PursuitHud> {
    let wrecked = traffic.cops_destroyed();
    if traffic.arrest_active() {
        return Some(PursuitHud {
            secs: traffic.pursuit_time(),
            evade: 0.0,
            cops_destroyed: wrecked,
            busted: 1.0,
            cooldown: false,
            hiding: None,
        });
    }
    // The bar spans the cool-down's own length (`required - grace`), so it opens empty; the meter
    // (`evade_level`) starts at the grace's share (`docs/specs/ai-pursuit.md` section 6).
    traffic.pursuit_feed().map(|feed| PursuitHud {
        evade: traffic.cooldown_bar(),
        busted,
        cooldown: traffic.cooldown_shown(),
        hiding: traffic.hiding_fill(),
        ..board_from_feed(feed, wrecked)
    })
}

impl WorldScene {
    /// The busted meter of player `id` as its HUD shows it, 0 to 1: its own meter while a co-op chase holds two or more
    /// targets; otherwise the police's meter when the police follow `id`; otherwise zero.
    pub(in crate::scenes::world) fn busted_meter_for(&self, id: PlayerId, traffic: &TrafficWorld) -> f32 {
        if let Some(session) = self.coop.as_ref() {
            let tuning = traffic.bust_tuning();
            if let Some(meter) = session.chase.meter(id, &tuning) {
                return meter;
            }
        }
        if self.chase_primary() != id {
            return 0.0;
        }
        traffic.busted_meter()
    }

    /// The co-op guest's chase readout. It must run before the career's HUD values, which leave a chase's heat alone:
    /// while a chase is on and the guest is one of its targets, the shared board and the shared heat. The busted meter
    /// is set every frame.
    pub(in crate::scenes::world) fn guest_chase_hud(&self, traffic: &TrafficWorld, state: &mut HudState) {
        state.busted = self.busted_meter_for(GUEST, traffic);
        let chased = self.coop.as_ref().is_some_and(|session| session.chase.is_chased(GUEST));
        if !(traffic.is_pursuing() && chased) {
            return;
        }
        let Some(drive) = self.players.first_guest() else { return };
        let view = drive.player_view();
        state.pursuit = pursuit_board(traffic, traffic.pursuit_meter(view.position, view.speed));
        state.vehicle_heat = traffic.vehicle_heat();
    }
}
