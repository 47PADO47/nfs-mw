//! The speech queue: requests come in with an event, wait their turn and are said one at a time.
//! Spec: `docs/specs/speech.md` §2 and §4.
//!
//! The dispatcher knows nothing about sound. [`Voice`] is the one line that can sound at a time; the dispatcher
//! asks it to start a line and asks whether it is still going.

use std::collections::HashMap;

use nfsmw_data::speech::{SpeechEvent, SpeechTune};

use super::history::History;
use super::params::SpeechParams;
use super::random::Rng;
use super::rules::{Actor, Asked, Context, Env, Verdict, judge};

/// Added to the priority of an interrupting event, so it goes before every other.
pub const INTERRUPT_BONUS: i32 = 100;
/// A line counts as heard (for the history) once it has been going this long, or when it ends.
const HEARD_AFTER: f32 = 3.0;
/// The dead air before the first line: long enough for any event's requirement.
const NEVER_SPOKEN: f32 = 65535.0;

/// The one voice the speech comes out of.
pub trait Voice {
    /// Whether a line is sounding.
    fn busy(&self) -> bool;

    /// Start the line for `event` as `request` asks (the speaker, 0 for any, and the parameters that choose the
    /// wording), cutting the line in progress when `cut`. False when there is nothing to say (no recording that
    /// fits): the request is then dropped.
    fn play(&mut self, event: &SpeechEvent, request: &Request, cut: bool) -> bool;
}

/// What a request asks for besides the event.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Request {
    /// The voice that should say it (1 to 9), or 0 for any.
    pub speaker: u16,
    pub actor: Option<Actor>,
    /// The situation the line describes (car colour, direction, ...): which variant of the sentence is said. Left
    /// empty, the wording is chosen at random among the variants the event has.
    pub params: SpeechParams,
}

/// What became of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Queued,
    /// The event was waiting already: its wait starts over.
    Refreshed,
    /// No event has this number.
    Unknown,
    /// Speech is switched off (the speech volume is zero).
    Muted,
    /// Dropped at random, as requests are as a pursuit drags on.
    Thinned,
}

struct Pending {
    event: u32,
    /// Priority with the interrupt bonus.
    priority: i32,
    request: Request,
    entry: f32,
    /// Waiting out the event's initial delay.
    delayed: bool,
    /// Order of arrival.
    seq: u64,
    /// Passed every check at the last look: it may be said as soon as it is its turn.
    ready: bool,
}

/// The line being said, or that was said last.
struct Current {
    event: u32,
    request: Request,
    priority: i32,
    interrupt: bool,
    interruptable: bool,
    enforce_dead_air: f32,
    started: f32,
    /// When the line ended; the silence the event enforces counts from here.
    finished: Option<f32>,
    /// Whether the history knows about it.
    counted: bool,
}

pub struct Dispatcher {
    events: HashMap<u32, SpeechEvent>,
    /// The number of every collection name (a name whose number another collection took still finds that event).
    names: HashMap<String, u32>,
    tune: SpeechTune,
    pending: Vec<Pending>,
    history: History,
    current: Option<Current>,
    now: f32,
    last_sounded: Option<f32>,
    pursuit_secs: f32,
    enabled: bool,
    rng: Rng,
    seq: u64,
    /// Events whose line ended since the last [`Dispatcher::take_finished`], with the request that was said.
    finished: Vec<(u32, Request)>,
}

impl Dispatcher {
    /// A dispatcher for `events` (the template collection with number 0 is not an event).
    pub fn new(events: Vec<SpeechEvent>, tune: SpeechTune, rng: Rng) -> Self {
        // Two collections can share a number (`collisionworld` and `collworld_spin`, both 98): the file has the one record,
        // the sentence of the first, so the first keeps the number.
        let mut by_id = HashMap::new();
        let mut names = HashMap::new();
        for event in events.into_iter().filter(|e| e.id != 0) {
            names.insert(event.name.to_ascii_lowercase(), event.id);
            by_id.entry(event.id).or_insert(event);
        }
        let events = by_id;
        Self {
            events,
            names,
            tune,
            pending: Vec::new(),
            history: History::default(),
            current: None,
            now: 0.0,
            last_sounded: None,
            pursuit_secs: 0.0,
            enabled: true,
            rng,
            seq: 0,
            finished: Vec::new(),
        }
    }

    pub fn event(&self, id: u32) -> Option<&SpeechEvent> {
        self.events.get(&id)
    }

    /// The event with this name (case does not matter), if there is one.
    pub fn find(&self, name: &str) -> Option<&SpeechEvent> {
        let id = self.names.get(&name.to_ascii_lowercase())?;
        self.events.get(id)
    }

    /// The events in order of number.
    pub fn events(&self) -> Vec<&SpeechEvent> {
        let mut all: Vec<&SpeechEvent> = self.events.values().collect();
        all.sort_by_key(|e| e.id);
        all
    }

    /// With speech off (volume zero) requests are refused, as the original does.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// How long the pursuit has lasted, in seconds: later requests are thinned out.
    pub fn set_pursuit_secs(&mut self, secs: f32) {
        self.pursuit_secs = secs;
    }

    /// Requests waiting.
    pub fn waiting(&self) -> usize {
        self.pending.len()
    }

    /// The event being said, if a line is sounding or its enforced silence is running.
    pub fn current(&self) -> Option<u32> {
        self.current.as_ref().map(|c| c.event)
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// A line sounds, is held in its enforced silence, or waits in the queue: the cop channel is busy.
    pub fn busy(&self) -> bool {
        self.current.is_some() || !self.pending.is_empty()
    }

    /// Seconds since a line last sounded (0 while one does; a large number before the first).
    pub fn silent_secs(&self) -> f32 {
        if self.current.as_ref().is_some_and(|c| c.finished.is_none()) {
            return 0.0;
        }
        self.last_sounded.map_or(NEVER_SPOKEN, |t| self.now - t)
    }

    /// Whether `event` waits in the queue.
    pub fn is_queued(&self, event: u32) -> bool {
        self.pending.iter().any(|p| p.event == event)
    }

    /// The event sounding now and the request it was said for (not the silence that follows a line).
    pub fn playing(&self) -> Option<(u32, &Request)> {
        let current = self.current.as_ref().filter(|c| c.finished.is_none())?;
        Some((current.event, &current.request))
    }

    /// The events whose lines ended since the last call, with their requests, oldest first.
    pub fn take_finished(&mut self) -> Vec<(u32, Request)> {
        std::mem::take(&mut self.finished)
    }

    /// Withdraws `event` from the queue (the original's recall).
    pub fn recall(&mut self, event: u32) {
        self.pending.retain(|p| p.event != event);
    }

    /// Forgets what was said (a pursuit reset starts with a clean history); the queue stays.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Withdraws every waiting request but keeps what was said.
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Withdraw every request and forget the history (the pursuit is over).
    pub fn reset(&mut self) {
        self.pending.clear();
        self.history.clear();
    }

    /// The chance that a repeated request is kept: 1 early in a pursuit, falling to 0 along the tune's ramp.
    pub fn keep_probability(&self) -> f32 {
        let [from, to] = self.tune.dropoff_ramp;
        let at = self.pursuit_secs;
        if at < from {
            return 1.0;
        }
        if at > to || to <= from {
            return 0.0;
        }
        1.0 - (at - from) / (to - from)
    }

    /// Ask for `event` to be said.
    pub fn request(&mut self, event: u32, request: Request) -> Outcome {
        if !self.enabled {
            return Outcome::Muted;
        }
        let Some(data) = self.events.get(&event) else { return Outcome::Unknown };
        if let Some(waiting) = self.pending.iter_mut().find(|p| p.event == event) {
            waiting.entry = self.now;
            waiting.request = request;
            return Outcome::Refreshed;
        }
        let repeated = self.history.count(event) > 0;
        if !data.do_not_dropout && repeated && self.rng.next_f32() > self.keep_probability() {
            return Outcome::Thinned;
        }
        let withdrawn = data.recall.clone();
        self.pending.retain(|p| !withdrawn.contains(&p.event));
        let bonus = if data.interrupt { INTERRUPT_BONUS } else { 0 };
        self.seq += 1;
        self.pending.push(Pending {
            event,
            priority: data.priority + bonus,
            request,
            entry: self.now,
            delayed: data.init_delay > 0.0,
            seq: self.seq,
            ready: false,
        });
        Outcome::Queued
    }

    /// Advance the clock by `dt` seconds and start the next line when its turn has come.
    pub fn update(&mut self, dt: f32, context: Context, voice: &mut impl Voice) {
        self.now += dt;
        let busy = voice.busy();
        self.follow(busy);
        self.judge_all(context, busy);
        let Some(index) = self.next() else { return };
        let event = self.pending[index].event;
        let priority = self.pending[index].priority;
        let Some(cut) = self.may_start(self.events[&event].interrupt, priority, busy) else { return };
        let waiting = self.pending.remove(index);
        let data = &self.events[&event];
        if !voice.play(data, &waiting.request, cut) {
            return;
        }
        self.current = Some(Current {
            event,
            request: waiting.request,
            priority,
            interrupt: data.interrupt,
            interruptable: data.interruptable,
            enforce_dead_air: data.enforce_dead_air,
            started: self.now,
            finished: None,
            counted: false,
        });
        self.last_sounded = Some(self.now);
    }

    /// Keep track of the line in progress: when it ends, when it counts as heard, when its silence is over.
    fn follow(&mut self, sounding: bool) {
        let Some(current) = self.current.as_mut() else { return };
        if sounding && current.finished.is_none() {
            self.last_sounded = Some(self.now);
            if !current.counted && self.now - current.started >= HEARD_AFTER {
                current.counted = true;
                self.history.touch(current.event, self.now);
            }
            return;
        }
        let first_look = current.finished.is_none();
        let ended = *current.finished.get_or_insert(self.now);
        if first_look {
            self.finished.push((current.event, current.request));
        }
        if !current.counted {
            current.counted = true;
            self.history.touch(current.event, self.now);
        }
        self.last_sounded = Some(self.last_sounded.map_or(ended, |t| t.max(ended)));
        if self.now - ended >= current.enforce_dead_air {
            self.current = None;
        }
    }

    /// Run the checks on every request: the delays end, the stale ones go, the others are marked ready or not.
    fn judge_all(&mut self, context: Context, sounding: bool) {
        let dead_air = match (sounding, self.last_sounded) {
            (true, _) => 0.0,
            (false, Some(t)) => self.now - t,
            (false, None) => NEVER_SPOKEN,
        };
        let playing = self.current.as_ref().filter(|c| c.finished.is_none()).map(|c| c.event);
        let env = Env { now: self.now, dead_air, history: &self.history, playing, context };
        let now = self.now;
        let mut verdicts = Vec::with_capacity(self.pending.len());
        for waiting in &mut self.pending {
            let data = &self.events[&waiting.event];
            if waiting.delayed && now - waiting.entry < data.init_delay {
                verdicts.push(Verdict::Defer);
                continue;
            }
            if waiting.delayed {
                waiting.delayed = false;
                waiting.entry = now;
            }
            let asked = Asked { entry: waiting.entry, actor: waiting.request.actor };
            let verdict = judge(data, &asked, &env);
            waiting.ready = verdict == Verdict::Keep;
            verdicts.push(verdict);
        }
        let mut verdicts = verdicts.into_iter();
        self.pending.retain(|_| verdicts.next() != Some(Verdict::Ditch));
    }

    /// The ready request that goes first: the highest priority, then the oldest.
    fn next(&self) -> Option<usize> {
        let ready = self.pending.iter().enumerate().filter(|(_, p)| p.ready);
        ready.min_by_key(|(_, p)| (std::cmp::Reverse(p.priority), p.seq)).map(|(i, _)| i)
    }

    /// Whether a line of this priority may start now, and if so whether it cuts the one in progress.
    fn may_start(&self, interrupt: bool, priority: i32, busy: bool) -> Option<bool> {
        let Some(current) = &self.current else {
            // Something else is sounding (a line played by hand): only an interrupt talks over it.
            return if busy { interrupt.then_some(true) } else { Some(false) };
        };
        if let Some(ended) = current.finished {
            // The line is over; the event may enforce some silence after it, which only an interrupt ignores.
            let quiet = self.now - ended >= current.enforce_dead_air;
            return (interrupt || quiet).then_some(false);
        }
        if !interrupt {
            return None;
        }
        let cuts = current.interruptable || (current.interrupt && priority > current.priority);
        cuts.then_some(true)
    }
}
