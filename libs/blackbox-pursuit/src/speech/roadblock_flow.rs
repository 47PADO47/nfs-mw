//! The roadblock chatter (`RoadblockFlow`): the leader calls for a roadblock when the heat's tables allow one,
//! dispatch answers, the cars warn of the roadblock ahead, and the outcome is commented on. Spec:
//! `docs/specs/ai-pursuit-speech.md` section 6.1b. The flow keeps the original's flag set; the notes in the code
//! mark the places where the original's flags make a branch unreachable.

use super::actors::Pick;
use super::cx::Cx;
use super::input::RoadblockHit;
use super::line::Line;
use super::params::{Backup, RoadblockKind};
use super::state::{Focus, PursuitState};

const SETUP: u32 = 1 << 0;
const LOS: u32 = 1 << 1;
const HELI_JOINED: u32 = 1 << 2;
const AVERTED: u32 = 1 << 3;
const NAILED_SPIKES: u32 = 1 << 4;
const NAILED_OTHER: u32 = 1 << 5;
const NAILED_VEHICLE: u32 = 1 << 6;
const ENGAGED: u32 = 1 << 7;
const POSITIONED: u32 = 1 << 8;
const REQUESTED: u32 = 1 << 9;
const RB_ENABLED: u32 = 1 << 10;
const HELI_RB_ENABLED: u32 = 1 << 11;
const SPIKES_ENABLED: u32 = 1 << 12;
const CALLED_FOR_SPIKES: u32 = 1 << 13;
const CALLED_FOR_NORMAL: u32 = 1 << 14;
const RESET_PENDING: u32 = 1 << 17;
const OUTCOME_TIMER_SET: u32 = 1 << 18;
const REQ_SERVICE: u32 = 1 << 28;

/// What the flow forgets when the roadblock changes.
const PER_ROADBLOCK: u32 =
    SETUP | LOS | HELI_JOINED | AVERTED | NAILED_SPIKES | NAILED_OTHER | NAILED_VEHICLE | ENGAGED | POSITIONED;

#[derive(Debug, Clone, Default)]
pub struct RoadblockFlow {
    flags: u32,
    t_engaged: f32,
    t_averted: f32,
    t_reset: f32,
    spike_offset: i32,
    pertinent: Option<u32>,
    blocks: u32,
}

impl RoadblockFlow {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// How many roadblocks the flow has seen come and go.
    pub fn roadblocks_seen(&self) -> u32 {
        self.blocks
    }

    pub fn has_nailed_spikes(&self) -> bool {
        self.flags & NAILED_SPIKES != 0
    }

    /// The player hit something of the roadblock.
    pub fn nailed(&mut self, now: f32, hit: RoadblockHit) {
        if self.flags & OUTCOME_TIMER_SET == 0 && self.flags & (AVERTED | ENGAGED) == 0 {
            self.t_engaged = now;
            self.flags |= OUTCOME_TIMER_SET;
        }
        self.flags |= ENGAGED
            | match hit {
                RoadblockHit::Spikes => NAILED_SPIKES,
                RoadblockHit::Object => NAILED_OTHER,
                RoadblockHit::Vehicle => NAILED_VEHICLE,
            };
    }

    /// The player drove round the roadblock.
    pub fn dodged(&mut self, now: f32) {
        if self.flags & OUTCOME_TIMER_SET == 0 && self.flags & (AVERTED | ENGAGED) == 0 {
            self.t_averted = now;
            self.flags |= OUTCOME_TIMER_SET;
        }
        self.flags |= AVERTED;
    }

    /// The roadblock's cars were placed.
    pub fn positioned(&mut self, offset: i32) {
        self.flags |= POSITIONED | REQ_SERVICE;
        self.spike_offset = offset;
    }

    /// The helicopter joins the roadblock: it says so when it is in the chase.
    pub fn heli_joins(&mut self, cx: &mut Cx) {
        self.flags |= HELI_JOINED;
        let heli = cx.roster.actors.iter().find(|a| a.heli).map(|a| a.id);
        if let (Some(heli), true) = (heli, cx.state_is(PursuitState::Active)) {
            cx.heli_self_strategy(heli, super::params::HeliStrategy::Roadblock);
        }
    }

    /// A line finished: the roadblock's request is served.
    pub fn on_said(&mut self, _line: Line) {
        self.flags &= !REQ_SERVICE;
    }

    pub fn update(&mut self, cx: &mut Cx) {
        self.sync(cx);
        self.service(cx);
    }

    fn sync(&mut self, cx: &mut Cx) {
        let block = cx.snap.roadblock;
        if self.flags & RESET_PENDING != 0 {
            let since = cx.now - self.t_reset;
            let replaced = self.pertinent != block.map(|b| b.id);
            if replaced && since >= cx.tune.rb_post_outcome_reset {
                self.pertinent = block.map(|b| b.id);
                self.flags &= !RESET_PENDING;
                if self.flags & SETUP != 0 {
                    self.flags |= REQ_SERVICE;
                }
                self.flags &= !PER_ROADBLOCK;
                self.blocks += 1;
            }
        }
        if let (Some(block), true) = (block, self.flags & RESET_PENDING == 0) {
            if self.flags & SETUP == 0 {
                self.flags |= SETUP | REQ_SERVICE;
            }
            // The original sets the flag when a car sees the player and clears it on the next look: it toggles.
            match block.visual && self.flags & LOS == 0 {
                true => self.flags |= LOS | REQ_SERVICE,
                false => self.flags &= !LOS,
            }
        }
        if self.settings_changed(cx) {
            self.request(cx);
        }
    }

    fn settings_changed(&self, cx: &Cx) -> bool {
        let level = cx.snap.level;
        let pairs =
            [(RB_ENABLED, level.roadblocks), (HELI_RB_ENABLED, level.heli_roadblocks), (SPIKES_ENABLED, level.spikes)];
        pairs.iter().any(|&(flag, now)| (self.flags & flag != 0) != now)
    }

    /// The leader calls for a roadblock (or is told there is none) and dispatch answers.
    fn request(&mut self, cx: &mut Cx) {
        let level = cx.snap.level;
        let leader = cx.roster.leader;
        if !level.roadblocks && self.flags & (REQUESTED | RB_ENABLED) == 0 {
            let first = !cx.has_been_said(Line::CallForRoadblock) && !cx.chan.queued(Line::CallForRoadblock);
            let busy = cx.chan.is_playing(Line::CallForRoadblock);
            if let (true, false, Some(leader)) = (first, busy, leader) {
                cx.call_for_roadblock(leader, level.spikes);
                cx.disp_roadblock_reply(leader, false, None, false, level.spikes);
                cx.negative_roadblock_reply(leader);
            }
            return;
        }
        let new_roadblocks = level.roadblocks && self.flags & RB_ENABLED == 0;
        let new_spikes = level.spikes && self.flags & (RB_ENABLED | SPIKES_ENABLED | CALLED_FOR_SPIKES) == RB_ENABLED;
        if let (true, Some(leader)) = (new_roadblocks || new_spikes, leader) {
            match cx.has_been_said(Line::CallForRoadblock) {
                false => cx.call_for_roadblock(leader, level.spikes),
                true => cx.roadblock_reminder(leader),
            }
            cx.disp_roadblock_reply(leader, true, None, false, level.spikes);
            self.flags |= RB_ENABLED;
            if level.spikes {
                self.flags |= SPIKES_ENABLED;
            }
        }
        // The helicopter's roadblock needs a request that the flow never records (REQUESTED and the CALLED flags are
        // never set in the original), so this branch is unreachable there as it is here.
        let heli_ready = self.flags & RB_ENABLED != 0
            && level.heli_roadblocks
            && self.flags & HELI_RB_ENABLED == 0
            && self.flags & (CALLED_FOR_SPIKES | CALLED_FOR_NORMAL) != 0
            && self.flags & REQUESTED != 0;
        if let (true, true, Some(leader)) = (level.roadblocks, heli_ready, leader) {
            cx.call_for_backup(leader, Backup::AirSupport);
            cx.disp_roadblock_reply(leader, false, Some(RoadblockKind::HeliDispatchOnly), false, level.spikes);
            self.flags |= HELI_RB_ENABLED;
        }
    }

    fn service(&mut self, cx: &mut Cx) {
        if !cx.focus_is(Focus::Strategy) && !cx.focus_is(Focus::Lost) {
            return;
        }
        let said_strategy = cx.has_been_said(Line::InitiateStrategy) || cx.has_been_said(Line::SelfStrategy);
        if self.flags & REQUESTED == 0 && said_strategy {
            self.request(cx);
        }
        if self.flags & REQ_SERVICE != 0 && self.flags & (AVERTED | ENGAGED) == 0 {
            if self.flags & LOS != 0 {
                self.approach(cx);
            } else if self.flags & SETUP != 0 {
                self.setup(cx);
            }
        }
        if self.flags & OUTCOME_TIMER_SET != 0 {
            self.effect(cx);
        }
    }

    /// A random secondary cop warns that a roadblock is ahead.
    fn approach(&mut self, cx: &mut Cx) {
        let Some(cop) = cx.random_cop(Pick::Secondary) else { return };
        cx.roadblock_approach(cop);
        self.flags &= !REQ_SERVICE;
    }

    /// The roadblock stands: the spike strip's position, the roadblock cop's call, or dispatch and a primary cop
    /// announcing that the pursuit is approaching. (The original's "no roadblock yet" opening branch is unreachable: the
    /// service only runs this once the set-up flag is set.)
    fn setup(&mut self, cx: &mut Cx) {
        if self.flags & POSITIONED != 0 {
            self.announce_position(cx);
            self.flags &= !REQ_SERVICE;
            return;
        }
        let dispatch_talking =
            cx.chan.is_playing(Line::DispRoadblockReply) || cx.chan.is_playing(Line::DispRoadblockUpdate);
        if !cx.rng.coin() || dispatch_talking {
            cx.roadblock_warning();
            self.flags &= !REQ_SERVICE;
            return;
        }
        let Some(primary) = cx.random_active(Pick::Primary, false) else { return };
        cx.disp_roadblock_update(true, false);
        cx.pursuit_approaching(primary);
        self.flags &= !REQ_SERVICE;
    }

    fn announce_position(&mut self, cx: &mut Cx) {
        let strips_one_or_less = cx.snap.roadblock.is_none_or(|b| b.spike_strips <= 1);
        if strips_one_or_less {
            cx.roadblock_position(self.spike_offset);
            return;
        }
        let cop_in_roadblock =
            cx.roster.actors.iter().find(|a| a.active && a.roadblock && !cx.layout.is_primary(a.voice)).map(|a| a.id);
        match cop_in_roadblock.or_else(|| cx.random_cop(Pick::Secondary)) {
            Some(cop) => cx.roadblock_approach(cop),
            None => cx.roadblock_warning(),
        }
    }

    /// After the outcome timer: an engaged roadblock with sight makes a cop call the arrest; the lines about the outcome
    /// follow once the timer is cleared (a dodge, or a dodge and a hit — the original never clears it for a hit alone).
    fn effect(&mut self, cx: &mut Cx) {
        let timer = cx.tune.rb_outcome_timer;
        let engaged_for = cx.now - self.t_engaged;
        let averted_for = cx.now - self.t_averted;
        let both = self.flags & (AVERTED | ENGAGED) == (AVERTED | ENGAGED);
        if both && averted_for >= timer && engaged_for >= timer {
            self.flags &= !OUTCOME_TIMER_SET;
        }
        let dodged_only = self.flags & (AVERTED | ENGAGED) == AVERTED;
        if dodged_only && averted_for >= timer {
            self.flags &= !OUTCOME_TIMER_SET;
        }
        let hit_only = self.flags & (AVERTED | ENGAGED) == ENGAGED;
        if hit_only
            && engaged_for >= timer
            && self.flags & LOS != 0
            && let Some(cop) = cx.random_active(Pick::Any, true)
        {
            cx.arrest(cop);
        }
        if self.flags & OUTCOME_TIMER_SET != 0 {
            return;
        }
        if let Some(block) = cx.snap.roadblock {
            let secondary = cx.random_cop(Pick::Secondary);
            let anyone = cx.random_cop(Pick::Any);
            let wrecked = block.cops_damaged > 0 || block.cops_destroyed > 0 || self.flags & ENGAGED != 0;
            match wrecked {
                true => self.say_engaged(cx, secondary),
                false => self.say_averted(cx, secondary, anyone, block.spike_strips > 0),
            }
            self.flags &= !REQ_SERVICE;
        }
        if self.flags & (AVERTED | ENGAGED) != 0 && self.flags & REQ_SERVICE == 0 {
            self.flags |= RESET_PENDING;
            self.t_reset = cx.now;
        }
    }

    fn say_engaged(&mut self, cx: &mut Cx, secondary: Option<u32>) {
        let spikes = self.has_nailed_spikes();
        match secondary {
            Some(cop) if cx.rng.coin() => cx.roadblock_engage(cop, spikes),
            _ => cx.extra_roadblock_engage(spikes),
        }
    }

    /// The "averted" line; true when dispatch said it (and so needs no second reply).
    fn averted_line(&mut self, cx: &mut Cx, secondary: Option<u32>, spike_strips: bool) -> bool {
        if spike_strips {
            cx.extra_roadblock_averted();
            return false;
        }
        if cx.rng.coin() {
            cx.disp_sub_roadblock_reply();
            return true;
        }
        match secondary {
            Some(cop) if cx.rng.coin() => cx.roadblock_averted(cop),
            _ => cx.extra_roadblock_averted(),
        }
        false
    }

    fn say_averted(&mut self, cx: &mut Cx, secondary: Option<u32>, anyone: Option<u32>, spike_strips: bool) {
        let dispatch_replied = self.averted_line(cx, secondary, spike_strips);
        let primary_caller = anyone.filter(|&id| cx.roster.is_primary(cx.layout, id));
        match (dispatch_replied, primary_caller) {
            (false, Some(cop)) => cx.call_for_sub_roadblock(cop),
            _ => cx.disp_sub_roadblock_reply(),
        }
    }
}
