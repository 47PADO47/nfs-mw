//! The scripted chases of the prologue in the world (`docs/specs/story-flow.md` section 2.4): race `16.1.0`, whose car
//! gives out after the call (it has only the call trigger), and race `16.2.1`, whose car is cut at `kill_engine`. The
//! rules are `nfsmw_career::story::chase_16_2_1`; this file feeds them the player's place and the race's trigger volumes
//! each frame, and carries out the events they return: the engine cut, the phone call, the cops and the bail, the fade
//! to black and the end of the race, which goes through the race's outro (`outro.rs`). After `16.2.1` the car lot
//! opens (`JumpToCarLot`).

use glam::Vec3;
use nfsmw_career::story::chase_16_2_1::{Chase, ChaseEvent, ChaseInput};
use nfsmw_career::story::prologue::OPENING_EVENT;
use nfsmw_data::gameplay::{BoxTrigger, GameplayData, PlaneTrigger, SphereTrigger, world_heading};

use super::Stage;
use super::chase_fade::FADE_SECS;
use crate::scenes::world::{WorldScene, space};

/// The career event id of the chase that ends in the car lot.
pub const CHASE_EVENT: &str = "16.2.1";

/// Whether `event` is one of the prologue races the chase script runs in.
pub fn is_chase_event(event: &str) -> bool {
    event == CHASE_EVENT || event == OPENING_EVENT
}

/// Seconds the phone takes to load the call's voice before the call starts (`WaitForCallLoad`, `notifycellcallstarted`).
/// The original waits on the voice stream; the rewrite uses a fixed wait, long enough that the engine gives out before
/// the call's own timers end the race.
pub const CALL_LOAD_SECS: f32 = 10.0;
/// Seconds after the sabotage at which the starved engine counts as blown (`notifyengineblown`), if the car has not
/// stalled before.
pub const ENGINE_BLOWN_AFTER_SECS: f32 = 6.0;
/// The speed in m/s under which a sabotaged car counts as stalled, once it has coasted [`STALL_MIN_SECS`].
pub const STALL_SPEED: f32 = 3.0;
/// Seconds a sabotaged car coasts before a low speed counts as the engine blowing.
pub const STALL_MIN_SECS: f32 = 1.5;

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
    /// A plane trigger: a slab across the road. The prologue's call, kill, cop and bail triggers are planes.
    Plane(PlaneTrigger),
}

/// Metres a plane trigger is thick along its heading, and how far it reaches above and below its position
/// (`docs/specs/race-wrongway.md` section 2).
const PLANE_THICKNESS: f32 = 1.0;
const PLANE_HALF_HEIGHT: f32 = 50.0;

impl Zone {
    /// Whether `point` (physics space) is inside the volume.
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            Zone::Box(zone) => zone.contains(point),
            Zone::Sphere(zone) => point.distance(zone.position) <= zone.radius,
            Zone::Plane(zone) => plane_contains(zone, point),
        }
    }

    /// Whether the player, moving from `from` to `to` this frame, touched the volume. A slab a metre thick is crossed in
    /// one frame at speed, so a plane is tested along the whole step; the other volumes by where the step ends.
    pub fn touched(&self, from: Vec3, to: Vec3) -> bool {
        match self {
            Zone::Plane(zone) => plane_crossed(zone, from, to),
            _ => self.contains(to),
        }
    }
}

/// A point in a plane trigger's frame: along its heading, across the road and up.
fn plane_local(zone: &PlaneTrigger, point: Vec3) -> Vec3 {
    let along = world_heading(zone.heading_deg);
    let across = along.cross(Vec3::Y);
    let offset = point - zone.position;
    Vec3::new(offset.dot(along), offset.dot(across), offset.y)
}

/// Whether a local point is inside the slab.
fn in_slab(zone: &PlaneTrigger, local: Vec3) -> bool {
    local.x.abs() <= PLANE_THICKNESS * 0.5 && local.y.abs() <= zone.width * 0.5 && local.z.abs() <= PLANE_HALF_HEIGHT
}

fn plane_contains(zone: &PlaneTrigger, point: Vec3) -> bool {
    in_slab(zone, plane_local(zone, point))
}

/// Whether the step from `from` to `to` passes through the slab.
fn plane_crossed(zone: &PlaneTrigger, from: Vec3, to: Vec3) -> bool {
    let (a, b) = (plane_local(zone, from), plane_local(zone, to));
    let half = PLANE_THICKNESS * 0.5;
    let span = b.x - a.x;
    if span.abs() < f32::EPSILON {
        return in_slab(zone, b);
    }
    let (t0, t1) = ((-half - a.x) / span, (half - a.x) / span);
    let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
    if lo > hi {
        return false;
    }
    in_slab(zone, a.lerp(b, (lo + hi) * 0.5))
}

/// The volume of the trigger child `child` of the race `race` (a full collection name, for example
/// `race_bin_16/16_2_1_d_day_intro`). A box trigger is looked up first, then a sphere.
pub fn zone_named(data: &GameplayData, race: &str, child: &str) -> Option<Zone> {
    let name = format!("{race}/{child}");
    if let Some(zone) = data.planes.iter().find(|zone| zone.name == name) {
        return Some(Zone::Plane(zone.clone()));
    }
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
    /// The player's place last frame, for the sweep through a plane trigger.
    last_point: Option<Vec3>,
    /// Seconds since the player entered the call trigger, once it did.
    call_secs: Option<f32>,
    /// Seconds the call's voice still has to load, once the call is requested.
    call_load_left: Option<f32>,
    /// Seconds since the engine was sabotaged, once it was.
    sabotaged_for: Option<f32>,
    /// The player's speed in m/s, as the world last reported it.
    speed: f32,
}

impl ChaseRun {
    /// The run for race `race`, with the volumes its trigger children have in `data`. The race `16.1.0` has only the
    /// call trigger; the others are not required.
    pub fn new(data: &GameplayData, race: &str) -> ChaseRun {
        let zones = TRIGGER_CHILDREN.map(|child| zone_named(data, race, child));
        let found: Vec<&str> =
            TRIGGER_CHILDREN.iter().zip(&zones).filter(|(_, zone)| zone.is_some()).map(|(name, _)| *name).collect();
        log::info!("chase: {race} has the triggers {found:?}");
        if zones[CALL].is_none() {
            log::warn!("chase: {race} lacks its call trigger, so the scripted chase will not run");
        }
        ChaseRun::from_zones(zones)
    }

    /// The run over these volumes, in the order of [`TRIGGER_CHILDREN`].
    pub fn from_zones(zones: [Option<Zone>; 4]) -> ChaseRun {
        ChaseRun {
            chase: Chase::new(),
            zones,
            inside: [false; 4],
            last_point: None,
            call_secs: None,
            call_load_left: None,
            sabotaged_for: None,
            speed: f32::INFINITY,
        }
    }

    /// Tells the run how fast the player drives: a sabotaged car that has coasted to a halt counts as blown.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Whether the chase has ended: the race went to postrace.
    pub fn is_finished(&self) -> bool {
        self.chase.is_finished()
    }

    /// One frame: `point` is the player's place (physics space) and `dt` the frame's time. Returns what the chase asks
    /// for, in order. The call starts [`CALL_LOAD_SECS`] after it is requested (the voice load).
    pub fn update(&mut self, point: Vec3, dt: f32) -> Vec<ChaseEvent> {
        let call_started = self.call_load_left.is_some_and(|left| left - dt <= 0.0);
        self.call_load_left = self.call_load_left.map(|left| left - dt).filter(|left| *left > 0.0);
        let events = self.step(point, dt, call_started);
        if events.iter().any(|event| matches!(event, ChaseEvent::PlaceCall { .. })) {
            self.call_load_left = Some(CALL_LOAD_SECS);
        }
        events
    }

    /// One frame with the call's start given by the caller (`call_started`: `notifycellcallstarted` this frame).
    pub fn step(&mut self, point: Vec3, dt: f32, call_started: bool) -> Vec<ChaseEvent> {
        let from = self.last_point.replace(point).unwrap_or(point);
        let now: [bool; 4] =
            std::array::from_fn(|i| self.zones[i].as_ref().is_some_and(|zone| zone.touched(from, point)));
        let entered = enter_edges(&mut self.inside, now);
        let call_entered = entered[CALL] && self.call_secs.is_none();
        if let Some(secs) = self.call_secs.filter(|_| !call_entered) {
            self.call_secs = Some(secs + dt);
        }
        if call_entered {
            self.call_secs = Some(0.0);
        }
        if let Some(secs) = self.sabotaged_for.as_mut() {
            *secs += dt;
        }
        let input = ChaseInput {
            secs_since_call_trigger: self.call_secs,
            at_kill_engine: entered[KILL],
            at_cop_spawn: entered[COPS],
            at_bail: entered[BAIL],
            call_started,
            engine_blown: self.engine_blown(),
            dt,
        };
        let events = self.chase.update(input);
        if events.iter().any(|event| matches!(event, ChaseEvent::SabotageEngine { .. })) {
            self.sabotaged_for.get_or_insert(0.0);
        }
        events
    }

    /// `notifyengineblown`: the sabotaged engine has run dry. The car blows after [`ENGINE_BLOWN_AFTER_SECS`], or once it
    /// has coasted down to a crawl.
    fn engine_blown(&self) -> bool {
        let Some(secs) = self.sabotaged_for else { return false };
        secs >= ENGINE_BLOWN_AFTER_SECS || (secs >= STALL_MIN_SECS && self.speed < STALL_SPEED)
    }
}

impl WorldScene {
    /// One frame of the scripted chase. It starts with race 16.1.0 or 16.2.1 running, is fed the player's place each
    /// frame and carries out what it asks. The run ends with the race and clears when the race is another one.
    pub(in crate::scenes::world) fn update_scripted_chase(&mut self, dt: f32) {
        let Some((is_chase, running, name)) = self
            .race
            .as_ref()
            .map(|session| (is_chase_event(&session.event), session.stage == Stage::Running, session.name.clone()))
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
        let speed = self.players.lead().map_or(0.0, |drive| drive.speed().abs());
        let Some(run) = self.career.chase.as_mut() else { return };
        run.set_speed(speed);
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
            ChaseEvent::FadeOn => {
                log::info!("chase: the screen fades to black");
                self.screen_fade.fade_out(FADE_SECS);
            }
            ChaseEvent::FadeOut => {
                log::info!("chase: the screen goes black");
                self.screen_fade.fade_out(FADE_SECS.min(0.5));
            }
            ChaseEvent::PostRace => self.chase_post_race(),
        }
    }

    /// The chase's race goes to postrace: the race has no result and no results screen; its outro plays (the movie of
    /// `16.2.1`) and the race's chain runs, or for `16.2.1` the car lot opens when the outro is over (`JumpToCarLot`).
    fn chase_post_race(&mut self) {
        log::info!("chase: the race is over");
        let Some(mut session) = self.race.take() else { return };
        self.stop_chase_effects();
        // The scripted end counts as the win of the event, as the NIS shell's does: the chain and the resume read it.
        if session.options.career {
            self.career_race_finished(&session.event, 1, session.cash_value);
        }
        super::outro::begin(&mut session, true);
        self.race = Some(session);
    }

    /// The chase is over: the engine runs again for whatever comes next, and the sabotage is lifted.
    fn stop_chase_effects(&mut self) {
        if let Some(drive) = self.players.lead_mut() {
            drive.set_sabotaged(false);
        }
    }
}

#[cfg(test)]
#[path = "scripted_chase_tests.rs"]
mod tests;
