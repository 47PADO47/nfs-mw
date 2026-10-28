//! The chase radio: the game's side of the speech machine in `blackbox-pursuit`. It turns the world's chase feed into the
//! machine's snapshot and notices, runs the machine each frame against the speech queue, and sends what it asks for to
//! the speech as events with parameters. Spec: `docs/specs/ai-pursuit-speech.md`.
//!
//! What the world cannot supply yet is left unset in the snapshot (see `docs/pursuit-system.md`, "Police radio"): the
//! speech map's off-road places, the player's brake and road alignment, AI racers, free-roam smashes, the backup's
//! arrival time and the roadblock's outcomes.

mod channel;
#[cfg(test)]
mod installed_tests;
mod params;
mod snapshot;
pub mod tables;
#[cfg(test)]
mod tests;
mod world_notices;

use std::collections::HashMap;

use blackbox_pursuit::EndReason;
use blackbox_pursuit::speech::{Effect, Line, Machine, Notice, Output, Snapshot, Talker, Tune};
use nfsmw_data::speech_chase::{ChaseTune, LevelSpeech};

pub use channel::{DispatcherChannel, LineIds, Sent, Silent};
pub use params::speech_params;

use super::AirSupport;
use super::roam::{self, RoamFeed};
use crate::audio::PursuitFeed;
use crate::audio::speech::Actor;
use crate::audio::speech::{Dispatcher, Request, Speech};

/// Seconds a 911 report that was asked for may take to be said before the machine is left idle again (it also waits
/// while its counters hold something).
const AWAIT_911: f32 = 30.0;

/// The machine's tune from the database's `speechtune` values.
pub fn tune_of(t: &ChaseTune) -> Tune {
    Tune {
        pursuit_init_delay: t.pursuit_init_delay,
        range_for_spotter: t.range_for_spotter_branch,
        speed_diff_for_blowby: t.speed_diff_for_blowby,
        blowby_interval: t.blowby_interval,
        suspect_outrun_range: t.suspect_outrun_range,
        time_considered_lost_no_los: t.time_considered_lost_no_los,
        inactivity: t.pursuit_inactivity_timer,
        no_los_commentary: t.no_los_commentary_time,
        min_speed_stopped: t.min_speed_considered_stopped,
        min_time_stopped: t.min_time_considered_stopped,
        high_intensity_heat: t.high_intensity_mark,
        duration_high_intensity: t.pursuit_duration_high_intensity,
        min_bailout_duration: t.min_pursuit_duration_for_bailouts,
        min_health_commentary: t.min_health_for_commentary,
        crash_slowdown: t.crash_slowdown_pct,
        smash_range: t.player_smash_speed_range,
        min_intensity_traffic: t.min_intensity_traffic_smash,
        min_intensity_cop_smash: t.min_intensity_cop_smash,
        min_havoc: t.min_havoc_for_suspect_behavior,
        min_height_airborne: t.min_height_airborne,
        hang_time: t.hang_time_for_commentary,
        flip_time: t.flip_time_for_commentary,
        fly_flip_speed: t.speed_thresh_fly_flip_intensity,
        contig_frames_180: t.min_contig_frames_for_180,
        range_180: t.max_range_for_180,
        time_180: t.max_time_for_180,
        outcome_track_time: t.outcome_track_time,
        outcome_fail_speed: t.outcome_fail_speed,
        rb_outcome_timer: t.rb_outcome_timer,
        rb_post_outcome_reset: t.rb_post_outcome_reset_time,
        backup_remind_time: t.bu_remind_time,
        racer_proximity: t.ai_racer_proximity,
    }
}

/// The machine, what it asked for and what it remembers of the last frame.
pub struct ChaseRadio {
    machine: Machine,
    tune: Tune,
    ids: Option<LineIds>,
    sent: Sent,
    last: Option<(Line, blackbox_pursuit::speech::Speaker)>,
    prev: snapshot::Prev,
    running: bool,
    last_snapshot: Option<Snapshot>,
    last_ended: EndReason,
    /// The 911 call that ended: the seconds the cops must keep away, until the world takes it.
    call_911: Option<f32>,
    busted_level: f32,
    asked: u32,
    said: u32,
    /// The free-roam frame held something that keeps the machine running.
    roam_armed: bool,
    /// The free-roam frame the world gave this frame, until the machine takes it.
    roam: Option<RoamFeed>,
    /// The session heat of the last free-roam frame, which picks the level row of the 911 figures.
    roam_heat: u32,
    /// Seconds a 911 report asked for has to be said before the machine stops being stepped for it.
    awaiting_911: f32,
    /// The notices the world's events have made (`world_notices`).
    world_notices: u32,
}

impl ChaseRadio {
    /// A different world must not inherit queued notices, voices, 911 state or the previous chase's terminal flow.
    /// Keep the loaded event mapping and database tuning, which belong to the audio library.
    pub fn reset_session(&mut self) {
        let ids = self.ids.take();
        let tune = self.tune;
        *self = Self::new(crate::audio::speech::random::Rng::from_clock().next_seed());
        self.ids = ids;
        self.tune = tune;
        self.machine.set_tune(tune);
    }

    /// The actual speech flow that drives police sirens and the formation setup gate.
    pub fn police_phase(&self) -> blackbox_pursuit::siren::Phase {
        use blackbox_pursuit::{siren::Phase, speech::Focus};
        match self.machine.focus() {
            Focus::Pursuit => Phase::Opening,
            Focus::Lost => Phase::Lost,
            Focus::Strategy | Focus::Terminal => Phase::Running,
        }
    }

    pub fn new(seed: u64) -> Self {
        Self {
            machine: Machine::new(tables::layout(), tune_of(&ChaseTune::default()), seed),
            tune: tune_of(&ChaseTune::default()),
            ids: None,
            sent: HashMap::new(),
            last: None,
            prev: snapshot::Prev::default(),
            running: false,
            last_snapshot: None,
            last_ended: EndReason::None,
            call_911: None,
            busted_level: 0.0,
            asked: 0,
            said: 0,
            roam_armed: false,
            roam: None,
            roam_heat: 1,
            awaiting_911: 0.0,
            world_notices: 0,
        }
    }

    /// Replaces the machine's thresholds with the database's.
    pub fn set_tune(&mut self, tune: &ChaseTune) {
        self.tune = tune_of(tune);
        self.machine.set_tune(self.tune);
    }

    /// The free-roam frame of this update: what the player did since the last one (the 911 logic's inputs). It is used
    /// when no chase runs; `None` clears it.
    pub fn set_roam(&mut self, roam: Option<RoamFeed>) {
        if let Some(feed) = &roam {
            self.roam_heat = feed.heat.max(1);
        }
        self.roam = roam;
    }

    /// The heat of the free-roam frame, which picks the row of the 911 figures.
    pub fn roam_heat(&self) -> u32 {
        self.roam_heat
    }

    /// The pending 911 call, if one ended since the last call.
    pub fn take_911_call(&mut self) -> Option<f32> {
        self.call_911.take()
    }

    /// A line for the console's `speech flow`.
    pub fn status(&self) -> String {
        let missing = self
            .ids
            .as_ref()
            .map_or("not loaded".to_owned(), |ids| format!("{} lines without an event", ids.missing().len()));
        format!(
            "chase radio: {}\n  asked {} lines, {} said; busted level {:.2}; 911 call {}; {} world notices; events: {missing}",
            self.machine.status(),
            self.asked,
            self.said,
            self.busted_level,
            self.call_911.map_or("none".to_owned(), |s| format!("{s:.0} s")),
            self.world_notices,
        )
    }

    /// The machine's snapshot of this frame: the chase's, or the lingering one just after a chase ended.
    fn frame(&mut self, chase: Option<&PursuitFeed>, ended: EndReason, level: &LevelSpeech) -> (Snapshot, Vec<Notice>) {
        let Some(feed) = chase else {
            self.prev.forget();
            let mut notices = Vec::new();
            if ended == EndReason::Busted && self.last_ended != EndReason::Busted {
                notices.push(Notice::Busted);
            }
            self.last_ended = ended;
            let roam = self.roam.take();
            if let Some(feed) = &roam {
                notices.extend(roam::notices(feed));
            }
            let snap = match (self.running, self.last_snapshot.take()) {
                // The frame the chase ended: the pursuit is evaded (or gone), once.
                (true, Some(mut last)) => {
                    last.pursuit.cooling_down = false;
                    last.pursuit.evaded = ended == EndReason::Evaded;
                    last
                }
                _ => roam.as_ref().map_or_else(Snapshot::default, |feed| roam::snapshot(feed, level)),
            };
            self.running = false;
            self.roam_armed = roam.as_ref().is_some_and(|feed| feed.armed);
            return (snap, notices);
        };
        let mut notices = Vec::new();
        if !self.running && self.machine.focus() == blackbox_pursuit::speech::Focus::Terminal {
            notices.push(Notice::Restart);
        }
        self.running = true;
        self.last_ended = EndReason::None;
        notices.extend(snapshot::notices(&mut self.prev, feed));
        let world = world_notices::notices(&mut self.prev.events_seen, &feed.speech.events, &self.tune, &feed.speech);
        self.world_notices += world.len() as u32;
        notices.extend(world);
        let snap = snapshot::snapshot(feed, level);
        self.last_snapshot = Some(snap.clone());
        (snap, notices)
    }

    /// Runs the machine for one frame. `dispatcher` is the speech queue when it is loaded.
    pub fn run(
        &mut self,
        dt: f32,
        chase: Option<&PursuitFeed>,
        ended: EndReason,
        level: &LevelSpeech,
        mut dispatcher: Option<&mut Dispatcher>,
    ) -> Output {
        let (snap, mut notices) = self.frame(chase, ended, level);
        self.awaiting_911 = (self.awaiting_911 - dt).max(0.0);
        let waiting = self.roam_armed || self.awaiting_911 > 0.0;
        if chase.is_none() && !self.running && self.machine.is_idle() && notices.is_empty() && !waiting {
            return Output::default();
        }
        if let (Some(d), Some(ids)) = (dispatcher.as_deref_mut(), self.ids.as_ref()) {
            for (event, _) in d.take_finished() {
                let Some((line, speaker, params)) = self.sent.get(&event).copied() else { continue };
                if ids.line(event) != Some(line) && ids.line(event).is_none() {
                    continue;
                }
                self.last = Some((line, speaker));
                self.said += 1;
                if line == Line::Disp911Report {
                    self.awaiting_911 = 0.0;
                }
                notices.push(Notice::Said { line, speaker, params });
            }
        }
        match (dispatcher.as_deref(), self.ids.as_ref()) {
            (Some(d), Some(ids)) => {
                let channel = DispatcherChannel { dispatcher: d, ids, sent: &self.sent, last: self.last };
                self.machine.step(dt, &snap, &notices, &channel)
            }
            _ => self.machine.step(dt, &snap, &notices, &Silent),
        }
    }

    /// Sends the machine's output to the speech: effects first, then the lines the air-support switches allow.
    pub fn apply(&mut self, out: Output, speech: &mut Speech, air: &AirSupport) {
        if self.ids.is_none() {
            self.ids = Some(LineIds::new(speech.dispatcher()));
        }
        for effect in out.effects {
            match effect {
                Effect::ClearQueue => speech.clear_playback(),
                Effect::ResetHistory => speech.dispatcher_mut().clear_history(),
                Effect::Recall(line) => {
                    if let Some(id) = self.ids.as_ref().and_then(|ids| ids.id(line)) {
                        speech.dispatcher_mut().recall(id);
                    }
                }
                Effect::Force911 { secs } => self.call_911 = Some(secs),
                Effect::BustedLevel(level) => self.busted_level = level,
            }
        }
        let Some(ids) = self.ids.as_ref() else { return };
        for request in out.requests {
            let Some(id) = ids.id(request.line) else {
                log::debug!("chase radio: no speech event for {:?}", request.line);
                continue;
            };
            if !air.allows_line(request.line) {
                continue;
            }
            if request.line == Line::Disp911Report {
                // The machine asks again every update until the report has been said; one request is enough.
                if self.awaiting_911 > 0.0 {
                    continue;
                }
                self.awaiting_911 = AWAIT_911;
            }
            self.asked += 1;
            self.sent.insert(id, (request.line, request.speaker, request.params));
            let actor = request.talker.map(actor_of);
            let asked = Request {
                speaker: u16::from(request.speaker),
                actor,
                params: speech_params(request.line, &request.params),
            };
            let outcome = speech.request_as(id, asked);
            log::debug!("chase radio: {:?} by {} -> {outcome:?}", request.line, request.speaker);
        }
    }
}

fn actor_of(talker: Talker) -> Actor {
    Actor { distance: talker.distance, on_screen: talker.in_view, line_of_sight: talker.los }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;

#[cfg(test)]
mod pause_tests;
