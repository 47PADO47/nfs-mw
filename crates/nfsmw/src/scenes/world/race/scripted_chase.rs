//! The scripted chase of race `16.2.1` in the world (`docs/specs/story-flow.md` section 2.4). The rules are
//! `nfsmw_career::story::chase_16_2_1`; this file feeds them the player's place and the race's trigger volumes each
//! frame, and carries out the events they return: the engine cut, the phone call, the cops and the bail, and the end of
//! the race, after which the car lot opens (`JumpToCarLot`).

use glam::Vec3;
use nfsmw_career::story::chase_16_2_1::{Chase, ChaseEvent, ChaseInput};
use nfsmw_data::gameplay::{BoxTrigger, GameplayData, SphereTrigger};

use super::Stage;
use crate::career::CareerRequest;
use crate::scenes::world::{WorldScene, space};

/// The career event id of the scripted chase.
pub const CHASE_EVENT: &str = "16.2.1";

/// The trigger children of the race, in the order [`ChaseRun`] keeps their volumes: the call, the engine kill, the cops
/// and the bail (`docs/specs/story-flow.md` section 2.4).
const TRIGGER_CHILDREN: [&str; 4] = ["call_from_mia", "kill_engine", "cop_spawn_trigger", "bail_trigger"];
const CALL: usize = 0;
const KILL: usize = 1;
const COPS: usize = 2;
const BAIL: usize = 3;

/// A trigger volume of the race: a box or a sphere, as the gameplay data stores it.
#[derive(Debug, Clone, PartialEq)]
pub enum Zone {
    /// A box trigger.
    Box(BoxTrigger),
    /// A sphere trigger.
    Sphere(SphereTrigger),
}

impl Zone {
    /// Whether `point` (physics space) is inside the volume.
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            Zone::Box(zone) => zone.contains(point),
            Zone::Sphere(zone) => point.distance(zone.position) <= zone.radius,
        }
    }
}

/// The volume of the trigger child `child` of the race `race` (a full collection name, for example
/// `race_bin_16/16_2_1_d_day_intro`). A box trigger is looked up first, then a sphere.
pub fn zone_named(data: &GameplayData, race: &str, child: &str) -> Option<Zone> {
    let name = format!("{race}/{child}");
    if let Some(zone) = data.triggers.iter().find(|zone| zone.name == name) {
        return Some(Zone::Box(zone.clone()));
    }
    data.spheres.iter().find(|zone| zone.name == name).map(|zone| Zone::Sphere(zone.clone()))
}

/// The entries of this frame: a trigger fires when the player goes from outside its volume to inside it. Updates
/// `inside` to `now`.
pub fn enter_edges(inside: &mut [bool; 4], now: [bool; 4]) -> [bool; 4] {
    let entered = std::array::from_fn(|i| now[i] && !inside[i]);
    *inside = now;
    entered
}

/// One run of the chase: the rules, the four trigger volumes of the race and the clock of the call trigger.
#[derive(Debug)]
pub struct ChaseRun {
    chase: Chase,
    /// The volumes of [`TRIGGER_CHILDREN`]; `None` where the race has no such child.
    zones: [Option<Zone>; 4],
    /// Whether the player was inside each volume last frame.
    inside: [bool; 4],
    /// Seconds since the player entered the call trigger, once it did.
    call_secs: Option<f32>,
    /// The call was requested in the last frame, so it is taken as started in this one.
    call_requested: bool,
}

impl ChaseRun {
    /// The run for race `race`, with the volumes its trigger children have in `data`.
    pub fn new(data: &GameplayData, race: &str) -> ChaseRun {
        let zones = TRIGGER_CHILDREN.map(|child| zone_named(data, race, child));
        if zones.iter().any(Option::is_none) {
            log::warn!("chase: {race} lacks a trigger child, so the scripted chase may not run");
        }
        ChaseRun::from_zones(zones)
    }

    /// The run over these volumes, in the order of [`TRIGGER_CHILDREN`].
    pub fn from_zones(zones: [Option<Zone>; 4]) -> ChaseRun {
        ChaseRun { chase: Chase::new(), zones, inside: [false; 4], call_secs: None, call_requested: false }
    }

    /// Whether the chase has ended: the race went to postrace.
    pub fn is_finished(&self) -> bool {
        self.chase.is_finished()
    }

    /// One frame: `point` is the player's place (physics space) and `dt` the frame's time. Returns what the chase asks
    /// for, in order. The phone takes a requested call at once, so the call is taken as started on the next frame (the
    /// voice load is not modelled).
    pub fn update(&mut self, point: Vec3, dt: f32) -> Vec<ChaseEvent> {
        let call_started = std::mem::take(&mut self.call_requested);
        self.step(point, dt, call_started)
    }

    /// One frame with the call's start given by the caller (`call_started`: `notifycellcallstarted` this frame).
    pub fn step(&mut self, point: Vec3, dt: f32, call_started: bool) -> Vec<ChaseEvent> {
        let now: [bool; 4] = std::array::from_fn(|i| self.zones[i].as_ref().is_some_and(|zone| zone.contains(point)));
        let entered = enter_edges(&mut self.inside, now);
        let call_entered = entered[CALL] && self.call_secs.is_none();
        if let Some(secs) = self.call_secs.filter(|_| !call_entered) {
            self.call_secs = Some(secs + dt);
        }
        if call_entered {
            self.call_secs = Some(0.0);
        }
        let input = ChaseInput {
            secs_since_call_trigger: self.call_secs,
            at_kill_engine: entered[KILL],
            at_cop_spawn: entered[COPS],
            at_bail: entered[BAIL],
            call_started,
            // The rewrite has no engine-blown event yet, so the sabotage ends the chase only by the call path.
            engine_blown: false,
            dt,
        };
        let events = self.chase.update(input);
        self.call_requested = events.iter().any(|event| matches!(event, ChaseEvent::PlaceCall { .. }));
        events
    }
}

impl WorldScene {
    /// One frame of the scripted chase. It starts with race 16.2.1 running, is fed the player's place each frame and
    /// carries out what it asks. The run ends with the race and clears when the race is another one.
    pub(in crate::scenes::world) fn update_scripted_chase(&mut self, dt: f32) {
        let Some((is_chase, running, name)) = self
            .race
            .as_ref()
            .map(|session| (session.event == CHASE_EVENT, session.stage == Stage::Running, session.name.clone()))
        else {
            self.career.chase = None;
            return;
        };
        if !is_chase {
            self.career.chase = None;
            return;
        }
        if !running {
            return;
        }
        if self.career.chase.is_none() {
            self.career.chase = self.gameplay.as_ref().map(|data| ChaseRun::new(data, &name));
        }
        let Some(point) = self.chase_point() else { return };
        let Some(run) = self.career.chase.as_mut() else { return };
        let events = run.update(point, dt);
        let finished = run.is_finished();
        for event in events {
            self.apply_chase_event(event);
        }
        if finished {
            self.career.chase = None;
        }
    }

    /// The player's place in physics space, once the car stands on the road.
    fn chase_point(&self) -> Option<Vec3> {
        let drive = self.players.lead().filter(|drive| drive.has_sim() && drive.waiting_for_road().is_none())?;
        Some(Vec3::from(space::to_physics(drive.position())))
    }

    /// Carries out one event of the chase.
    fn apply_chase_event(&mut self, event: ChaseEvent) {
        match event {
            ChaseEvent::SabotageEngine { level } => {
                log::info!("chase: engine sabotaged at level {level}");
                if let Some(drive) = self.players.lead_mut() {
                    drive.set_sabotaged(true);
                }
            }
            ChaseEvent::PlaceCall { id } => {
                log::info!("chase: phone call {id} requested");
                self.career.phone.request_call(id);
            }
            ChaseEvent::CopsOn => {
                if let Some(traffic) = self.traffic.as_mut() {
                    traffic.bar_cops(false);
                }
            }
            ChaseEvent::SpawnCops { count, .. } => {
                log::warn!(
                    "chase: {count} mid-size cop(s) asked for at the cop trigger; placing cars at a spot is not built yet"
                );
            }
            ChaseEvent::DispIntroRace => log::info!("chase: the race intro display is not drawn yet"),
            ChaseEvent::BailPursuit { a, b } => {
                log::info!("chase: bail pursuit ({a}, {b})");
                if let Some(traffic) = self.traffic.as_mut() {
                    traffic.stop_pursuit();
                }
            }
            ChaseEvent::NoNewPursuits => {
                if let Some(traffic) = self.traffic.as_mut() {
                    traffic.bar_cops(true);
                }
            }
            ChaseEvent::FadeOn => log::info!("chase: the screen fades on (no fade in the world yet)"),
            ChaseEvent::FadeOut => log::info!("chase: the screen fades out"),
            ChaseEvent::PostRace => self.chase_post_race(),
        }
    }

    /// The chase's race goes to postrace: the race ends without a result, then `JumpToCarLot` opens the car lot.
    fn chase_post_race(&mut self) {
        log::info!("chase: {CHASE_EVENT} over, the car lot opens");
        if let Some(session) = self.race.take() {
            self.end_race(session);
        }
        self.career.requests.push_back(CareerRequest::FirstCarLot);
    }
}

#[cfg(test)]
#[path = "scripted_chase_tests.rs"]
mod tests;
