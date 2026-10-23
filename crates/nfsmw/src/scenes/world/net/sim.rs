//! `WorldSim for WorldSimHost`, and what its events read of the world.

use blackbox_pursuit::EndReason;
use blackbox_pursuit::targets::{TargetId, TargetStatus};
use nfsmw_sim::{EntityId, EntityView, OwnState, PlayerId, PlayerJoin, SimError, SimEvent, SimInputs, Tick, WorldSim};

use super::super::drive::{DriveInput, STEP};
use super::super::players::{LEAD, Party};
use super::super::{StateHasher, WorldScene};
use super::events::{CarSpot, ChaseState, RacerState};
use super::flags::{OWN_PARKED, OWN_SHIFT_LIGHT, OWN_SHIFTING, OWN_SPEEDBREAKER, OWN_WAITING};
use super::host::WorldSimHost;
use super::ids::{EntityTable, Native};
use super::seat::MAX_PLAYERS;
use crate::input::ActionState;
use crate::viewer::Scene;

impl WorldSim for WorldSimHost {
    fn tick(&self) -> Tick {
        self.tick
    }

    /// The host's own player (`PlayerId(0)`) is the world's lead: its car is in the world from the opening, in the car
    /// `HostConfig` names, and joining only marks it seated (`join` is not read). Every other player comes in as a co-op
    /// guest in the car `join.car_type` names ([`super::catalog::car_type_id`]); the installed parts are not applied yet.
    fn add_player(&mut self, id: PlayerId, join: &PlayerJoin) -> Result<(), SimError> {
        if id.0 >= MAX_PLAYERS {
            return Err(SimError::Full);
        }
        if id == LEAD {
            if self.lead_joined {
                return Err(SimError::AlreadyJoined(id));
            }
            self.lead_joined = true;
            return Ok(());
        }
        if self.scene.players.get(id).is_some() {
            return Err(SimError::AlreadyJoined(id));
        }
        let folder = self.catalog.folder(join.car_type).ok_or(SimError::UnknownCar(join.car_type))?.to_owned();
        if let Err(error) = self.scene.seat_remote(&mut self.renderer, id, &folder, join.spawn) {
            log::warn!("net: player {} could not join in {folder}: {error:#}", u16::from(id.0) + 1);
            return Err(SimError::UnknownCar(join.car_type));
        }
        if let Some(drive) = self.scene.players.get_mut(id) {
            drive.set_remote_input(Some(DriveInput::default()));
        }
        Ok(())
    }

    /// A remote player's car leaves the world. The host's own car stays (the world's lead): it drives on with neutral
    /// input until the player joins again.
    fn remove_player(&mut self, id: PlayerId) {
        self.resets.retain(|&reset| reset != id);
        if id == LEAD {
            self.lead_joined = false;
            self.idle_lead();
            return;
        }
        self.scene.unseat_remote(&mut self.renderer, id);
    }

    fn step(&mut self, inputs: &SimInputs, events: &mut Vec<SimEvent>) {
        self.feed_inputs(inputs);
        self.release_race_countdown();
        self.wait_for_streaming();
        self.scene.update(&mut self.renderer, &ActionState::default(), STEP);
        self.tick = self.tick.wrapping_add(1);
        self.collect_events(events);
    }

    fn entities(&self, out: &mut Vec<EntityView>) {
        super::export::export(&self.scene, &self.ids, &self.catalog, out);
    }

    fn own_state(&self, id: PlayerId) -> Option<OwnState> {
        let drive = self.scene.players.get(id)?;
        let mut flags = 0;
        // A person in a race drives a racer of the traffic world while their own car is parked: the gauges are the
        // racer's, and the player is in the world, not parked.
        let racer_hud = self.race_hud_of(id);
        if racer_hud.is_none() && self.scene.players.is_parked(id) {
            flags |= OWN_PARKED;
        }
        if racer_hud.is_none() && (drive.waiting_for_road().is_some() || !drive.has_sim()) {
            flags |= OWN_WAITING;
        }
        let Some(hud) = racer_hud.or_else(|| drive.hud_state()) else {
            return Some(OwnState { flags, ..OwnState::default() });
        };
        for (on, flag) in [
            (hud.speedbreaker_active, OWN_SPEEDBREAKER),
            (hud.shifting, OWN_SHIFTING),
            (hud.shift_light, OWN_SHIFT_LIGHT),
        ] {
            if on {
                flags |= flag;
            }
        }
        Some(OwnState {
            rpm: finite(hud.rpm),
            // The wire's gear ids: reverse 0, neutral 1, first 2 (`blackbox_vehicle::drivetrain`).
            gear: u8::try_from(hud.gear.saturating_add(1)).unwrap_or(0),
            nitrous: finite(hud.nos).clamp(0.0, 1.0),
            boost: finite(hud.speedbreaker).clamp(0.0, 1.0),
            flags,
        })
    }

    /// The world's state hash (`state_hash.rs`) after the tick count.
    fn state_hash(&self) -> u64 {
        let mut hash = StateHasher::default();
        hash.u32(self.tick);
        hash.u64(self.scene.state_hash());
        hash.finish()
    }
}

impl WorldSimHost {
    /// Releases the staged race's countdown on the tick that puts GO on the tick the server scheduled.
    fn release_race_countdown(&mut self) {
        let Some(go) = self.race_go_at else { return };
        let Some(lead) = self.scene.staged_race() else {
            // No race stands on the grid: the schedule is for one that has not staged yet, or one that is gone.
            if self.scene.race.is_none() {
                self.race_go_at = None;
            }
            return;
        };
        if u64::from(self.tick) + u64::from(lead) < u64::from(go) {
            return;
        }
        self.scene.release_staged_race();
        self.race_go_at = None;
    }

    /// The gauges of the racer `id` drives in the running race, when `id` is one of the race's people.
    fn race_hud_of(&self, id: PlayerId) -> Option<crate::hud::HudState> {
        let seat = self.scene.race_people.iter().position(|&person| person == id)?;
        self.scene.traffic.as_ref()?.racer_hud(seat + 1)
    }
}

fn finite(x: f32) -> f32 {
    match x.is_finite() {
        true => x,
        false => 0.0,
    }
}

/// The entity id of one side of a contact: a player's car is its `PlayerId`.
pub(super) fn party_id(ids: &EntityTable, party: Party) -> Option<EntityId> {
    match party {
        Party::Player(id) => Some(EntityId::from(id.0)),
        Party::Car(entity) => ids.get(Native::Car(entity)),
    }
}

/// The chase after the tick (ADR 0010 section 4): with remote players the co-op chase lists its targets; without, the
/// police's one player is the chased one.
pub(super) fn chase_state(scene: &WorldScene) -> ChaseState {
    let Some(traffic) = scene.traffic.as_ref() else { return ChaseState::default() };
    let pursuing = traffic.is_pursuing();
    let primary = scene.chase_primary();
    let mut state = ChaseState {
        pursuing,
        police_busted: traffic.is_busted(),
        primary,
        evaded: traffic.end_reason() == EndReason::Evaded,
        ..ChaseState::default()
    };
    if let Some(session) = scene.coop.as_ref() {
        let status = |n: u8| session.chase.targets.status(TargetId(n));
        state.chased = session.chase.chased().collect();
        state.busted = (0..MAX_PLAYERS).filter(|&n| status(n) == Some(TargetStatus::Busted)).map(PlayerId).collect();
    }
    // The chase lists its targets from its next police tick on; the police's own player is chased from the start.
    if pursuing && state.chased.is_empty() && !state.busted.contains(&primary) {
        state.chased.push(primary);
    }
    if !pursuing {
        state.chased.clear();
    }
    state
}

/// The racers of the running race, the people in their seats: the first human racer is the lead, the second player 2
/// (the split-screen seat a co-op race gives the guest, `players/race.rs`). `None` without a race engine.
pub(super) fn racer_states(scene: &WorldScene) -> Option<Vec<RacerState>> {
    let engine = scene.race.as_ref()?.engine.as_ref()?;
    let people = &scene.race_people;
    let mut seats = match people.is_empty() {
        true => vec![LEAD, super::super::players::coop::GUEST],
        false => std::iter::once(LEAD).chain(people.iter().copied()).collect(),
    }
    .into_iter();
    let racers = engine.racers().iter().map(|racer| RacerState {
        player: racer.human.then(|| seats.next()).flatten(),
        lap_times: racer.lap_times.clone(),
        finished_at: racer.finished_at,
        trap_speeds: racer.traps.slots().to_vec(),
    });
    Some(racers.collect())
}

/// The players in play, where their cars are after the tick.
pub(super) fn car_spots(scene: &WorldScene) -> Vec<CarSpot> {
    let spot = |player: &super::super::players::Player| CarSpot {
        player: player.id,
        position: player.drive.position(),
        speed: player.drive.speed(),
        waiting: player.drive.waiting_for_road().is_some() || !player.drive.has_sim(),
    };
    scene.players.iter().map(spot).collect()
}
