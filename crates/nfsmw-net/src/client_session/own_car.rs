//! The owner's car: prediction ahead of the server, reconciliation with the own-car block, the input lead and the
//! input history the INPUT frames are built from (sync-model.md sections 2 and 5.3).

use std::ops::RangeInclusive;

use blackbox_vehicle::InputState;
use nfsmw_sim::Tick;

use crate::clock::LeadControl;
use crate::input::{InputFrame, InputHistory, MAX_FRAME_TICKS, expand_tick};
use crate::predict::{MAX_TICKS_PER_FRAME, Pose, PredictedCar, Predictor};
use crate::reconcile::{AuthState, Outcome, Reconciler, Tuning};
use crate::schema::OwnBlock;

/// The owner's car and its input.
pub struct OwnCar<C: PredictedCar> {
    template: C,
    predictor: Option<Predictor<C>>,
    reconciler: Reconciler,
    lead: Option<LeadControl>,
    history: InputHistory,
    /// The newest input tick the server confirmed.
    acked: Option<Tick>,
    /// Input was recorded since the last INPUT frame.
    unsent: bool,
    /// The race start: input before it is held neutral.
    start_tick: Option<Tick>,
}

impl<C: PredictedCar + Clone> OwnCar<C> {
    /// A car built from the loadout; the first own block places it.
    pub fn new(template: C, tuning: Tuning, uses_clutch: bool) -> Self {
        Self {
            template,
            predictor: None,
            reconciler: Reconciler::new(tuning),
            lead: None,
            history: InputHistory::new(uses_clutch),
            acked: None,
            unsent: false,
            start_tick: None,
        }
    }

    /// Forgets the prediction (a reconnect or a clock the session no longer trusts); the next own block places the
    /// car again.
    pub fn forget(&mut self) {
        self.predictor = None;
        self.lead = None;
        self.acked = None;
        self.unsent = false;
    }

    pub fn set_start_tick(&mut self, tick: Option<Tick>) {
        self.start_tick = tick;
    }

    pub fn predictor(&self) -> Option<&Predictor<C>> {
        self.predictor.as_ref()
    }

    pub fn reconciler(&self) -> &Reconciler {
        &self.reconciler
    }

    pub fn lead_ticks(&self) -> Option<u32> {
        self.lead.as_ref().map(LeadControl::ticks)
    }

    /// The drawn pose: physics plus the smoothed correction offset.
    pub fn pose(&self) -> Option<Pose> {
        Some(self.reconciler.drawn_pose(self.predictor.as_ref()?))
    }

    /// Takes the own block of a snapshot whose `ack_input` low bits are `ack_low`, at server tick estimate
    /// `server_tick` and best round trip `rtt_ms`. The first block places the car and starts prediction at the lead.
    pub fn on_block(
        &mut self,
        snapshot_tick: Tick,
        ack_low: u16,
        block: &OwnBlock,
        server_tick: f64,
        rtt_ms: f64,
        now_ms: u64,
    ) -> Option<Outcome> {
        let ack = expand_tick(ack_low, snapshot_tick);
        self.acked = Some(self.acked.map_or(ack, |acked| acked.max(ack)));
        let auth = AuthState::from_own_block(ack, block);
        let Some(predictor) = self.predictor.as_mut() else {
            let lead = LeadControl::from_rtt(rtt_ms, self.reconciler.tuning().input_buffer_ticks);
            let mut car = self.template.clone();
            car.set_state(&auth.state);
            let next = (lead.predicted_tick(server_tick).floor().max(0.0) as Tick).max(auth.tick + 1);
            self.lead = Some(lead);
            self.predictor = Some(Predictor::new(car, next));
            return None;
        };
        let outcome = self.reconciler.apply(predictor, &auth, now_ms);
        if let Some(lead) = self.lead.as_mut() {
            lead.on_report(&block.report, now_ms);
        }
        Some(outcome)
    }

    /// The clock stepped: the prediction is void.
    pub fn clock_step(&mut self, server_tick: f64, now_ms: u64) -> Option<Outcome> {
        let next = self.target_tick(server_tick)?;
        let predictor = self.predictor.as_mut()?;
        Some(self.reconciler.clock_step(predictor, next, now_ms))
    }

    /// The tick prediction should have reached at `server_tick`: `floor(S_est + lead)`.
    pub fn target_tick(&self, server_tick: f64) -> Option<Tick> {
        Some(self.lead.as_ref()?.predicted_tick(server_tick).floor().max(0.0) as Tick)
    }

    /// The ticks the game should step now, oldest first; after a long stall the oldest are skipped so at most
    /// [`MAX_TICKS_PER_FRAME`] remain (the server's missing-input rule covers the gap).
    pub fn due(&mut self, server_tick: f64) -> Option<RangeInclusive<Tick>> {
        let target = self.target_tick(server_tick)?;
        let predictor = self.predictor.as_mut()?;
        if target >= predictor.next_tick() + MAX_TICKS_PER_FRAME {
            predictor.skip_to(target + 1 - MAX_TICKS_PER_FRAME);
        }
        let next = predictor.next_tick();
        (next <= target).then_some(next..=target)
    }

    /// Steps the car for `tick` with `input` (neutral before the race start). Only the next tick is taken; an earlier
    /// or later one is refused (`false`), as is a step while prediction is stalled.
    pub fn step(&mut self, tick: Tick, input: &InputState) -> bool {
        let Some(predictor) = self.predictor.as_mut() else {
            return false;
        };
        if tick != predictor.next_tick() {
            return false;
        }
        let neutral = InputState::default();
        let input = match self.start_tick.is_some_and(|start| tick < start) {
            true => &neutral,
            false => input,
        };
        let Some(code) = predictor.step(input) else {
            return false;
        };
        self.history.record(tick, code);
        self.unsent = true;
        true
    }

    /// The INPUT frame to send, if inputs were recorded since the last call: the newest unacknowledged ticks.
    pub fn take_frame(&mut self) -> Option<InputFrame> {
        if !self.unsent {
            return None;
        }
        self.unsent = false;
        self.history.frame(self.acked, MAX_FRAME_TICKS)
    }

    /// Advances the correction offset's decay by `dt_ms`.
    pub fn advance(&mut self, dt_ms: f32) {
        self.reconciler.advance(dt_ms);
    }
}
