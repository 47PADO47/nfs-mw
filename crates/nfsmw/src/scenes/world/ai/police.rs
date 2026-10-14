//! Police in the chase: the heat of a pursuit, the offences the player commits, the police helicopter that joins it and
//! the high-speed screen effects. The rules are in `blackbox-pursuit` and `blackbox-speedfx`; this file reads the
//! world and feeds them. Specs: `docs/specs/ai-pursuit-heat.md`, `docs/specs/ai-helicopter.md`.
//!
//! Spaces: the helicopter flies in physics space (y up, z forward) and the minimap draws in render space (z up); the
//! player is measured in render space and converted with [`crate::scenes::world::space`].

use std::collections::BTreeSet;
use std::rc::Rc;

use super::support;
use blackbox_pursuit::formation::finisher::{Event, Finisher};
use blackbox_pursuit::formation::slots::assign;
use blackbox_pursuit::formation::{Formation, Picker};
use blackbox_pursuit::helicopter::control::{Flat, Heights};
use blackbox_pursuit::helicopter::{Helicopter, Mode, SearchInput, THINK_PERIOD, Target, Tuning};
use blackbox_pursuit::rules::{self, Contact};
use blackbox_pursuit::support::Strategy;
use blackbox_pursuit::{Busted, Cooldown, EndReason, Fines, Heat, Infraction, InfractionLog};
use blackbox_render::Instance;
use blackbox_roads::{HeliSheet, RandomSource, SegmentFilter, TrackZone, closest_segment, to_zone_space};
use blackbox_speedfx::{Effects, SpeedFx, Tuning as SpeedTuning};
use glam::{Vec2, Vec3};

use super::traffic_events::SPEEDING_MPS;
use crate::scenes::world::drive::{CarPose, CarRig};
use crate::scenes::world::space;

/// Heat levels with a helicopter in their cop mix (`pursuitlevels`, [verified]: heat 4 to 6 and 8 to 10).
const HELI_LEVELS: [u32; 6] = [4, 5, 6, 8, 9, 10];
/// Seconds of fuel for heat levels 4 to 10 (`HeliFuelTime`, [verified]).
const FUEL_TIME: [f32; 7] = [60.0, 90.0, 180.0, 10.0, 75.0, 120.0, 400.0];
/// Seconds before the next helicopter may join, per heat level 1 to 10 (`TimeBetweenHeliActive`, [verified]).
const RESPAWN_TIME: [f32; 10] = [0.0, 0.0, 120.0, 180.0, 180.0, 180.0, 180.0, 120.0, 60.0, 5.0];
/// Heat levels whose `pursuitsupport` has a skid-hit air entry (`SKID_HIT`, [verified]).
const SKID_LEVELS: [u32; 5] = [5, 6, 7, 9, 10];
/// Seconds of heat per level, 1 to 10 (`ai-pursuit-heat.md` §3.2, with the bin-independent values).
const TIME_PER_LEVEL: [f32; 10] = [600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 780.0, 820.0, 2000.0];
/// How far behind the player the helicopter spawns on the road (`SpawnPursuitHelicopter`).
const SPAWN_BEHIND: f32 = 250.0;
/// How high above the player it spawns.
const SPAWN_HEIGHT: f32 = 20.0;
/// The helicopter takes its first join this long after a chase starts (`mSpawnHeliTimer`).
const FIRST_JOIN: f32 = 10.0;
/// Seconds between support checks (`support_check_timer`, spec section 6).
const SUPPORT_CHECK: f32 = 10.0;
/// Turns the search circles per search point (0.125 of a turn, the spec's step).
const SEARCH_STEP_TURNS: f32 = 0.125;

/// The car's heat after an escape: `EvadeSuccessHeatAdjust` of the default collection (install-verified, `ai-pursuit-heat.md` §3.3).
const EVADE_HEAT_ADJUST: f32 = 0.95;
/// The track path zone kind of a hiding spot (`pursuit-data.md` §5: type 8 HIDDEN).
pub const HIDDEN_ZONE: u32 = 8;
/// How far (metres) the player's height may be from a hiding zone's elevation to count as inside it (spec §5).
const HIDDEN_HEIGHT: f32 = 1.25;

/// The throttle above which a hit counts as deliberate (unconfirmed).
const DELIBERATE_THROTTLE: f32 = 0.5;

/// How far from a road centreline (metres, horizontally) the player still counts as on the road. Unconfirmed: about a
/// road's half width and a margin.
const ROAD_EDGE: f32 = 9.0;

/// The fines of the infractions, in [`Infraction::ALL`] order (`infractions`, [verified]).
const FINES: Fines = Fines::new([150, 350, 1000, 350, 300, 100, 300, 75]);

/// The player's car as the police see it this frame, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerView {
    pub position: Vec3,
    /// The direction the car faces (unit length).
    pub forward: Vec3,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// Nitrous left, 0 to 1 (0 without the part).
    pub nitrous: f32,
    /// The throttle held this frame, 0 to 1. A hit with the throttle down counts as deliberate (a proxy: the install's
    /// rule is not read yet).
    pub throttle: f32,
}

/// What the world reports this frame, besides the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sensed {
    /// A cop can see the player.
    pub seen: bool,
    /// A uniform random number in `0..=1` for the helicopter's search.
    pub unit_random: f32,
    /// The player is on a road the cops know (within `ROAD_EDGE` of a road centreline).
    pub on_road: bool,
    /// Distance from the player to the nearest cop, metres.
    pub nearest_cop: f32,
    /// The player hit traffic within the traffic's hit memory.
    pub hit_traffic: bool,
    /// The player is in a hiding spot (a HIDDEN track zone at its height): the evade progress runs faster.
    pub hidden: bool,
    /// Two uniform random numbers for the formation picker (the weighted record and the timer's jitter).
    pub formation_random: [f32; 2],
    /// The cops in position at their formation slot (count) and their average distance to it, metres; `None` without a
    /// formation or a cop in position.
    pub formation_gap: Option<(u32, f32)>,
    /// A uniform random number for the ground support roll.
    pub support_random: f32,
}

/// The police state of the world: heat, the infraction log, the helicopter and the speed effects.
pub struct Police {
    heat: Option<Heat>,
    infractions: InfractionLog,
    heli: Option<Helicopter>,
    /// Seconds until the next helicopter may join.
    heli_wait: f32,
    /// Seconds of think time not yet spent on the helicopter.
    heli_clock: f32,
    seen: bool,
    speed_fx: SpeedFx,
    /// Where the helicopter's search circles, in turns.
    search_turns: f32,
    /// The helicopter's model, when it is loaded (`COPHELI`).
    rig: Option<Rc<CarRig>>,
    /// The escape: the grace, the cool-down meter and EVADED (`blackbox_pursuit::Cooldown`).
    cooldown: Cooldown,
    /// How the last chase ended, kept after it until the next chase (`blackbox_pursuit::EndReason`).
    end_reason: EndReason,
    /// Seconds since the last chase ended; the end card fades by it.
    since_end: f32,
    /// The car's heat carried from the last escape (`VehicleHeat`, the career stable's value for the one car).
    carried_heat: f32,
    /// The formation timer and the last formation chosen (`blackbox_pursuit::formation`).
    formations: Picker,
    last_formation: Option<Formation>,
    last_level: u32,
    /// The finisher of the chosen formation (`blackbox_pursuit::formation::finisher`).
    finisher: Finisher,
    /// Seconds of pursuit so far; the support checks wait for `support::minimum_delay`.
    pursuit_time: f32,
    /// Seconds until the next support check (starts at `SUPPORT_CHECK`).
    support_timer: f32,
    /// A roadblock was asked for this check and is not yet placed.
    roadblock_requested: bool,
    /// A roadblock of this chase stands (a heavy roadblock is then not rolled again).
    roadblock_standing: bool,
    /// The HIDDEN track zones of the track (`TRACK_PATH_ZONES`, kind 8).
    hidden_zones: Vec<TrackZone>,
    /// The helicopter's height sheet of the track, when it was read (`nfsmw_data::world::heli_sheet`).
    heli_sheet: Option<HeliSheet>,
    /// The busted timer of the chase (`blackbox_pursuit::Busted`); reset when a chase starts, kept after it ends.
    busted: Busted,
    /// The cop cars wrecked in this chase (`IncNumCopsDestroyed`); reset when a chase starts, kept after it ends.
    cops_destroyed: u32,
}

impl Default for Police {
    fn default() -> Self {
        Self {
            heat: None,
            infractions: InfractionLog::default(),
            heli: None,
            heli_wait: 0.0,
            heli_clock: 0.0,
            seen: false,
            speed_fx: SpeedFx::new(SpeedTuning::default()),
            search_turns: 0.0,
            rig: None,
            cooldown: Cooldown::default(),
            end_reason: EndReason::None,
            since_end: 0.0,
            carried_heat: 0.0,
            formations: Picker::new(),
            last_formation: None,
            last_level: 0,
            finisher: Finisher::default(),
            pursuit_time: 0.0,
            support_timer: SUPPORT_CHECK,
            roadblock_requested: false,
            roadblock_standing: false,
            hidden_zones: Vec::new(),
            heli_sheet: None,
            busted: Busted::default(),
            cops_destroyed: 0,
        }
    }
}

impl Police {
    /// A chase starts at `level`: the heat is at least the base and at least `level`, the infraction log empties and
    /// the helicopter's first join is set.
    pub fn begin(&mut self, level: u32) {
        let mut heat = Heat::new(1.0, 10.0);
        heat.begin();
        heat.raise_to(self.carried_heat);
        heat.raise_to(level as f32);
        self.heat = Some(heat);
        self.infractions.clear();
        self.heli_wait = FIRST_JOIN;
        self.seen = true;
        self.cooldown = Cooldown::default();
        self.end_reason = EndReason::None;
        self.since_end = 0.0;
        self.pursuit_time = 0.0;
        self.support_timer = SUPPORT_CHECK;
        self.roadblock_requested = false;
        self.roadblock_standing = false;
        self.busted.reset();
        self.cops_destroyed = 0;
    }

    /// A cop car was wrecked. Only a chase counts it. The rep points of the wreck (`RepPointsForDestroying[heat]`,
    /// `docs/specs/ai-pursuit-heat.md`) are not paid yet: the economy is not built, so this is the hook they go in.
    pub fn record_wreck(&mut self) {
        if self.heat.is_none() {
            return;
        }
        self.cops_destroyed += 1;
    }

    /// The cop cars wrecked in the chase.
    pub fn cops_destroyed(&self) -> u32 {
        self.cops_destroyed
    }

    /// The chase is over with `reason`: the helicopter, if any, leaves. The reason and the counts stay for the summary.
    pub fn end(&mut self, reason: EndReason) {
        self.heat = None;
        self.end_reason = reason;
        self.since_end = 0.0;
        if let Some(heli) = self.heli.as_mut() {
            heli.leave();
        }
    }

    /// The heat level the chase is at, if one is on.
    pub fn level(&self) -> Option<u32> {
        self.heat.map(|h| h.level())
    }

    pub fn infractions(&self) -> &InfractionLog {
        &self.infractions
    }

    /// The high-speed effects on screen now.
    pub fn effects(&self) -> Effects {
        self.speed_fx.effects()
    }

    /// The helicopter's model, to draw it with (`None` until the model is loaded).
    /// The track's HIDDEN zones (kind `HIDDEN_ZONE`), read once per track.
    /// The track's helicopter height sheet; without one the helicopter keeps a flat floor.
    pub fn set_heli_sheet(&mut self, sheet: HeliSheet) {
        self.heli_sheet = Some(sheet);
    }

    pub fn set_hidden_zones(&mut self, zones: Vec<TrackZone>) {
        self.hidden_zones = zones;
    }

    pub fn set_rig(&mut self, rig: Option<Rc<CarRig>>) {
        self.rig = rig;
    }

    /// The helicopter's draw instances while it flies. Its frame is converted from physics to render space.
    pub fn instances(&self, out: &mut Vec<Instance>) {
        let (Some(rig), Some(heli)) = (self.rig.as_ref(), self.heli.as_ref()) else { return };
        let pose = CarPose {
            position: space::to_render(heli.body.position.to_array()),
            rotation: render_rotation(heli.body.orientation),
            wheels: Default::default(),
        };
        rig.instances(&pose, None, &BTreeSet::new(), out);
    }

    /// The helicopter's place on the minimap in render space, and its heading, while it flies.
    pub fn helicopter_blip(&self) -> Option<([f32; 2], [f32; 2])> {
        let heli = self.heli.as_ref()?;
        let at = space::to_render(heli.body.position.to_array());
        let forward = space::to_render(heli.body.forward().to_array());
        Some(([at.x, at.y], [forward.x, forward.y]))
    }

    /// Advances the police by `dt` seconds. `pursuing` is a chase on; `sensed` is what the world saw this frame.
    /// Returns `true` when the player has got away this frame: the chase should end. Without a world (the tests).
    #[cfg(test)]
    pub fn update(&mut self, dt: f32, pursuing: bool, player: &PlayerView, sensed: Sensed) -> bool {
        self.update_in_world(dt, pursuing, player, sensed, &|_, _| true)
    }

    /// As [`Police::update`], with the world: `clear(from, to)` is true when the line between two physics points is not
    /// blocked by the world (the helicopter's sight).
    pub fn update_in_world(
        &mut self,
        dt: f32,
        pursuing: bool,
        player: &PlayerView,
        sensed: Sensed,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) -> bool {
        self.speed_fx.update(player.speed, player.nitrous, dt);
        self.heli_wait = (self.heli_wait - dt).max(0.0);
        let mut evaded = false;
        if pursuing && self.heat.is_some() {
            self.record_infractions(player, sensed);
            if let Some(heat) = self.heat.as_mut() {
                heat.grow(dt, time_per_level(heat.level()));
            }
            let level = self.level().unwrap_or(1);
            self.tick_formations(dt, level, sensed.formation_random);
            self.tick_finisher(dt, sensed.formation_gap);
            self.tick_support(dt, sensed.seen, level, sensed.support_random);
            evaded = self.tick_cooldown(dt, sensed, level);
            if let (true, Some(heat)) = (evaded, self.heat) {
                self.carried_heat = heat.value() * EVADE_HEAT_ADJUST;
            }
        }
        if !pursuing {
            self.age_end_card(dt);
        }
        self.think_helicopter(dt, pursuing, player, sensed, clear);
        evaded
    }

    /// The support check (spec section 6): every `SUPPORT_CHECK` seconds, after the level's minimum delay, while the
    /// cops see the player, the heavy options of the level are rolled. A roadblock roll is recorded for the world to place;
    /// the other strategies are logged (their cars are not built).
    fn tick_support(&mut self, dt: f32, seen: bool, level: u32, random: f32) {
        self.pursuit_time += dt;
        self.support_timer -= dt;
        if self.support_timer > 0.0 || self.pursuit_time < support::minimum_delay(level) {
            return;
        }
        self.support_timer = SUPPORT_CHECK;
        if !seen {
            return;
        }
        let roll =
            blackbox_pursuit::support::roll(&support::heavy_options(level), &[], random, self.roadblock_standing);
        match roll.map(|r| r.strategy) {
            Some(Strategy::HeavyRoadblock) => self.roadblock_requested = true,
            Some(other) => log::info!("support: {other:?} rolled (not built)"),
            None => {}
        }
    }

    /// Whether a roadblock was asked for since the last call (the world places it).
    pub fn take_roadblock_request(&mut self) -> bool {
        std::mem::take(&mut self.roadblock_requested)
    }

    /// The world placed the roadblock: the chase does not roll another heavy roadblock.
    pub fn roadblock_placed(&mut self) {
        self.roadblock_standing = true;
    }

    /// Advances the chosen formation's finisher with the cops' gap to their slots. Armed and ended are logged.
    fn tick_finisher(&mut self, dt: f32, gap: Option<(u32, f32)>) {
        let Some(params) = self.last_formation.and_then(blackbox_pursuit::formation::finisher::params_for) else {
            return;
        };
        let (count, average) = gap.map_or((0, None), |(count, average)| (count, Some(average)));
        match self.finisher.tick(dt, count, average, &params) {
            Event::Armed => log::info!("the finisher runs: the cops hold their slots"),
            Event::Ended => log::info!("the finisher is over: the cops return to the chase"),
            Event::None => {}
        }
    }

    /// Whether a finisher is running; the cops then drive straight at the player.
    pub fn finisher_running(&self) -> bool {
        self.finisher.is_running()
    }

    /// Advances the formation timer: a heat change chooses at once; otherwise the timer runs and picks when it is up.
    fn tick_formations(&mut self, dt: f32, level: u32, random: [f32; 2]) {
        if level != self.last_level {
            self.last_level = level;
            self.formations.heat_changed();
        }
        if let Some(choice) = self.formations.tick(dt, level, true, false, random) {
            log::debug!("cop formation: {:?} for {:.0} s", choice.formation, choice.duration);
            self.last_formation = Some(choice.formation);
            self.finisher = Finisher::default();
        }
    }

    /// The last formation the chase chose, for the status line.
    pub fn last_formation(&self) -> Option<Formation> {
        self.last_formation
    }

    /// The offences of this frame: speeding, and losing the cops (resisting arrest).
    fn record_infractions(&mut self, player: &PlayerView, sensed: Sensed) {
        if rules::speeding(player.speed, SPEEDING_MPS) {
            self.infractions.record(Infraction::Speeding, &FINES);
        }
        if rules::off_road(sensed.on_road, sensed.seen, sensed.nearest_cop) {
            self.infractions.record(Infraction::OffRoad, &FINES);
        }
        if sensed.hit_traffic {
            let contact = Contact {
                target: rules::Target::Traffic,
                deliberate: player.throttle > DELIBERATE_THROTTLE,
                closing_speed: player.speed.abs(),
                in_pursuit: true,
                cop_sees: sensed.seen,
                nearest_cop: sensed.nearest_cop,
                first_damage: false,
            };
            if let Some(infraction) = rules::contact_infraction(&contact) {
                self.infractions.record(infraction, &FINES);
            }
        }
        if rules::lost_sight(self.seen, sensed.seen) {
            self.infractions.record(Infraction::ResistingArrest, &FINES);
        }
        self.seen = sensed.seen;
    }

    fn think_helicopter(
        &mut self,
        dt: f32,
        pursuing: bool,
        player: &PlayerView,
        sensed: Sensed,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) {
        if self.heli.is_none() && pursuing {
            self.try_join(player);
        }
        let Some(mut heli) = self.heli.take() else { return };
        let from = heli.body.position;
        let line = |to: Vec3| clear(from, to + Vec3::Y);
        let body = to_body(player);
        let flat = Flat(body.position.y - 2.0);
        let sheet = SheetFloor(self.heli_sheet.as_ref());
        let heights: &dyn Heights = if self.heli_sheet.is_some() { &sheet } else { &flat };
        self.heli_clock += dt;
        let mut gone = false;
        while self.heli_clock >= THINK_PERIOD && !gone {
            self.heli_clock -= THINK_PERIOD;
            self.search_turns = (self.search_turns + SEARCH_STEP_TURNS).fract();
            let search = SearchInput { angle_turns: self.search_turns, unit_random: sensed.unit_random };
            let target = Target { hidden: false, in_tunnel: false };
            gone = heli.think(THINK_PERIOD, &body, target, &line, heights, search);
        }
        if gone {
            self.heli_wait = respawn_time(self.level().unwrap_or(1));
            return;
        }
        if heli.too_far(&body) && !matches!(heli.mode, Mode::Exit(_)) {
            heli.leave();
        }
        self.heli = Some(heli);
    }

    /// Sends a helicopter in when the level has one, its wait is over and the chase is on.
    fn try_join(&mut self, player: &PlayerView) {
        let Some(level) = self.level() else { return };
        if self.heli_wait > 0.0 || !HELI_LEVELS.contains(&level) {
            return;
        }
        let body = to_body(player);
        let behind = body.position - horizontal(body.forward).normalize_or_zero() * SPAWN_BEHIND
            + Vec3::new(0.0, SPAWN_HEIGHT, 0.0);
        let heading = body.forward.z.atan2(body.forward.x);
        let tuning = Tuning {
            spawn_distance: SPAWN_BEHIND,
            fuel_time: fuel_time(level),
            skid_hit: SKID_LEVELS.contains(&level),
            ..Tuning::default()
        };
        self.heli = Some(Helicopter::spawn(behind, heading, tuning));
    }
}

#[cfg(test)]
#[path = "police_tests.rs"]
mod tests;

mod busted;
mod escape;
mod helpers;
mod world;

use helpers::{SheetFloor, fuel_time, horizontal, render_rotation, respawn_time, time_per_level, to_body};
