//! The speech pursuit state and focus: which pursuit is the player's, whether the cops have contact, the lost-contact
//! sequence, and the bail-out and reset (`SyncPursuit`, `TerminatePursuit`, `ResetPursuit`, `DealWithDeadAir`).

use super::actors::Pick;
use super::cx::Cx;
use super::flows::Flows;
use super::line::Line;
use super::output::Effect;
use super::state::{Core, Focus, PursuitState, Quadrant};

/// Why the cops give up on a pursuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bail {
    /// The suspect outran them.
    Outrun,
    /// The game ends the pursuit for them.
    Forced,
}

impl Cx<'_> {
    /// The two inactivity cut-offs: the time without contact before the pursuit is lost, and before it is abandoned.
    fn cutoffs(&self) -> [f32; 2] {
        let extra = self.tune.time_considered_lost_no_los;
        [self.tune.inactivity[0] + extra, self.tune.inactivity[1] + extra]
    }

    /// The pursuit is over for the cops' talk: no pursuit, or one cooling down with time left.
    fn cooling_with_time(&self) -> bool {
        let p = &self.snap.pursuit;
        !p.exists || (p.cooling_down && p.cooldown_remaining > 0.0)
    }

    /// Reads the pursuit and sets the state, the timers and the focus; says the lost-contact lines.
    pub fn sync_pursuit(&mut self, flows: &mut Flows) {
        self.read_pursuit_state();
        let lost_for = self.perp_lost_time();
        match self.core.state {
            PursuitState::Active => {
                self.core.t_no_los = self.now;
                self.core.t_since_last_pursuit = self.now;
            }
            _ => self.core.t_los = self.now,
        }
        let cutoffs = self.cutoffs();
        if self.focus_is(Focus::Lost)
            && self.state_is(PursuitState::Active)
            && self.core.time_since_last_chase < cutoffs[1]
        {
            self.core.focus = Focus::Pursuit;
            self.core.quadrant = Quadrant::Reset;
            self.core.flags.setup_restarted = false;
            self.core.flags.pursuit_expired = false;
            self.clear_queue();
            flows.pursuit.reacquire(self);
        }
        let out_of_contact = matches!(self.core.state, PursuitState::Searching | PursuitState::Inactive);
        if self.focus_is(Focus::Strategy) && out_of_contact && lost_for > cutoffs[0] && self.cooling_with_time() {
            self.terminate_pursuit(Bail::Outrun);
        } else if self.focus_is(Focus::Lost) {
            self.lost_sequence(flows, lost_for, cutoffs, out_of_contact);
        }
        if self.focus_is(Focus::Terminal) {
            self.core.state = PursuitState::Inactive;
        }
        let running = matches!(self.core.state, PursuitState::Active | PursuitState::Searching);
        self.core.duration = match running && self.snap.pursuit.exists {
            true => self.now - self.core.t_pursuit_start,
            false => -1.0,
        };
        self.heli_introduction();
    }

    fn read_pursuit_state(&mut self) {
        let p = self.snap.pursuit;
        if !p.exists {
            let was_running = matches!(self.core.state, PursuitState::Active | PursuitState::Searching);
            self.core.t_pursuit_start = self.now;
            self.core.state = match p.other_target {
                true => PursuitState::OtherTarget,
                false => PursuitState::Inactive,
            };
            if was_running {
                self.core.t_since_last_pursuit = self.now;
            }
            return;
        }
        if !matches!(self.core.state, PursuitState::Active | PursuitState::Searching) && self.core.duration < 0.0 {
            self.core.t_pursuit_start = self.now;
        }
        let searching = p.re_acquiring || !p.perp_in_sight || p.cooling_down;
        let heli_los = self.snap.heli.is_some_and(|h| h.active && h.los);
        if searching || (self.core.los_count == 0 && !heli_los) {
            self.core.state = PursuitState::Searching;
        } else {
            if self.core.state != PursuitState::Active {
                self.core.time_since_last_chase = self.now - self.core.t_since_last_pursuit;
            }
            self.core.state = PursuitState::Active;
        }
        // While the chase runs the 911 counters are cleared: they only count in free roam.
        self.core.traffic_hits_911 = 0;
        self.core.cost_911 = 0;
    }

    /// Focus Lost: the quadrant lines while contact stays lost, "time expired" when the cool-down is over, a reset when
    /// the pursuit is abandoned.
    fn lost_sequence(&mut self, flows: &mut Flows, lost_for: f32, cutoffs: [f32; 2], out_of_contact: bool) {
        if !out_of_contact {
            return;
        }
        let in_window = lost_for > cutoffs[0] && lost_for < cutoffs[1];
        if in_window && !self.busy() && self.cooling_with_time() {
            self.quadrant_step();
            return;
        }
        let p = self.snap.pursuit;
        let finished = !p.exists || p.evaded || (p.cooling_down && p.cooldown_remaining <= 0.0);
        if !finished {
            return;
        }
        if self.core.quadrant == Quadrant::Expired && (!p.exists || p.evaded) && !self.core.flags.pursuit_expired {
            self.clear_queue();
            self.disp_time_expired();
            self.core.quadrant = Quadrant::Reset;
            self.core.flags.pursuit_expired = true;
            return;
        }
        if lost_for > cutoffs[1] {
            self.reset_pursuit(flows);
        }
    }

    /// One step of the lost-contact fiction: forming, then a possible suspect and a wrong one, or one of three
    /// stray remarks, then done.
    fn quadrant_step(&mut self) {
        match self.core.quadrant {
            Quadrant::Initial => {
                self.quadrant_forming();
                self.core.quadrant = Quadrant::Forming;
            }
            Quadrant::Forming => {
                if self.rng.coin() {
                    self.core.quadrant = Quadrant::Fiction2;
                    self.possible_suspect();
                    return;
                }
                self.core.quadrant = Quadrant::Fiction1;
                match self.rng.below(3) {
                    0 => self.suspect_possibly_gone(),
                    1 => self.quadrant_moving(),
                    _ => self.other_lead(),
                }
            }
            Quadrant::Fiction2 => {
                self.wrong_suspect();
                self.core.quadrant = Quadrant::Expired;
            }
            Quadrant::Fiction1 | Quadrant::Expired | Quadrant::Reset => self.core.quadrant = Quadrant::Expired,
        }
    }

    fn heli_introduction(&mut self) {
        if !self.core.flags.heli_intro_requested
            || !self.focus_is(Focus::Strategy)
            || self.state_is(PursuitState::Inactive)
        {
            return;
        }
        let Some(heli) = self.roster.actors.iter().find(|a| a.heli).map(|a| a.id) else { return };
        self.backup_arrives(heli);
        self.core.flags.heli_intro_requested = false;
    }

    /// The cops give up: a forced bail-out stops everything; an outrun says "lost suspect" (when no cop is in view) and
    /// dispatch's break-away, and the focus turns to the lost-contact sequence.
    pub fn terminate_pursuit(&mut self, how: Bail) {
        if how == Bail::Forced {
            self.clear_queue();
            let voice = self.random_active(Pick::Any, false).and_then(|id| self.roster.find(id)).map_or(0, |a| a.voice);
            self.misc_bailout(voice);
            self.core.focus = Focus::Terminal;
            return;
        }
        let cops_visible = self.snap.cops.iter().any(|c| c.in_view);
        self.clear_queue();
        if !cops_visible {
            let last = self.chan.last().map_or(0, |(_, speaker)| speaker);
            self.misc_lost_suspect(last);
        }
        self.disp_break_away();
        self.core.focus = Focus::Lost;
        self.core.quadrant = Quadrant::Initial;
        self.core.flags.setup_restarted = false;
    }

    /// Everything back to the start: the flows, the actors, the timers and the history.
    pub fn reset_pursuit(&mut self, flows: &mut Flows) {
        flows.reset();
        self.effect(Effect::ResetHistory);
        let now = self.now;
        self.roster.clear();
        // What the original keeps through a reset: the heading it follows, the blow-by memory, the heat it last
        // announced and when the last chase ended.
        let old = self.core.clone();
        *self.core = Core::new();
        self.core.t_pursuit_start = now;
        self.core.t_since_last_pursuit = old.t_since_last_pursuit;
        self.core.time_since_last_chase = old.time_since_last_chase;
        self.core.blowby = old.blowby;
        self.core.prev_heat = old.prev_heat;
        self.core.current = old.current;
        self.core.last_known = old.last_known;
        self.core.smoothed_forward = old.smoothed_forward;
        self.core.direction_tracking = old.direction_tracking;
        self.core.t_direction = old.t_direction;
    }

    /// While the pursuit is active and the channel has been silent: dispatch asks for an update, the leader answers or
    /// reports the place, and the cops notice when contact is lost.
    pub fn deal_with_dead_air(&mut self) {
        if !self.state_is(PursuitState::Active) || self.focus_is(Focus::Pursuit) {
            return;
        }
        let Some(leader) = self.roster.leader else { return };
        if !self.chan.queued(Line::DispPursuitUpdate) && self.core.duration > 60.0 {
            self.disp_pursuit_update(leader);
        }
        let heli = self.roster.actors.iter().find(|a| a.heli).copied();
        // The original also asks for a speech-map place; the game cannot supply one, so the heading decides.
        let place_known = self.heading_valid();
        if self.perp_lost_time() >= self.tune.no_los_commentary {
            match heli {
                None => self.lost_visual(leader),
                Some(h) if h.los => match !place_known || self.rng.coin() {
                    true => self.pursuit_update_reply(h.id),
                    false => self.location_report(h.id),
                },
                Some(h) => self.heli_lost_visual(h.id),
            }
        } else if !self.chan.queued(Line::PursuitUpdateReply)
            && !self.chan.queued(Line::LocationReport)
            && self.core.duration > 60.0
        {
            match !place_known || self.rng.coin() {
                true => self.pursuit_update_reply(leader),
                false => self.location_report(leader),
            }
        }
        if self.chan.queued(Line::LocationReport) && self.has_been_said(Line::DispPursuitUpdate) {
            self.pursuit_update_reply(leader);
        }
    }
}
