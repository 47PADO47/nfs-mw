//! Normal chase commentary (`StrategyFlow`): the leader announces the cops' formation or says what a lone cop will do,
//! calls cops into position, asks for backup, comments on how a strategy turned out, and says when the suspect is
//! outrunning the cops. Spec: `docs/specs/ai-pursuit-speech.md` section 6.1.

use super::cx::Cx;
use super::input::Strategy;
use super::line::Line;
use super::params::{Backup, HeliStrategy, Params};
use super::state::{Focus, PursuitState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Transition,
    CullCheck,
    SoloCheck,
    Outrun,
    /// Contact lost: the flow only waits to terminate.
    Lost,
    ReqBackup,
    CallToPos,
    Waiting,
    Terminal,
    Outcome,
}

#[derive(Debug, Clone)]
pub struct StrategyFlow {
    pub step: Step,
    last_step: Step,
    busy: u32,
    solo: bool,
    backup_denied: bool,
    distance: [f32; 2],
    speed: [f32; 2],
    formation_count: usize,
    formation: Option<Strategy>,
    backup: Backup,
    t_requested: f32,
    last_backup: Option<Backup>,
}

impl Default for StrategyFlow {
    fn default() -> Self {
        Self {
            step: Step::CullCheck,
            last_step: Step::Transition,
            busy: 0,
            solo: false,
            backup_denied: false,
            distance: [0.0; 2],
            speed: [0.0; 2],
            formation_count: 0,
            formation: None,
            backup: Backup::GroundUnits,
            t_requested: f32::NEG_INFINITY,
            last_backup: None,
        }
    }
}

impl StrategyFlow {
    pub fn change_to(&mut self, step: Step) {
        if step != self.step {
            self.last_step = self.step;
        }
        self.step = step;
    }

    /// The flow may hand over when it waits or is done.
    pub fn is_transitionable(&self) -> bool {
        matches!(self.step, Step::Transition | Step::Waiting)
    }

    pub fn reset(&mut self) {
        *self = Self { step: Step::Transition, ..Self::default() };
    }

    /// Backup was requested (by a heavy support call): the flow asks for it on the radio.
    pub fn backup_requested(&mut self, cx: &Cx, backup: Backup, denied: bool) {
        self.backup = backup;
        self.backup_denied = denied;
        if cx.state_is(PursuitState::Active) && cx.focus_is(Focus::Strategy) {
            self.change_to(Step::ReqBackup);
        }
    }

    /// A line finished playing: the backup calls remember what was asked and when.
    pub fn on_said(&mut self, now: f32, line: Line, params: &Params) {
        if !matches!(line, Line::CallForBackup | Line::BackupReminder) {
            return;
        }
        if let Some(backup) = params.backup {
            self.last_backup = Some(backup);
            self.t_requested = now;
        }
    }

    pub fn update(&mut self, cx: &mut Cx) {
        match self.step {
            Step::Transition => {}
            Step::CullCheck => self.cull_check(cx),
            Step::SoloCheck => self.solo_check(cx),
            Step::Waiting => self.waiting(cx),
            Step::Terminal => self.terminal(),
            Step::Outcome => self.outcome(cx),
            Step::Outrun => self.outrun(cx),
            Step::Lost => self.lost(cx),
            Step::ReqBackup => self.request_backup(cx),
            Step::CallToPos => self.call_to_position(cx),
        }
    }

    fn cull_check(&mut self, cx: &Cx) {
        if matches!(cx.core.state, PursuitState::Active | PursuitState::Searching) {
            self.change_to(Step::SoloCheck);
        }
    }

    fn done(&mut self, cx: &Cx) -> bool {
        if cx.state_is(PursuitState::Inactive) || !cx.focus_is(Focus::Strategy) {
            self.change_to(Step::Terminal);
            return true;
        }
        false
    }

    /// The leader announces the formation, or a lone cop what it will do; a single cop (or none in sight) asks for backup.
    fn solo_check(&mut self, cx: &mut Cx) {
        self.distance[0] = cx.core.pursuit_dist;
        self.speed[0] = cx.snap.player.speed_mph;
        if self.done(cx) {
            return;
        }
        let Some(leader) = cx.roster.leader else { return };
        if cx.busy() {
            return;
        }
        if cx.roster.active().len() == 1 || cx.core.los_count == 0 {
            self.backup = Backup::GroundUnits;
            self.change_to(Step::ReqBackup);
            return;
        }
        let strategy = cx.snap.pursuit.strategy;
        self.formation = Some(strategy);
        let alone = cx.core.los_count <= 1 || cx.roster.formation.len() <= 1;
        let leader_info = cx.roster.find(leader).copied();
        if alone {
            match leader_info {
                Some(l) if l.heli && l.active => cx.heli_self_strategy(leader, HeliStrategy::Follow),
                Some(l) if l.active => cx.self_strategy(leader, strategy),
                _ => {}
            }
            self.solo = true;
        } else if leader_info.is_some_and(|l| l.active) {
            cx.initiate_strategy(leader, strategy);
            self.solo = false;
        }
        self.distance[0] = cx.core.pursuit_dist;
        self.change_to(Step::Waiting);
    }

    /// Each cop in formation that is not in position is called there by the leader (the reminder if one was said).
    fn call_to_position(&mut self, cx: &mut Cx) {
        if self.done(cx) {
            return;
        }
        let Some(leader) = cx.roster.leader else { return };
        if cx.is_heli(leader) {
            self.change_to(Step::Waiting);
            return;
        }
        if !cx.roster.find(leader).is_some_and(|l| l.active) {
            return;
        }
        let callees: Vec<u32> =
            cx.roster.actors.iter().filter(|a| a.in_formation && a.active && !a.in_position).map(|a| a.id).collect();
        for cop in callees {
            match cx.has_been_said(Line::CallToPosition) {
                true => cx.call_to_position_reminder(leader),
                false => cx.call_to_position(leader, cop),
            }
            let last = cx.chan.last().map(|(line, _)| line);
            let acknowledged = matches!(last, Some(Line::CallToPosition | Line::CallToPositionReminder));
            if acknowledged && !cx.chan.queued(Line::Acknowledge) {
                cx.ack(cop);
            }
        }
        self.change_to(Step::Waiting);
    }

    fn eta_valid(cx: &Cx) -> bool {
        let eta = cx.snap.pursuit.backup_eta;
        eta > 10.0 && eta < 200.0
    }

    /// Dispatch answers a backup request with an arrival time (half the time, when it is known) or a yes or no.
    fn dispatch_answers(&mut self, cx: &mut Cx, caller: u32, update_only: bool) {
        if Self::eta_valid(cx) && cx.rng.coin() && !self.backup_denied {
            cx.disp_backup_eta();
            return;
        }
        match update_only {
            true => cx.disp_backup_update(!self.backup_denied),
            false => {
                cx.disp_backup_reply(caller, !self.backup_denied, self.backup);
                if self.backup_denied {
                    cx.negative_backup_reply(caller);
                }
            }
        }
    }

    /// One cop asks for backup: the first call, a call, a reminder, and at last the call for swarming.
    fn request_backup(&mut self, cx: &mut Cx) {
        if self.done(cx) || !cx.snap.pursuit.exists {
            return;
        }
        if !cx.busy() && self.busy == 0 {
            let Some(caller) = cx.roster.active().first().copied() else { return };
            self.say_backup_call(cx, caller);
            return;
        }
        if self.busy != 0 && !cx.busy() {
            self.busy = 0;
            let back = self.last_step;
            self.change_to(back);
        }
    }

    fn say_backup_call(&mut self, cx: &mut Cx, caller: u32) {
        let high = cx.high_intensity();
        if !cx.has_been_said(Line::InitialCallForBackup) && !high {
            cx.initial_call_for_backup(caller);
            self.dispatch_answers(cx, caller, false);
        } else if !cx.has_been_said(Line::CallForBackup) || cx.core.active_cops > 1 {
            cx.call_for_backup(caller, self.backup);
            self.dispatch_answers(cx, caller, false);
        } else if !cx.has_been_said(Line::BackupReminder) && !high {
            let due = cx.now - self.t_requested >= cx.tune.backup_remind_time && self.last_backup == Some(self.backup);
            match due {
                true => cx.backup_reminder(caller, self.backup),
                false => cx.call_for_backup(caller, self.backup),
            }
            self.dispatch_answers(cx, caller, true);
        } else {
            self.call_for_swarming(cx, caller);
        }
        self.busy += 1;
    }

    fn call_for_swarming(&mut self, cx: &mut Cx, caller: u32) {
        cx.call_for_swarming(caller);
        if self.backup_denied {
            cx.disp_backup_update(false);
            return;
        }
        cx.disp_backup_eta();
        let heli = cx.roster.actors.iter().find(|a| a.heli).map(|a| a.id);
        match heli {
            Some(heli) if cx.rng.coin() => cx.heli_swarming(heli),
            _ => cx.swarming_reply(),
        }
        cx.swarming_reply_follow();
    }

    fn waiting(&mut self, cx: &mut Cx) {
        if cx.state_is(PursuitState::Inactive) {
            self.change_to(Step::Terminal);
            return;
        }
        let Some(leader) = cx.roster.leader else { return };
        if cx.busy() {
            return;
        }
        if cx.state_is(PursuitState::Searching) {
            self.change_to(Step::Outrun);
            return;
        }
        let active = cx.roster.active();
        if active.is_empty() {
            self.change_to(Step::Terminal);
            return;
        }
        if active.len() == 1 {
            self.solo = true;
        }
        let strategy = cx.snap.pursuit.strategy;
        let changed =
            self.formation != Some(strategy) && !matches!(strategy, Strategy::Follow | Strategy::StaggerFollow);
        if changed && active.len() > 1 {
            cx.strategy_reset(leader, true);
            self.change_to(Step::SoloCheck);
            return;
        }
        if cx.snap.pursuit.collapse_active || cx.snap.pursuit.finisher_active {
            self.finish(cx, leader, strategy, active.len());
            return;
        }
        if cx.state_is(PursuitState::Active) {
            let contact = matches!(strategy, Strategy::Pit | Strategy::BoxIn | Strategy::RollingBlock | Strategy::Herd);
            if contact && cx.roster.formation.is_empty() && active.len() > 1 {
                self.change_to(Step::CallToPos);
                return;
            }
            let members: Vec<u32> = cx.roster.actors.iter().filter(|a| a.in_formation).map(|a| a.id).collect();
            for member in members {
                cx.bullhorn(member);
            }
            if cx.core.pursuit_dist > cx.tune.suspect_outrun_range {
                self.change_to(Step::Outrun);
                return;
            }
        }
        let lost_members = cx.roster.formation.len() < self.formation_count;
        if lost_members || cx.snap.pursuit.evade_level > 0.25 || active.len() == 1 {
            self.formation_count = cx.roster.formation.len();
            self.backup = Backup::GroundUnits;
            self.change_to(Step::ReqBackup);
            return;
        }
        self.formation_count = cx.roster.formation.len();
    }

    /// The formation is closing in (collapse) or the finisher runs: the cops announce the move.
    fn finish(&mut self, cx: &mut Cx, leader: u32, strategy: Strategy, active: usize) {
        match strategy {
            Strategy::Pit if active >= 2 => {
                match cx.rng.coin() {
                    true => {
                        if let Some(closest) = cx.roster.closest(false, false) {
                            cx.intent_to_ram(closest);
                        }
                    }
                    false => cx.strategy_execute(leader),
                }
                self.change_to(Step::Outcome);
            }
            Strategy::BoxIn | Strategy::RollingBlock if active >= 2 => {
                match cx.rng.coin() {
                    true => cx.strategy_execute(leader),
                    false => self.call_to_position(cx),
                }
                self.change_to(Step::Outcome);
            }
            Strategy::HeliPursuit => {
                if let Some(heli) = cx.roster.actors.iter().find(|a| a.heli).map(|a| a.id) {
                    cx.heli_self_strategy(heli, HeliStrategy::Follow);
                }
            }
            Strategy::Pit | Strategy::BoxIn | Strategy::RollingBlock => {}
            Strategy::Follow | Strategy::Herd | Strategy::StaggerFollow => {
                if let Some(closest) = cx.roster.closest(false, false) {
                    cx.bullhorn(closest);
                }
            }
        }
    }

    /// With one cop in sight far behind, "he's outrunning us"; with none in sight, the closest cop (or the
    /// helicopter) says the visual is lost.
    fn outrun(&mut self, cx: &mut Cx) {
        if !matches!(cx.core.state, PursuitState::Searching | PursuitState::Active) {
            self.change_to(Step::Terminal);
            return;
        }
        if cx.roster.leader.is_none() || cx.busy() {
            return;
        }
        let seeing = cx.roster.with_los();
        if seeing.len() > 1 {
            self.change_to(Step::Waiting);
            return;
        }
        let far = !cx.core.flags.cops_ahead && cx.core.pursuit_dist >= cx.tune.suspect_outrun_range;
        if let (1, true) = (seeing.len(), far) {
            cx.suspect_outrun(seeing[0]);
            self.change_to(Step::Waiting);
            return;
        }
        if !seeing.is_empty() || cx.perp_lost_time() < cx.tune.no_los_commentary {
            self.change_to(Step::Waiting);
            return;
        }
        self.say_lost_visual(cx);
        self.change_to(Step::Lost);
    }

    fn say_lost_visual(&mut self, cx: &mut Cx) {
        let Some(speaker) = cx.roster.active().first().copied() else { return };
        if cx.is_heli(speaker) {
            cx.heli_lost_visual(speaker);
            return;
        }
        cx.lost_visual(speaker);
        let heli = cx.roster.actors.iter().find(|a| a.heli && a.active && a.los).map(|a| a.id);
        if let Some(heli) = heli {
            cx.heli_spotter(heli);
            cx.location_report(heli);
        }
    }

    fn lost(&mut self, cx: &Cx) {
        if cx.state_is(PursuitState::Inactive) {
            return;
        }
        self.change_to(Step::Terminal);
    }

    fn terminal(&mut self) {
        *self = Self { step: Step::Transition, last_step: self.last_step, ..Self::default() };
    }

    /// Some seconds after a strategy line: the pursuit grew apart (the strategy failed) or held (it works).
    fn outcome(&mut self, cx: &mut Cx) {
        if !cx.snap.pursuit.exists {
            return;
        }
        let Some(leader) = cx.roster.leader else { return };
        if cx.busy() {
            return;
        }
        self.distance[1] = cx.core.pursuit_dist;
        self.speed[1] = cx.snap.player.speed_mph;
        let grew = self.distance[1] - self.distance[0] > 0.0 || self.speed[1] - self.speed[0] > 0.0;
        if grew {
            let far = cx.core.pursuit_dist > 50.0 || cx.core.los_count == 0 || !cx.core.flags.cops_ahead;
            match far {
                true => {
                    let spokesperson = match cx.is_heli(leader) {
                        true => cx.roster.closest(false, false),
                        false => Some(leader),
                    };
                    if let Some(spokesperson) = spokesperson {
                        cx.anticipate_fail(spokesperson);
                    }
                }
                false => cx.strategy_reset(leader, self.formation != Some(cx.snap.pursuit.strategy)),
            }
        } else if !self.solo {
            cx.anticipate_success(leader);
        }
        self.change_to(Step::Waiting);
    }
}
