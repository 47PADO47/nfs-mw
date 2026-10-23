//! What the server loop asks of the real world beyond `WorldSim` (`nfsmw_server::WorldControl`): a player's reset, the
//! parked ghost of a dropped player, and the player's view of the chase and the race for the global block.

use blackbox_pursuit::CooldownStatus;
use nfsmw_net::frames::{GlobalBlock, PursuitStatus};
use nfsmw_server::{RaceRequest, WorldControl};
use nfsmw_sim::PlayerId;

use super::super::race::RaceOptions;
use super::host::WorldSimHost;
use super::sim::{chase_state, racer_states};

/// Minutes in a day: the global block's time of day is under it.
const DAY_MINUTES: f32 = 1440.0;
/// The highest heat the global block carries.
const MAX_HEAT: f32 = 15.0;

impl WorldControl for WorldSimHost {
    /// The reset key of the player's car on the next tick ([`WorldSimHost::request_reset`]): back on the nearest road,
    /// or the catch-up warp while the prompt is offered. Refused for a player with no car in the world.
    fn reset_player(&mut self, id: PlayerId) -> bool {
        if self.scene.players.get(id).is_none() {
            return false;
        }
        self.request_reset(id);
        true
    }

    /// A dropped player's car is parked while its slot waits for a resume (MP-50): out of play, it neither steps,
    /// collides, shows nor is exported (co-op's parking, `players/mod.rs`). Unparking brings it back where it stood.
    fn set_ghost(&mut self, id: PlayerId, ghost: bool) -> bool {
        self.scene.players.park(id, ghost)
    }

    /// Starts the race the track id names, with the session's laps and traffic, and the players of the request as its
    /// people beside the lead (`players/race.rs`). The world runs the grid, the intro and the countdown itself.
    fn start_race(&mut self, request: &RaceRequest) -> Result<(), String> {
        let data = self.scene.gameplay.clone().ok_or("the race data could not be read from the install")?;
        let race = data
            .races
            .iter()
            .find(|race| blackbox_feng::fe_hash_upper(&race.name) == request.track)
            .cloned()
            .ok_or_else(|| format!("no race has the track id {:#010x}", request.track))?;
        let options = RaceOptions {
            laps: (request.laps > 0).then_some(u32::from(request.laps)),
            traffic: Some(i32::from(request.traffic_density)),
            ..RaceOptions::default()
        };
        // The grid stands and waits: the server schedules GO (`schedule_race_go`) once everyone has loaded.
        self.scene.hold_next_race(true);
        let begun = self.scene.begin_race(race, options, None);
        if begun.is_err() {
            self.scene.hold_next_race(false);
        }
        begun.map(drop)
    }

    /// The race is ready to count down: its racers stand on the grid. True too when there is no race to wait for (it
    /// could not start, or it is gone).
    fn race_staged(&self) -> bool {
        self.scene.race.is_none() || self.scene.staged_race().is_some()
    }

    /// The countdown starts as late as it can while GO still falls on `go_tick` (at once when that is too close).
    fn schedule_race_go(&mut self, go_tick: u32) {
        self.race_go_at = Some(go_tick);
    }

    /// Ends the race the world runs; the people's cars come back behind the lead.
    fn stop_race(&mut self) {
        self.race_go_at = None;
        self.scene.hold_next_race(false);
        self.scene.abandon_race();
    }

    /// The world's clock, and for a chased player the heat, the busted meter (its own while two or more are chased,
    /// ADR 0010 section 4), the cooldown bar and the chase's state; for a racer its place and lap.
    fn fill_global(&self, id: PlayerId, global: &mut GlobalBlock) {
        let scene = &self.scene;
        global.time_of_day = (scene.day.hour() * 60.0).rem_euclid(DAY_MINUTES);
        if let Some(place) = race_place(self, id) {
            (global.place, global.lap) = place;
        }
        let Some(traffic) = scene.traffic.as_ref() else { return };
        let chase = chase_state(scene);
        let busted = chase.busted.contains(&id);
        let chased = chase.chased.contains(&id);
        if !busted && !chased {
            return;
        }
        global.heat = traffic.heat_value().round().clamp(0.0, MAX_HEAT) as u8;
        global.busted = scene.busted_meter_for(id, traffic).clamp(0.0, 1.0);
        global.cooldown = traffic.cooldown_bar().clamp(0.0, 1.0);
        global.pursuit = match (busted, traffic.cooldown_status()) {
            (true, _) => PursuitStatus::Busted,
            (false, CooldownStatus::CoolDown) => PursuitStatus::Cooldown,
            (false, CooldownStatus::Evaded) => PursuitStatus::Evaded,
            (false, CooldownStatus::InitialChase) => PursuitStatus::Chased,
        };
    }
}

/// The player's place (from 1) and lap (from 1) in the running race, if it races.
fn race_place(host: &WorldSimHost, id: PlayerId) -> Option<(u8, u8)> {
    let racers = racer_states(&host.scene)?;
    let index = racers.iter().position(|racer| racer.player == Some(id))?;
    let hud = host.scene.race_hud_for(index)?;
    let lap = hud.lap.map_or(0, |(lap, _)| lap);
    Some((u8::try_from(hud.place).unwrap_or(u8::MAX).min(31), u8::try_from(lap).unwrap_or(u8::MAX).min(63)))
}
