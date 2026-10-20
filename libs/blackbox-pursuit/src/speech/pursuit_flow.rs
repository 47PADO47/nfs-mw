//! The opening exchange of a chase (`PursuitFlow`): the first cop's "attempt vehicle stop", dispatch's go-ahead, the
//! vehicle report; or a spotter and dispatch's escalation when the player blew past a cop or a 911 call started the
//! chase. Spec: `docs/specs/ai-pursuit-speech.md` section 5.

use super::actors::Pick;
use super::cx::Cx;
use super::line::{Line, Speaker};
use super::params::{Intensity, Params, PursuitType};
use super::state::PursuitState;

/// The flow's states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Done: the focus may pass to the strategy flow.
    Transition,
    CullCheck,
    CloseInCheck,
    PrimaryBranch,
    SpotterBranch,
    ScriptedBranch,
    WaitForSpotter,
    LostWhileSpotterWait,
    PlayerStopped,
    Bailout,
    ChangeTarget,
    Terminal,
}

/// What started the chase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Reported911,
    CopAssaulted,
    Spotted,
    Reacquired,
    Scripted,
    CopAssaultedScripted,
    Unknown,
}

impl Cause {
    pub(super) fn is_assault(self) -> bool {
        matches!(self, Cause::CopAssaulted | Cause::CopAssaultedScripted)
    }
}

#[derive(Debug, Clone)]
pub struct PursuitFlow {
    pub step: Step,
    pub(super) busy: u32,
    pub cause: Cause,
    pub(super) first_on_scene: Option<u32>,
    request_restart: bool,
    unit_rammed_said: bool,
    speaker: Speaker,
}

impl Default for PursuitFlow {
    fn default() -> Self {
        Self {
            step: Step::CullCheck,
            busy: 0,
            cause: Cause::Unknown,
            first_on_scene: None,
            request_restart: false,
            unit_rammed_said: false,
            speaker: 0,
        }
    }
}

impl PursuitFlow {
    pub fn is_transitionable(&self) -> bool {
        self.step == Step::Transition
    }

    pub fn reset(&mut self) {
        let keep = (self.request_restart, self.unit_rammed_said, self.speaker);
        *self = Self::default();
        // The original's reset keeps these (it clears only the scene, the busy count, the state and the cause).
        (self.request_restart, self.unit_rammed_said, self.speaker) = keep;
    }

    pub fn on_cop_removed(&mut self, id: u32) {
        if self.first_on_scene == Some(id) {
            self.first_on_scene = None;
        }
    }

    /// Contact was regained after a loss: the leader says "re-initiate pursuit" (unless a 911 call is on).
    pub fn reacquire(&mut self, cx: &mut Cx) {
        if cx.core.flags.disp911_active {
            return;
        }
        self.cause = Cause::Reacquired;
        self.step = Step::PrimaryBranch;
    }

    /// A deliberate hit on a cop during the opening restarts it with "unit rammed", unless that was said already.
    /// Returns whether the flow restarts.
    pub fn requires_restart(&mut self, cx: &mut Cx) -> bool {
        if cx.chan.is_playing(Line::AttemptVehicleStop) {
            let rammed = cx.chan.playing().is_some_and(|(_, p)| p.pursuit_type == Some(PursuitType::UnitRammed));
            if rammed {
                self.cause = Cause::CopAssaulted;
                self.unit_rammed_said = true;
            }
        }
        if self.unit_rammed_said {
            return false;
        }
        self.request_restart = true;
        self.busy = 0;
        true
    }

    /// Restarts the opening because a cop was assaulted.
    pub fn restart_assaulted(&mut self, cx: &mut Cx) {
        self.cause = match self.cause {
            Cause::Scripted => Cause::CopAssaultedScripted,
            _ => Cause::CopAssaulted,
        };
        self.step = Step::PrimaryBranch;
        self.update(cx);
    }

    /// A line finished playing.
    pub fn on_said(&mut self, cx: &mut Cx, line: Line, params: &Params) {
        match line {
            Line::AttemptVehicleStop if params.pursuit_type == Some(PursuitType::UnitRammed) => {
                self.unit_rammed_said = true
            }
            Line::ReinitiatePursuit if cx.snap.pursuit.pursuits > 2 => cx.escalate_by_coin(),
            _ => {}
        }
    }

    pub fn update(&mut self, cx: &mut Cx) {
        match self.step {
            Step::Transition => {}
            Step::CullCheck => self.cull_check(cx),
            Step::CloseInCheck => self.close_in_check(cx),
            Step::PrimaryBranch => self.primary_branch(cx),
            Step::SpotterBranch => self.spotter_branch(cx),
            Step::ScriptedBranch => self.scripted_branch(cx),
            Step::WaitForSpotter => self.spotter_wait(cx),
            Step::LostWhileSpotterWait => self.lost_while_spotter_wait(cx),
            Step::PlayerStopped => self.player_stopped(cx),
            Step::Bailout => self.bailout(cx),
            Step::ChangeTarget => self.change_target(cx),
            Step::Terminal => self.terminal(cx),
        }
    }

    fn cull_check(&mut self, cx: &mut Cx) {
        if cx.state_is(PursuitState::Active) {
            self.step = Step::CloseInCheck;
        }
    }

    /// The cause of the chase, once the pursuit has run for the initial delay.
    fn close_in_check(&mut self, cx: &mut Cx) {
        if cx.state_is(PursuitState::Inactive) || cx.roster.actors.is_empty() {
            self.reset();
            return;
        }
        if cx.core.duration < cx.tune.pursuit_init_delay {
            return;
        }
        let blowby = cx.core.blowby;
        let recent = cx.now - blowby.at;
        let passed_a_cop = blowby.distance < cx.tune.range_for_spotter
            && blowby.speed >= cx.tune.speed_diff_for_blowby
            && recent < cx.tune.blowby_interval
            && cx.snap.player.speed_mph >= 35.0
            && !cx.core.flags.cops_ahead;
        if passed_a_cop {
            self.cause = Cause::Spotted;
        }
        if cx.core.flags.disp911_active {
            self.cause = Cause::Reported911;
        }
        if cx.snap.level.pursuit_race {
            self.cause = Cause::Scripted;
        }
        if cx.time_last_nailed_cop() < 10.0 && self.cause != Cause::Scripted {
            self.cause = Cause::CopAssaulted;
        }
        self.step = match self.cause {
            Cause::Spotted => Step::SpotterBranch,
            Cause::CopAssaulted | Cause::CopAssaultedScripted | Cause::Unknown => Step::PrimaryBranch,
            Cause::Reported911 => Step::WaitForSpotter,
            Cause::Scripted | Cause::Reacquired => Step::ScriptedBranch,
        };
    }

    /// The branches that wait for a state change of the pursuit: bail out, or follow the other target.
    pub(super) fn pursuit_gone(&mut self, cx: &Cx) -> bool {
        if cx.state_is(PursuitState::Inactive) {
            self.step = Step::Bailout;
            return true;
        }
        if cx.state_is(PursuitState::OtherTarget) {
            self.step = Step::ChangeTarget;
            return true;
        }
        false
    }

    fn primary_branch(&mut self, cx: &mut Cx) {
        if self.pursuit_gone(cx) {
            return;
        }
        if self.busy == 0 {
            self.say_opening(cx);
            return;
        }
        if cx.player_stop_time() >= cx.tune.min_time_stopped && !self.cause.is_assault() {
            self.busy = 0;
            self.step = Step::PlayerStopped;
            return;
        }
        if cx.busy() || self.request_restart {
            return;
        }
        self.step = Step::Terminal;
        self.busy = 0;
    }

    /// The closest cop opens: "attempt vehicle stop" (or "re-initiate pursuit"), dispatch's go-ahead and the vehicle
    /// report; with several pursuits a confirmation or the driver history.
    fn say_opening(&mut self, cx: &mut Cx) {
        let Some(leader) = cx.roster.closest(false, false) else {
            self.step = Step::CullCheck;
            return;
        };
        if !cx.roster.is_primary(cx.layout, leader) && !cx.make_leader(leader) {
            self.step = Step::CullCheck;
            return;
        }
        if self.cause == Cause::Reacquired {
            cx.reinitiate_pursuit(leader);
            self.busy += 1;
            return;
        }
        if self.request_restart {
            self.restore_speaker(cx, leader);
            self.request_restart = false;
        }
        cx.attempt_vehicle_stop(leader);
        self.speaker = cx.roster.find(leader).map_or(0, |a| a.voice);
        if cx.roster.leader.is_none() {
            cx.make_leader(leader);
        }
        cx.disp_go_ahead();
        if !self.cause.is_assault() {
            cx.vehicle_report(leader);
        }
        if self.cause == Cause::CopAssaultedScripted {
            self.step = Step::Terminal;
            return;
        }
        self.confirm_with_other_pursuits(cx, leader);
        self.busy += 1;
    }

    /// After a restart the first speaker's voice goes back to the cop that has it.
    fn restore_speaker(&mut self, cx: &mut Cx, leader: u32) {
        let voice_now = cx.roster.find(leader).map(|a| a.voice);
        if self.speaker == 0 || voice_now == Some(self.speaker) {
            return;
        }
        let Some(original) = cx.roster.by_voice(self.speaker).map(|a| a.id) else { return };
        cx.roster.swap_identity(leader, original);
    }

    fn confirm_with_other_pursuits(&mut self, cx: &mut Cx, leader: u32) {
        let pursuits = cx.snap.pursuit.pursuits;
        if pursuits <= 1 {
            return;
        }
        if pursuits <= 2 {
            cx.suspect_confirmed(leader);
            self.cause = Cause::Reacquired;
            return;
        }
        match cx.random_active(Pick::Primary, false) {
            Some(other) if other != leader => cx.driver_history(other),
            Some(_) => cx.say_dispatch(Line::DriverHistory, Params::default()),
            None => {}
        }
        self.cause = Cause::Reacquired;
    }

    fn player_stopped(&mut self, cx: &mut Cx) {
        if self.pursuit_gone(cx) {
            return;
        }
        if self.busy == 0 {
            let opened = [Line::AttemptVehicleStop, Line::Spotter, Line::SpotterReply, Line::GoAhead]
                .iter()
                .any(|&line| cx.said_or_playing(line));
            let stopped = cx.player_stop_time() > 1.0 || cx.snap.player.speed_mph < 10.0;
            if opened
                && stopped
                && let Some(leader) = cx.roster.leader
            {
                cx.interrupt_composed(leader, Intensity::Normal);
                self.busy += 1;
            }
        } else if cx.player_stop_time() >= cx.tune.min_time_stopped
            && let Some(leader) = cx.roster.leader
        {
            cx.interrupt_composed(leader, Intensity::Normal);
            self.step = Step::Terminal;
            self.busy = 0;
            return;
        }
        if self.busy != 0 {
            return;
        }
        self.step = Step::Transition;
        self.busy = 0;
    }

    fn bailout(&mut self, cx: &mut Cx) {
        if let Some(cop) = cx.random_active(Pick::Any, false) {
            cx.bailout_low(cop);
        }
        self.reset();
    }

    fn change_target(&mut self, cx: &mut Cx) {
        if let Some(closest) = cx.roster.closest(false, true) {
            cx.focus_change(closest);
        }
        cx.disp_pursuit_escalation();
        self.reset();
    }

    /// The leader's last remarks of the opening: "initiate pursuit", the location, "suspect behaviour".
    fn terminal(&mut self, cx: &mut Cx) {
        if self.busy == 0 {
            let leader = cx.roster.leader;
            let fresh = !matches!(
                self.cause,
                Cause::Spotted
                    | Cause::Reported911
                    | Cause::Scripted
                    | Cause::Reacquired
                    | Cause::CopAssaultedScripted
                    | Cause::CopAssaulted
            );
            if let Some(leader) = leader {
                if cx.snap.player.speed_mph >= 60.0 && fresh {
                    cx.initiate_pursuit(leader);
                }
                if !cx.high_intensity() && !self.cause.is_assault() {
                    cx.location_report(leader);
                }
            }
            if cx.core.cost_911 as f32 >= cx.tune.min_havoc
                && !self.cause.is_assault()
                && let Some(seer) = cx.roster.closest(true, true)
            {
                cx.suspect_behaviour(seer);
            }
            self.busy += 1;
            return;
        }
        if !cx.busy() {
            self.step = Step::Transition;
            self.busy = 0;
            self.first_on_scene = None;
        }
    }
}

impl Cx<'_> {
    /// Dispatch's escalation, with the place when a coin and the heading allow, else the generic one.
    pub fn escalate_by_coin(&mut self) {
        match self.rng.coin() && self.heading_valid() {
            true => self.disp_pursuit_escalation(),
            false => self.disp_pursuit_escalation_generic(),
        }
    }
}
