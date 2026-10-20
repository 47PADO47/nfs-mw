//! Police in the chase: the heat of a pursuit, the offences the player commits, the police helicopter that joins it and
//! the high-speed screen effects. The rules are in `blackbox-pursuit` and `blackbox-speedfx`; this file reads the
//! world and feeds them. Specs: `docs/specs/ai-pursuit-heat.md`, `docs/specs/ai-helicopter.md`.
//!
//! Spaces: the helicopter flies in physics space (y up, z forward) and the minimap draws in render space (z up); the
//! player is measured in render space and converted with [`crate::scenes::world::space`].

use std::collections::BTreeSet;
use std::rc::Rc;

use blackbox_pursuit::formation::finisher::{Event, Finisher};
use blackbox_pursuit::formation::slots::assign;
use blackbox_pursuit::formation::{Formation, Picker};
use blackbox_pursuit::helicopter::Helicopter;
use blackbox_pursuit::rules::{self, Contact};
use blackbox_pursuit::{Busted, Cooldown, EndReason, Heat, Infraction, InfractionLog};
use blackbox_render::Instance;
use blackbox_roads::{HeliSheet, RandomSource, SegmentFilter, TrackZone, closest_segment, to_zone_space};
use blackbox_speedfx::{Effects, SpeedFx, Tuning as SpeedTuning};
use glam::{Vec2, Vec3};

use super::traffic_events::SPEEDING_MPS;
use crate::scenes::world::drive::{CarPose, CarRig};
use crate::scenes::world::space;
use crate::settings::bartender::BartenderSettings;
use crate::settings::extra_options;
use crate::settings::fairbusts::{FairBustsSettings, ResetPenalty};
use crate::settings::helicopter::HeliSettings;

/// The helicopter takes its first join this long after a chase starts (`mSpawnHeliTimer`).
const FIRST_JOIN: f32 = 10.0;
/// Seconds between support checks (`support_check_timer`, spec section 6).
const SUPPORT_CHECK: f32 = 10.0;
/// A HIDDEN zone's first data word, as a name hash (`docs/formats/pursuit-data.md`, HIDDEN). `"Car"` hides the player
/// from cars only, `"Heli"` from helicopters only, and any other word from both.
const CAR_HASH: u32 = blackbox_hash::bstring_hash("Car");
/// See [`CAR_HASH`].
const HELI_HASH: u32 = blackbox_hash::bstring_hash("Heli");
/// The track zone kind of a tunnel (`TUNNEL`, `pursuit-data.md`: type 3).
pub const TUNNEL_ZONE: u32 = 3;

/// The track path zone kind of a hiding spot (`pursuit-data.md` §5: type 8 HIDDEN).
pub const HIDDEN_ZONE: u32 = 8;
/// How far (metres) the player's height may be from a hiding zone's elevation to count as inside it (spec §5).
const HIDDEN_HEIGHT: f32 = 1.25;

/// The throttle above which a hit counts as deliberate (unconfirmed).
const DELIBERATE_THROTTLE: f32 = 0.5;

/// How far from a road centreline (metres, horizontally) the player still counts as on the road. Unconfirmed: about a
/// road's half width and a margin.
const ROAD_EDGE: f32 = 9.0;

/// The player's car as the police see it this frame, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerView {
    pub position: Vec3,
    /// The direction the car faces (unit length).
    pub forward: Vec3,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// Nitrous burning, 0 to 1 (0 while it does not burn and without the part).
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
    /// The player's hit this frame is the first that damages a cop (an assault).
    pub struck_cop: bool,
    /// The player's car touched a breakable prop this frame (damage to property).
    pub struck_property: bool,
    /// The player is in a hiding spot (a HIDDEN track zone at its height): the evade progress runs faster.
    pub hidden: bool,
    /// The player is in a HIDDEN zone that hides it from helicopters (a zone flagged `Car` does not).
    pub hidden_from_heli: bool,
    /// The player is inside a tunnel zone: the helicopter does not see into one (the spec's camera test, read as the
    /// player's place).
    pub in_tunnel: bool,
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
    /// Standalone rules assume available assets; the world supplies actual model readiness through `set_rig`.
    heli_available: bool,
    /// The helicopter's tuning per level, from `NFSMWHelicopterSettings.ini` (vanilla until it is loaded).
    heli_settings: HeliSettings,
    /// The pursuit tunables from `NFSMWBartenderSettings.ini`: cop cap, heat growth, bust rules and fines (vanilla until
    /// it is loaded).
    settings: BartenderSettings,
    /// The reset penalty of a manual reset, from `NFSMWFairBustsSettings.ini` (FairBusts' default until it is loaded).
    reset_penalty: ResetPenalty,
    /// Seconds of the post-reset blink left (`busted::RESET_BLINK` after a manual reset); 0 when not flashing.
    reset_blink: f32,
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
    /// A ram or leader request was rolled and waits for the chase to take it (`take_support_request`).
    support_requested: Option<blackbox_pursuit::support::Request>,
    /// The heat the priority check of the leader options was last run for, and whether it has run (once per heat).
    priority_heat: u32,
    priority_done: bool,
    /// The HIDDEN track zones of the track (`TRACK_PATH_ZONES`, kind 8).
    hidden_zones: Vec<TrackZone>,
    /// The TUNNEL track zones of the track (kind `TUNNEL_ZONE`), read once per track.
    tunnel_zones: Vec<TrackZone>,
    /// The helicopter's height sheet of the track, when it was read (`nfsmw_data::world::heli_sheet`).
    heli_sheet: Option<HeliSheet>,
    /// The busted timer of the chase (`blackbox_pursuit::Busted`); reset when a chase starts, kept after it ends.
    busted: Busted,
    /// The chase's rep points, cost to state and wrecks (`rep.rs`); reset when a chase starts, kept after it ends.
    rep: rep::RepState,
    /// The bust sequence after the meter fills (`police/busted/flow.rs`); reset when a chase starts.
    bust: busted::BustFlow,
}

impl Default for Police {
    fn default() -> Self {
        Self {
            heat: None,
            infractions: InfractionLog::default(),
            bust: busted::BustFlow::default(),
            heli: None,
            heli_available: true,
            heli_settings: HeliSettings::default(),
            settings: BartenderSettings::default(),
            reset_penalty: FairBustsSettings::default().penalty(BartenderSettings::default().busted.reach),
            reset_blink: 0.0,
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
            support_requested: None,
            priority_heat: 0,
            priority_done: false,
            hidden_zones: Vec::new(),
            tunnel_zones: Vec::new(),
            heli_sheet: None,
            busted: Busted::default(),
            rep: rep::RepState::default(),
        }
    }
}

impl Police {
    /// Replaces the pursuit tunables and the FairBusts penalty (read from their files when the track opens). The reset
    /// penalty takes the Bartender file's normal reach, which a fair reset uses.
    pub fn set_settings(&mut self, settings: BartenderSettings, fairbusts: FairBustsSettings) {
        self.reset_penalty = fairbusts.penalty(settings.busted.reach);
        self.settings = settings;
    }

    /// The pursuit tunables in force.
    pub fn settings(&self) -> &BartenderSettings {
        &self.settings
    }

    /// The reset penalty in force (FairBusts, or the stock penalty when it is disabled).
    pub fn reset_penalty(&self) -> ResetPenalty {
        self.reset_penalty
    }

    /// The heat the car carries into the next chase (the career's record of the car, set by the career at each start).
    pub fn set_carried_heat(&mut self, heat: f32) {
        self.carried_heat = heat;
    }

    /// A chase starts at `level`: the heat is at least the base and at least `level`, the infraction log empties and
    /// the helicopter's first join is set.
    pub fn begin(&mut self, level: u32) {
        let bounds = extra_options::active().heat_bounds().unwrap_or_default();
        let mut heat = Heat::new(bounds.min, bounds.max);
        heat.begin();
        heat.raise_to(self.carried_heat);
        heat.raise_to(level as f32);
        self.heat = Some(heat);
        self.infractions.clear();
        self.bust.reset();
        self.heli_wait = FIRST_JOIN;
        self.seen = true;
        self.cooldown = Cooldown::default();
        self.end_reason = EndReason::None;
        self.since_end = 0.0;
        self.pursuit_time = 0.0;
        self.support_timer = SUPPORT_CHECK;
        self.roadblock_requested = false;
        self.roadblock_standing = false;
        self.support_requested = None;
        self.priority_heat = 0;
        self.priority_done = false;
        self.busted.reset();
        self.reset_blink = 0.0;
        self.rep.reset();
        self.last_formation = None;
        self.last_level = 0;
        self.formations = Picker::new();
        self.finisher = Finisher::default();
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

    /// The track's TUNNEL zones, read once per track.
    pub fn set_tunnel_zones(&mut self, zones: Vec<TrackZone>) {
        self.tunnel_zones = zones;
    }

    pub fn set_rig(&mut self, rig: Option<Rc<CarRig>>) {
        self.heli_available = rig.is_some();
        if !self.heli_available {
            self.heli = None;
        }
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

    /// The helicopter's place on the minimap and the way its nose points, in render space, while it flies.
    pub fn helicopter_blip(&self) -> Option<([f32; 2], [f32; 2])> {
        let heli = self.heli.as_ref()?;
        let at = space::to_render(heli.body.position.to_array());
        let nose = space::to_render(heli.body.forward().to_array());
        Some(([at.x, at.y], [nose.x, nose.y]))
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
        mut sensed: Sensed,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) -> bool {
        self.speed_fx.update(player.speed, player.nitrous, dt);
        self.reset_blink = (self.reset_blink - dt).max(0.0);
        self.heli_wait = (self.heli_wait - dt).max(0.0);
        self.think_helicopter(dt, pursuing, player, sensed, clear);
        sensed.seen |= self.helicopter_sees();
        let mut evaded = false;
        if pursuing && self.heat.is_some() {
            self.record_infractions(player, sensed);
            if self.cooldown.status() == blackbox_pursuit::CooldownStatus::InitialChase
                && let Some(heat) = self.heat.as_mut()
            {
                heat.grow(dt, self.settings.time_per_level(heat.level()));
            }
            let level = self.level().unwrap_or(1);
            self.tick_formations(dt, level, sensed.formation_random);
            self.tick_finisher(dt, sensed.formation_gap);
            self.tick_support(dt, sensed.seen, level, sensed.support_random);
            evaded = self.tick_cooldown(dt, sensed, level);
            if let (true, Some(heat)) = (evaded, self.heat) {
                self.carried_heat = heat.value() * self.settings.evade_heat_adjust;
            }
        }
        if !pursuing {
            self.age_end_card(dt);
        }
        evaded
    }

    /// Whether a roadblock was asked for since the last call (the world places it).
    pub fn take_roadblock_request(&mut self) -> bool {
        std::mem::take(&mut self.roadblock_requested)
    }

    /// The world placed the roadblock: the chase does not roll another heavy roadblock.
    pub fn roadblock_placed(&mut self) {
        self.roadblock_standing = true;
        self.on_roadblock();
    }

    /// A roadblock of this chase stands.
    pub fn roadblock_standing(&self) -> bool {
        self.roadblock_standing
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
        let limits = extra_options::active();
        if rules::speeding(player.speed, limits.speeding_limit().unwrap_or(SPEEDING_MPS)) {
            self.infractions.record(Infraction::Speeding, &self.settings.fines);
        }
        if limits.reckless_limit().is_some_and(|limit| rules::speeding(player.speed, limit)) {
            self.infractions.record(Infraction::RecklessDriving, &self.settings.fines);
        }
        if rules::off_road(sensed.on_road, sensed.seen, sensed.nearest_cop) {
            self.infractions.record(Infraction::OffRoad, &self.settings.fines);
        }
        if sensed.hit_traffic {
            self.record_contact(rules::Target::Traffic, player, sensed, false);
        }
        if sensed.struck_cop {
            self.record_contact(rules::Target::Cop, player, sensed, true);
        }
        if sensed.struck_property {
            self.record_contact(rules::Target::Breakable, player, sensed, false);
        }
        if rules::lost_sight(self.seen, sensed.seen) {
            self.infractions.record(Infraction::ResistingArrest, &self.settings.fines);
        }
        self.seen = sensed.seen;
    }

    /// Records the infraction of a contact with `target`, if the rules make one (`rules::contact_infraction`).
    fn record_contact(&mut self, target: rules::Target, player: &PlayerView, sensed: Sensed, first_damage: bool) {
        let contact = Contact {
            target,
            deliberate: player.throttle > DELIBERATE_THROTTLE,
            closing_speed: player.speed.abs(),
            in_pursuit: true,
            cop_sees: sensed.seen,
            nearest_cop: sensed.nearest_cop,
            first_damage,
        };
        let Some(infraction) = rules::contact_infraction(&contact) else { return };
        self.infractions.record(infraction, &self.settings.fines);
    }
}

#[cfg(test)]
#[path = "police_tests.rs"]
mod tests;

mod busted;
mod escape;
mod heli;
mod helpers;
mod mod_feed;
mod original_camera;
mod patrol;
mod rep;
mod roll;
mod siren_feed;
mod world;

use helpers::{horizontal, render_rotation};
