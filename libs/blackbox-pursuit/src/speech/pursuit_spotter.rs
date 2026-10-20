//! The branches of the opening that start from a spotter, a 911 call or a script (see [`pursuit_flow`](super::pursuit_flow)).

use super::cx::Cx;
use super::pursuit_flow::{Cause, PursuitFlow, Step};
use super::state::PursuitState;
use crate::infraction::Infraction;

impl PursuitFlow {
    pub(super) fn spotter_branch(&mut self, cx: &mut Cx) {
        if self.pursuit_gone(cx) {
            return;
        }
        if self.busy == 0 {
            self.say_spotter(cx);
            return;
        }
        if cx.player_stop_time() >= cx.tune.min_time_stopped && !self.cause.is_assault() {
            self.busy = 0;
            self.step = Step::PlayerStopped;
            return;
        }
        if cx.busy() {
            return;
        }
        match cx.core.los_count > 0 {
            true => self.step = Step::WaitForSpotter,
            false => {
                if let Some(first) = self.first_on_scene {
                    cx.lost_visual(first);
                }
                self.step = Step::LostWhileSpotterWait;
            }
        }
        self.busy = 0;
    }

    /// The first cop on scene says "spotter" (the wanted variant for a hot chase), perhaps with the car's description;
    /// dispatch goes ahead or acknowledges, and another cop replies.
    fn say_spotter(&mut self, cx: &mut Cx) {
        let wanted = cx.high_intensity() || cx.core.infraction == Some(Infraction::ResistingArrest);
        let Some(first) = cx.roster.closest(false, wanted) else { return };
        self.first_on_scene = Some(first);
        match wanted {
            true => cx.spotter_wanted(first),
            false => cx.spotter(first),
        }
        if cx.rng.coin() {
            cx.vehicle_report(first);
        }
        match cx.rng.coin() {
            true => cx.disp_go_ahead(),
            false => cx.disp_ack(),
        }
        let another = cx.roster.closest(true, true);
        let first_voice = cx.roster.find(first).map(|a| a.voice);
        if let Some(other) = another
            && cx.roster.find(other).map(|a| a.voice) != first_voice
        {
            cx.reply_911(other);
        }
        self.busy += 1;
    }

    pub(super) fn scripted_branch(&mut self, cx: &mut Cx) {
        if self.busy == 0 {
            match cx.heading_valid() {
                true => cx.disp_pursuit_escalation(),
                false => cx.disp_pursuit_escalation_generic(),
            }
            if cx.snap.player.car.is_some_and(|c| c.colour > 0) && cx.snap.player.commentable {
                cx.disp_vehicle_description();
            }
            self.busy += 1;
            return;
        }
        if !cx.busy() {
            self.step = Step::Terminal;
            self.busy = 0;
        }
    }

    pub(super) fn lost_while_spotter_wait(&mut self, cx: &mut Cx) {
        if self.busy == 0 {
            if self.pursuit_gone(cx) {
                return;
            }
            if cx.core.los_count > 0 {
                self.step = Step::WaitForSpotter;
                self.busy = 0;
                return;
            }
        }
        if cx.busy() {
            return;
        }
        if self.busy != 0 {
            self.reset();
            self.step = Step::CullCheck;
            self.busy = 0;
        }
    }

    pub(super) fn spotter_wait(&mut self, cx: &mut Cx) {
        if self.busy == 0 {
            if !cx.state_is(PursuitState::Active) {
                self.step = Step::LostWhileSpotterWait;
                return;
            }
            if let Some(seer) = cx.roster.closest(true, true)
                && cx.make_leader(seer)
                && self.cause != Cause::Scripted
            {
                self.announce_spotter(cx, seer);
            }
            self.busy += 1;
            return;
        }
        if cx.player_stop_time() >= cx.tune.min_time_stopped && !self.cause.is_assault() {
            self.busy = 0;
            self.step = Step::PlayerStopped;
            return;
        }
        if !cx.busy() {
            self.step = Step::Terminal;
            self.busy = 0;
        }
    }

    /// The cop that sees the player confirms (spotted or spotter reply) when it is not the first on scene or the
    /// chase came from a 911 call; dispatch then escalates, unless it was a 911 chase.
    fn announce_spotter(&mut self, cx: &mut Cx, seer: u32) {
        let seer_voice = cx.roster.find(seer).map(|a| a.voice);
        let first_voice = self.first_on_scene.and_then(|f| cx.roster.find(f)).map(|a| a.voice);
        let confirm = self.cause == Cause::Reported911 || (first_voice.is_some() && first_voice != seer_voice);
        if confirm {
            let ahead_or_secondary = cx.core.flags.cops_ahead || !cx.roster.is_primary(cx.layout, seer);
            match ahead_or_secondary {
                true => cx.spotted(seer),
                false => cx.spotter_reply(seer),
            }
        }
        if self.cause != Cause::Reported911 {
            cx.escalate_by_coin();
        }
    }
}
