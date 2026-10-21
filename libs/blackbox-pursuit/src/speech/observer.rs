//! The observer: remarks on the player's driving and on crashes while the pursuit is active (`Observer`). Collisions
//! arrive as queued observations; the rest are assessments made every observation step. Spec:
//! `docs/specs/ai-pursuit-speech.md` section 7.

use glam::Vec3;

use super::actors::Pick;
use super::cx::Cx;
use super::line::{Line, Speaker};
use super::output::Effect;
use super::params::Intensity;
use super::state::{Focus, Observation, PursuitState};

// What the observer is tracking.
const U_TURN: u32 = 1;
const BRAKING: u32 = 2;
pub(super) const OUTCOME: u32 = 4;
pub(super) const CAR_RAM: u32 = 8;
const U_TURN_RESULT: u32 = 16;
pub(super) const LOST: u32 = 32;
const ARREST: u32 = 64;

/// The busted level at which the arrest bullhorn begins.
const ARREST_LEVEL: f32 = 0.8;

#[derive(Debug, Clone)]
struct Queued {
    kind: Observation,
    voice: Speaker,
    force: f32,
}

#[derive(Debug, Clone)]
pub struct Observer {
    queue: Vec<Queued>,
    pub(super) tracking: u32,
    t_flipped: f32,
    t_unstable: f32,
    pub(super) t_outcome: f32,
    t_gas_station: f32,
    gas_station: Vec3,
    dot_track: f32,
    turn_start: Vec3,
    turn_forward: Vec3,
    turn_points: u32,
    brake_speed: f32,
    pub(super) offroad_history: u32,
    offroad_now: Option<u8>,
    pub ram_cop: Option<Speaker>,
    pub(super) voice_911: Speaker,
}

impl Default for Observer {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            tracking: 0,
            t_flipped: 0.0,
            t_unstable: 0.0,
            t_outcome: 0.0,
            t_gas_station: f32::NEG_INFINITY,
            gas_station: Vec3::ZERO,
            dot_track: 0.0,
            turn_start: Vec3::ZERO,
            turn_forward: Vec3::ZERO,
            turn_points: 0,
            brake_speed: 0.0,
            offroad_history: 0,
            offroad_now: None,
            ram_cop: None,
            voice_911: 0,
        }
    }
}

impl Observer {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Notes an observation (unless the pursuit is not on, or it is still at the opening, or over). The same kind
    /// observed again refreshes the earlier note.
    pub fn observe(&mut self, cx: &mut Cx, kind: Observation, voice: Speaker, force: f32) {
        if !cx.snap.pursuit.exists || matches!(cx.core.focus, Focus::Pursuit | Focus::Terminal) {
            return;
        }
        match self.queue.iter_mut().find(|q| q.kind == kind) {
            Some(note) => {
                note.voice = voice;
                note.force = note.force.max(force);
            }
            None => self.queue.push(Queued { kind, voice, force }),
        }
        cx.core.last_observation = kind;
    }

    /// The observation step: say what was observed, then assess the player's driving.
    pub fn update(&mut self, cx: &mut Cx) {
        if !cx.state_is(PursuitState::Active) {
            return;
        }
        if !cx.snap.pursuit.exists
            || (cx.focus_is(Focus::Pursuit) && cx.snap.pursuit.busted < 0.5)
            || cx.focus_is(Focus::Terminal)
        {
            return;
        }
        let queued = std::mem::take(&mut self.queue);
        for note in queued {
            if let Some(speaker) = cx.roster.by_voice(note.voice).map(|a| a.id) {
                self.collision(cx, note.kind, note.force, speaker);
            }
        }
        self.assess_arrest(cx);
        if self.tracking & ARREST != 0 {
            return;
        }
        self.assess_los(cx);
        self.assess_flippage(cx);
        self.assess_u_turn(cx);
        self.assess_outcome(cx);
        self.assess_braking(cx);
        self.assess_outrun(cx);
        self.assess_offroad(cx);
        self.gas_station_aftermath(cx);
    }

    /// What a cop says about an observed collision.
    fn collision(&mut self, cx: &mut Cx, kind: Observation, force: f32, speaker: u32) {
        let intensity = match force > 0.75 {
            true => Intensity::High,
            false => Intensity::Normal,
        };
        match kind {
            Observation::SuspectHitWorld => cx.impact_suspect_world(speaker),
            Observation::SuspectHitTraffic => cx.impact_suspect_traffic(speaker, intensity),
            Observation::SuspectHitSemi => cx.impact_suspect_semi(speaker),
            Observation::SuspectHitGuardrail => cx.impact_suspect_guardrail(speaker),
            Observation::SuspectHitTrain => cx.impact_suspect_train(speaker),
            Observation::SuspectHitGasStation => cx.impact_suspect_gas_station(speaker),
            Observation::SuspectHitSpikeBelt => cx.impact_suspect_spike_belt(speaker),
            Observation::Airborne => cx.suspect_airborne(speaker, intensity),
            Observation::Rollover => cx.suspect_rollover(speaker, intensity),
            Observation::Spunout => cx.suspect_spunout(speaker, intensity),
            _ => {}
        }
    }

    /// The busted bar nearly full: the best cop (the helicopter when it is close) speaks over the bullhorn.
    fn assess_arrest(&mut self, cx: &mut Cx) {
        let busted = cx.snap.pursuit.busted;
        let queued_or_playing = [Line::HeliBullhornArrest, Line::BullhornArrest]
            .iter()
            .any(|&l| cx.chan.queued(l) || cx.chan.is_playing(l));
        if busted > ARREST_LEVEL && !queued_or_playing {
            self.tracking |= ARREST;
            if let Some(cop) = cx.random_active(Pick::Any, true) {
                cx.clear_queue();
                let heli = cx.roster.actors.iter().find(|a| a.heli && a.active && a.distance < 100.0).map(|a| a.id);
                match heli {
                    Some(heli) => cx.heli_bullhorn_arrest(heli),
                    None => cx.bullhorn_arrest(cop),
                }
            }
        } else {
            self.tracking &= !ARREST;
        }
        cx.effect(Effect::BustedLevel(busted));
    }

    /// After a "lost visual", the first cop to see the player again says so.
    fn assess_los(&mut self, cx: &mut Cx) {
        if matches!(cx.core.focus, Focus::Pursuit | Focus::Lost) {
            self.tracking &= !LOST;
        }
        if cx.time_in_view() <= 0.0 || self.tracking & LOST == 0 {
            return;
        }
        if let Some(&first) = cx.roster.with_los().first() {
            cx.regain_visual(first);
            self.tracking &= !LOST;
        }
    }

    fn assess_flippage(&mut self, cx: &mut Cx) {
        if cx.core.los_count == 0 {
            return;
        }
        let flipped = cx.snap.player.up_y < 0.0;
        if !flipped {
            self.t_flipped = cx.now;
        }
        if !flipped || cx.now - self.t_flipped < cx.tune.flip_time {
            return;
        }
        if let Some(cop) = cx.roster.closest(true, true) {
            let intensity = fly_flip_intensity(cx);
            cx.suspect_rollover(cop, intensity);
        }
    }

    /// A U-turn: the car's heading against the road's falls below 0.7, passes -0.7 within a number of frames and a
    /// distance, and holds there.
    fn assess_u_turn(&mut self, cx: &mut Cx) {
        let player = cx.snap.player;
        let Some(road_forward) = player.road_forward else { return };
        if cx.core.los_count == 0 {
            self.tracking &= !(U_TURN | U_TURN_RESULT);
            self.t_unstable = cx.now;
            return;
        }
        let dot = player.forward.dot(road_forward);
        if dot >= 0.7 && self.tracking & U_TURN_RESULT == 0 {
            self.t_unstable = cx.now;
        }
        if dot < 0.7 && self.tracking & (U_TURN | U_TURN_RESULT) == 0 {
            self.dot_track = dot;
            self.turn_forward = road_forward;
            self.tracking |= U_TURN;
            self.turn_points = 0;
            self.turn_start = player.position;
        }
        if self.tracking & U_TURN == 0 {
            return;
        }
        if dot > -0.7 && self.tracking & U_TURN_RESULT == 0 {
            match dot > self.dot_track {
                true => self.abandon_turn(cx.now),
                false => {
                    self.dot_track = dot;
                    self.turn_points += 1;
                }
            }
        }
        if dot <= -0.7 && self.turn_points >= cx.tune.contig_frames_180 && self.tracking & U_TURN_RESULT == 0 {
            match self.turn_start.distance(player.position) > cx.tune.range_180 {
                true => self.abandon_turn(cx.now),
                false => {
                    self.tracking |= U_TURN_RESULT;
                    self.t_unstable = cx.now;
                }
            }
        }
        if self.tracking & U_TURN_RESULT == 0 {
            return;
        }
        if player.forward.dot(self.turn_forward) > -0.7 {
            self.abandon_turn(cx.now);
            self.tracking &= !U_TURN_RESULT;
            return;
        }
        if cx.now - self.t_unstable <= cx.tune.time_180 {
            return;
        }
        if let Some(cop) = cx.roster.closest(true, true) {
            cx.suspect_uturn(cop);
            self.abandon_turn(cx.now);
            self.tracking &= !U_TURN_RESULT;
        }
    }

    fn abandon_turn(&mut self, now: f32) {
        self.tracking &= !U_TURN;
        self.t_unstable = now;
        self.turn_points = 0;
        self.turn_start = Vec3::ZERO;
    }

    /// A strategy was announced: some seconds later the cop says how it went.
    fn assess_outcome(&mut self, cx: &mut Cx) {
        if !cx.focus_is(Focus::Strategy) || !cx.state_is(PursuitState::Active) {
            return;
        }
        let busted = cx.snap.pursuit.busted;
        if (busted > 0.0 && busted < 1.0) || cx.snap.pursuit.collapse_active {
            let cop = cx.random_active(Pick::Primary, false);
            if let Some(cop) = cop.filter(|&c| !cx.is_heli(c) && !cx.chan.queued(Line::AnticipateSuccess)) {
                cx.anticipate_success(cop);
            }
        }
        if self.tracking & OUTCOME == 0 || busted >= 1.0 {
            return;
        }
        let tracked = cx.now - self.t_outcome;
        if tracked < cx.tune.outcome_track_time {
            return;
        }
        let Some(cop) =
            cx.roster.actors.iter().find(|a| a.active && a.los && cx.layout.is_primary(a.voice)).map(|a| a.id)
        else {
            return;
        };
        let mut say_outcome = true;
        if self.tracking & CAR_RAM != 0 {
            if let Some(ram) = self.ram_cop.and_then(|v| cx.roster.by_voice(v)).copied() {
                say_outcome = false;
                match ram.since_rammed(cx.now) <= tracked {
                    true => cx.anticipate_success(ram.id),
                    false => cx.anticipate_fail(ram.id),
                }
            }
            self.tracking &= !CAR_RAM;
            cx.core.flags.cops_immune = false;
        }
        if say_outcome {
            match cx.snap.player.speed_mph >= cx.tune.outcome_fail_speed {
                true => cx.outcome_fail(cop, None),
                false if !cx.is_heli(cop) => cx.anticipate_success(cop),
                false => {}
            }
        }
        self.tracking &= !OUTCOME;
    }

    /// Hard braking on the road at speed, with a big loss of speed: "suspect brake".
    fn assess_braking(&mut self, cx: &mut Cx) {
        let player = cx.snap.player;
        let Some(road_forward) = player.road_forward else { return };
        if cx.core.los_count == 0 {
            return;
        }
        let aligned = player.forward.dot(road_forward);
        let last_crash = (cx.now - cx.core.t_last_crashed).min(cx.time_last_nailed_cop());
        if self.tracking & BRAKING == 0 {
            let starts = aligned >= 0.7
                && (player.brake >= 1.0 || player.handbrake >= 1.0)
                && player.speed_mph > 60.0
                && last_crash > 5.0;
            if !starts {
                return;
            }
            self.tracking |= BRAKING;
            self.brake_speed = player.speed_mph;
        }
        if aligned < 0.7 || (player.brake < 0.5 && player.handbrake < 0.5) || last_crash < 5.0 {
            self.tracking &= !BRAKING;
            return;
        }
        if player.speed_mph / self.brake_speed.max(f32::EPSILON) >= 1.0 - cx.tune.crash_slowdown {
            return;
        }
        if let Some(cop) = cx.roster.closest(true, true) {
            cx.suspect_brake(cop);
            self.tracking &= !BRAKING;
        }
    }

    /// Every cop with sight has been falling behind for eight seconds and none is ahead: "suspect outrun".
    fn assess_outrun(&mut self, cx: &mut Cx) {
        if cx.core.los_count == 0 || cx.time_in_view() < 2.0 {
            return;
        }
        let seeing: Vec<f32> =
            cx.roster.actors.iter().filter(|a| a.active && a.los).map(|a| cx.now - a.last_closing).collect();
        if seeing.is_empty() || !seeing.iter().all(|&t| t > 8.0) || cx.core.flags.cops_ahead {
            return;
        }
        if let Some(cop) = cx.random_active(Pick::Any, false) {
            cx.suspect_outrun(cop);
        }
    }

    /// The player drives through a place the cops know by name: the first cop to see it says so.
    fn assess_offroad(&mut self, cx: &mut Cx) {
        if cx.core.los_count < 1 || cx.time_in_view() < 2.0 {
            return;
        }
        let Some(place) = cx.snap.player.offroad_id else { return };
        if self.offroad_now == Some(place) {
            return;
        }
        let Some(cop) = cx.roster.closest(true, true) else { return };
        self.offroad_now = Some(place);
        if place >= 24 {
            return;
        }
        let seen_before = self.offroad_history & (1 << place) != 0;
        cx.offroad(cop, place, seen_before);
    }

    /// For fifteen seconds after a gas station blew up, cops within fifty metres call for an emergency vehicle.
    fn gas_station_aftermath(&mut self, cx: &mut Cx) {
        if cx.now - self.t_gas_station >= 15.0 {
            return;
        }
        let near: Vec<u32> =
            cx.roster.actors.iter().filter(|a| a.position.distance(self.gas_station) < 50.0).map(|a| a.id).collect();
        for cop in near {
            cx.call_for_ev(cop, Some(super::params::Ev::GasStationFire));
        }
    }

    /// A gas station exploded: a cop with sight remarks on it, and the aftermath starts.
    pub fn gas_station_exploded(&mut self, cx: &mut Cx, by_player: bool, at: Vec3) {
        if !cx.focus_is(Focus::Strategy) {
            return;
        }
        if by_player {
            let closest = cx.roster.closest(false, true).filter(|&c| cx.roster.find(c).is_some_and(|a| a.los));
            if let Some(cop) = closest {
                cx.impact_suspect_gas_station(cop);
            }
        }
        self.t_gas_station = cx.now;
        self.gas_station = at;
    }

    /// A tunnel entered or left: the helicopter warns of an approaching tunnel.
    pub fn tunnel(&mut self, cx: &mut Cx, inside: bool) {
        cx.core.tunnel = inside;
        let heli = cx.roster.actors.iter().find(|a| a.heli && a.active && a.los).map(|a| a.id);
        if let (true, Some(heli)) = (inside, heli) {
            cx.heli_hazard_alert(heli);
        }
    }

    /// The player was airborne long enough for a cop to remark on it.
    pub fn airborne(&mut self, cx: &mut Cx, altitude: f32, secs: f32) {
        if cx.core.los_count == 0 || altitude < cx.tune.min_height_airborne || secs < cx.tune.hang_time {
            return;
        }
        if let Some(cop) = cx.roster.closest(true, true) {
            let intensity = fly_flip_intensity(cx);
            cx.suspect_airborne(cop, intensity);
        }
    }

    /// The player passed a cop at a much higher speed: the cop says "spotted".
    pub fn blew_past(&mut self, cx: &mut Cx, cop: u32) {
        let searching = matches!(cx.core.state, PursuitState::Active | PursuitState::Searching);
        if !cx.focus_is(Focus::Strategy) || !searching || cx.core.los_count > 1 {
            return;
        }
        let Some(actor) = cx.roster.find(cop).copied() else { return };
        if cx.snap.player.speed_mph > actor.speed + 50.0 && !actor.roadblock {
            cx.spotted(cop);
        }
    }
}

/// The intensity of a flip or a jump: high at speed.
fn fly_flip_intensity(cx: &Cx) -> Intensity {
    match cx.snap.player.speed_mph >= cx.tune.fly_flip_speed {
        true => Intensity::High,
        false => Intensity::Normal,
    }
}
