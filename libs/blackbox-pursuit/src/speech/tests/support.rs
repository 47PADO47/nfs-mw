//! A scripted chase: a layout, a tune, a radio channel that plays what is asked, and a scene to edit.

use std::collections::HashMap;

use glam::Vec3;

use super::super::*;
use crate::infraction::Infraction;

pub const DISPATCH: Speaker = 1;
pub const HELI: Speaker = 2;
pub const CITY: Battalion = Battalion(4);
pub const COASTAL: Battalion = Battalion(2);
pub const RHINO: Battalion = Battalion(32);

pub fn layout() -> Layout {
    static POOLS: [Pool; 3] =
        [Pool { battalion: CITY, size: 20 }, Pool { battalion: COASTAL, size: 10 }, Pool { battalion: RHINO, size: 6 }];
    Layout {
        dispatch: DISPATCH,
        heli: HELI,
        primary: &[3, 4, 5],
        secondary: &[6, 7, 8],
        cross: 9,
        heli_unit: (Battalion(64), 1),
        pools: &POOLS,
        random_battalions: &[CITY, COASTAL],
        placeable_roads: (1, 80),
        locatable_roads: (1, 87),
    }
}

pub fn tune() -> Tune {
    Tune {
        pursuit_init_delay: 2.0,
        range_for_spotter: 100.0,
        speed_diff_for_blowby: 60.0,
        blowby_interval: 10.0,
        suspect_outrun_range: 120.0,
        time_considered_lost_no_los: 3.0,
        inactivity: [2.0, 150.0],
        no_los_commentary: 2.0,
        min_speed_stopped: 5.0,
        min_time_stopped: 2.0,
        high_intensity_heat: 2.0,
        duration_high_intensity: 150.0,
        min_bailout_duration: 60.0,
        min_health_commentary: 0.33,
        crash_slowdown: 0.66,
        smash_range: [10.0, 30.0],
        min_intensity_traffic: 0.5,
        min_intensity_cop_smash: 0.85,
        min_havoc: 200.0,
        min_height_airborne: 0.5,
        hang_time: 0.9,
        flip_time: 1.0,
        fly_flip_speed: 100.0,
        contig_frames_180: 12,
        range_180: 50.0,
        time_180: 5.0,
        outcome_track_time: 3.0,
        outcome_fail_speed: 20.0,
        rb_outcome_timer: 1.5,
        rb_post_outcome_reset: 2.0,
        backup_remind_time: 20.0,
        racer_proximity: 145.0,
    }
}

/// A radio that plays each request for `hold` seconds, one at a time, and remembers what was said.
pub struct Radio {
    pub now: f32,
    hold: f32,
    pending: Vec<Request>,
    sounding: Option<(Request, f32)>,
    history: HashMap<Line, u32>,
    started: HashMap<Line, f32>,
    last: Option<(Line, Speaker)>,
    last_sounded: f32,
    /// Every request the machine made, with the time.
    pub log: Vec<(f32, Request)>,
    /// Every line that was played, with the time it started.
    pub played: Vec<(f32, Request)>,
    pub effects: Vec<Effect>,
    notices: Vec<Notice>,
}

impl Radio {
    pub fn new(hold: f32) -> Self {
        Self {
            now: 0.0,
            hold,
            pending: Vec::new(),
            sounding: None,
            history: HashMap::new(),
            started: HashMap::new(),
            last: None,
            last_sounded: 0.0,
            log: Vec::new(),
            played: Vec::new(),
            effects: Vec::new(),
            notices: Vec::new(),
        }
    }

    /// Takes the machine's output: a request waits its turn (a request for a line already waiting replaces it).
    pub fn take(&mut self, out: &Output) {
        for request in &out.requests {
            // The queue drops a line said less than twenty seconds ago (the events' default interval).
            let interval = match request.line {
                Line::AttemptVehicleStop => 0.0,
                _ => 20.0,
            };
            let recent = self.started.get(&request.line).is_some_and(|t| self.now - t < interval);
            if recent {
                continue;
            }
            self.log.push((self.now, *request));
            match self.pending.iter_mut().find(|p| p.line == request.line) {
                Some(waiting) => *waiting = *request,
                None => self.pending.push(*request),
            }
        }
        for effect in &out.effects {
            self.effects.push(*effect);
            if matches!(effect, Effect::ClearQueue) {
                self.pending.clear();
                self.sounding = None;
            }
            if let Effect::Recall(line) = effect {
                self.pending.retain(|p| p.line != *line);
            }
            if matches!(effect, Effect::ResetHistory) {
                self.history.clear();
            }
        }
    }

    /// Advances the clock: the line that sounds ends, the next one starts.
    pub fn advance(&mut self, dt: f32) {
        self.now += dt;
        if let Some((request, end)) = self.sounding
            && self.now >= end
        {
            self.sounding = None;
            *self.history.entry(request.line).or_insert(0) += 1;
            self.last = Some((request.line, request.speaker));
            self.last_sounded = self.now;
            self.notices.push(Notice::Said { line: request.line, speaker: request.speaker, params: request.params });
        }
        if self.sounding.is_none() && !self.pending.is_empty() {
            let next = self.pending.remove(0);
            self.played.push((self.now, next));
            self.started.insert(next.line, self.now);
            self.sounding = Some((next, self.now + self.hold));
        }
    }

    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    pub fn lines(&self) -> Vec<Line> {
        self.log.iter().map(|(_, r)| r.line).collect()
    }

    pub fn played_lines(&self) -> Vec<Line> {
        self.played.iter().map(|(_, r)| r.line).collect()
    }

    pub fn count(&self, line: Line) -> usize {
        self.log.iter().filter(|(_, r)| r.line == line).count()
    }

    pub fn first(&self, line: Line) -> Option<Request> {
        self.log.iter().map(|(_, r)| *r).find(|r| r.line == line)
    }

    pub fn all(&self, line: Line) -> Vec<Request> {
        self.log.iter().map(|(_, r)| *r).filter(|r| r.line == line).collect()
    }
}

impl Channel for Radio {
    fn busy(&self) -> bool {
        self.sounding.is_some() || !self.pending.is_empty()
    }

    fn dead_air(&self) -> f32 {
        match self.busy() {
            true => 0.0,
            false => self.now - self.last_sounded,
        }
    }

    fn said(&self, line: Line) -> u32 {
        self.history.get(&line).copied().unwrap_or(0)
    }

    fn queued(&self, line: Line) -> bool {
        self.pending.iter().any(|p| p.line == line)
    }

    fn playing(&self) -> Option<(Line, Params)> {
        self.sounding.map(|(r, _)| (r.line, r.params))
    }

    fn last(&self) -> Option<(Line, Speaker)> {
        self.last
    }
}

/// A chase to script: the machine, the radio and the scene it reads.
pub struct Chase {
    pub machine: Machine,
    pub radio: Radio,
    pub snap: Snapshot,
    pub notices: Vec<Notice>,
    pub dt: f32,
}

/// A cop on the road, `distance` metres from the player, in sight.
pub fn cop(id: u32, distance: f32) -> CopInfo {
    CopInfo {
        id,
        active: true,
        in_view: true,
        los: true,
        distance,
        speed_mph: 80.0,
        position: Vec3::new(distance, 0.0, 0.0),
        road_battalion: Some(CITY),
        ..CopInfo::default()
    }
}

impl Chase {
    /// A chase with one cop in sight and the player doing 80 mph on road 5, heading along +Z (south).
    pub fn new(seed: u64) -> Self {
        let snap = Snapshot {
            pursuit: PursuitInfo { exists: true, perp_in_sight: true, heat: 1.0, ..PursuitInfo::default() },
            player: PlayerInfo {
                speed_mph: 80.0,
                velocity_dir: Vec3::Z,
                forward: Vec3::Z,
                road: Some(5),
                car: Some(Car { kind: 2, colour: 1 }),
                ..PlayerInfo::default()
            },
            cops: vec![cop(10, 40.0)],
            ..Snapshot::default()
        };
        Self {
            machine: Machine::new(layout(), tune(), seed),
            radio: Radio::new(1.0),
            snap,
            notices: Vec::new(),
            dt: 0.1,
        }
    }

    /// Runs `secs` of the scene.
    pub fn run(&mut self, secs: f32) {
        let steps = (secs / self.dt).round() as u32;
        for _ in 0..steps {
            let mut notices = self.radio.take_notices();
            notices.append(&mut self.notices);
            let out = self.machine.step(self.dt, &self.snap, &notices, &self.radio);
            self.radio.take(&out);
            self.radio.advance(self.dt);
        }
    }

    pub fn infraction(&mut self, infraction: Infraction) {
        self.notices.push(Notice::Infraction(infraction));
    }

    /// The cops lose the player: nobody sees it.
    pub fn lose_sight(&mut self) {
        self.snap.pursuit.perp_in_sight = false;
        for cop in &mut self.snap.cops {
            cop.los = false;
            cop.in_view = false;
        }
    }

    pub fn regain_sight(&mut self) {
        self.snap.pursuit.perp_in_sight = true;
        for cop in &mut self.snap.cops {
            cop.los = true;
            cop.in_view = true;
        }
    }
}
