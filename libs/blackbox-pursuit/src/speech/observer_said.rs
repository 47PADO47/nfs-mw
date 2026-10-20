//! What the observer learns when a line finishes: outcome tracking, the 911 follow-ups, the lost mark (see
//! [`observer`](super::observer)).

use super::cx::Cx;
use super::line::{Line, Speaker};
use super::observer::{CAR_RAM, LOST, OUTCOME, Observer};
use super::output::Effect;
use super::params::Params;
use super::state::PursuitState;

impl Observer {
    /// A line finished: what the observer learns from it (outcome tracking, the 911 follow-ups, the lost mark).
    pub fn on_said(&mut self, cx: &mut Cx, line: Line, speaker: Speaker, params: &Params, nailed_spikes: bool) {
        let mut outcome = false;
        match line {
            Line::OffroadMoment => {
                if let Some((place, _)) = params.offroad {
                    self.offroad_history |= 1 << place;
                }
            }
            Line::CollisionWorld => {
                outcome = true;
            }
            Line::CollWorldFlip | Line::BullhornArrest | Line::Arrest | Line::StrategyExecute => {
                outcome = true;
            }
            Line::RoadblockEngage | Line::ExtraRoadblockEngage | Line::HeliBullhornArrest => outcome = !nailed_spikes,
            Line::IntentToRam => outcome = self.intent_to_ram(cx, speaker, params),
            Line::LostVisual | Line::LostSuspect | Line::HeliLostVisual => self.tracking |= LOST,
            Line::ReinitiatePursuit => {
                self.tracking &= !LOST;
                cx.effect(Effect::Recall(Line::RegainVisual));
            }
            Line::Disp911Report => {
                self.voice_911 = cx.more_details();
                cx.core.flags.disp911_active = cx.snap.level.cops_enabled;
                if cx.snap.level.cops_enabled {
                    cx.effect(Effect::Force911 { secs: cx.snap.level.lifetime_911 });
                }
            }
            Line::MoreDetails => self.after_more_details(cx),
            _ => {}
        }
        if outcome {
            self.tracking |= OUTCOME;
            self.t_outcome = cx.now;
        }
    }

    fn intent_to_ram(&mut self, cx: &mut Cx, speaker: Speaker, _params: &Params) -> bool {
        if speaker == cx.layout.heli {
            self.tracking &= !CAR_RAM;
            return true;
        }
        self.tracking |= CAR_RAM;
        self.ram_cop = cx.roster.by_voice(speaker).map(|a| a.voice);
        if self.ram_cop.is_some() {
            cx.core.flags.cops_immune = true;
        }
        true
    }

    /// The "more details" of a 911 call: a description of the car (half the time) or none, and a cop's reply.
    fn after_more_details(&mut self, cx: &mut Cx) {
        if cx.state_is(PursuitState::Active) {
            return;
        }
        let describable = cx.snap.player.car.is_some_and(|c| c.colour != 0) && cx.snap.player.commentable;
        match describable && cx.rng.coin() {
            true => cx.disp_vehicle_description(),
            false => cx.disp_no_vehicle_description(),
        }
        if self.voice_911 > 0 {
            cx.unit_911_reply(self.voice_911);
        }
    }
}
