//! The predicted car and its ring of unacknowledged ticks.

use std::collections::VecDeque;

use blackbox_vehicle::InputState;
use nfsmw_sim::Tick;

use super::{CarState, PredictedCar};
use crate::input::InputCode;

/// Ticks the ring holds: about 2 s at 60 Hz (sync-model.md section 9). A window this long without an acknowledgement
/// stops prediction until the server answers.
pub const RING_TICKS: usize = 128;

/// Most ticks the client simulates in one frame (sync-model.md section 8, client frame stall). Later ticks are dropped
/// and the server's missing-input rule covers them.
pub const MAX_TICKS_PER_FRAME: u32 = 6;

/// One predicted tick: the input applied at `tick` and the car after stepping it.
#[derive(Debug, Clone)]
pub struct Entry<S> {
    pub tick: Tick,
    /// The quantised input, exactly as it was sent.
    pub input: InputCode,
    /// The reduced state after the step.
    pub state: CarState,
    /// The whole car after the step, for a rewind to this tick.
    pub saved: S,
}

/// The owner's car, run ahead of the server, with the ticks the server has not yet confirmed.
#[derive(Debug, Clone)]
pub struct Predictor<C: PredictedCar> {
    car: C,
    /// Entries in ascending tick order. Ticks dropped by a stall leave gaps.
    ring: VecDeque<Entry<C::Saved>>,
    capacity: usize,
    /// The tick the next [`Predictor::step`] predicts.
    next: Tick,
    /// The newest tick the server confirmed (`ack_input`).
    acked: Option<Tick>,
    /// The server froze the car (a bust, a countdown): inputs are held neutral.
    frozen: bool,
}

impl<C: PredictedCar> Predictor<C> {
    /// A predictor whose first predicted tick is `next`, with a ring of [`RING_TICKS`].
    pub fn new(car: C, next: Tick) -> Self {
        Self::with_capacity(car, next, RING_TICKS)
    }

    /// A predictor with a ring of `capacity` ticks (at least 1).
    pub fn with_capacity(car: C, next: Tick, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { car, ring: VecDeque::with_capacity(capacity), capacity, next, acked: None, frozen: false }
    }

    /// The predicted car, at the newest predicted tick.
    pub fn car(&self) -> &C {
        &self.car
    }

    /// The predicted car, mutably. Changing it outside a step is not recorded in the ring.
    pub fn car_mut(&mut self) -> &mut C {
        &mut self.car
    }

    /// The tick the next step predicts (`C + 1` once tick `C` is predicted).
    pub fn next_tick(&self) -> Tick {
        self.next
    }

    /// The newest tick the server confirmed.
    pub fn acked(&self) -> Option<Tick> {
        self.acked
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Hold inputs neutral (the server froze the car) or release them.
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Ticks predicted but not confirmed.
    pub fn unacknowledged(&self) -> usize {
        self.ring.len()
    }

    /// Whether the ring is full: the client stops predicting and waits (the "connection problem" marker).
    pub fn is_stalled(&self) -> bool {
        self.ring.len() >= self.capacity
    }

    /// The entries of the unacknowledged window, oldest first.
    pub fn entries(&self) -> impl Iterator<Item = &Entry<C::Saved>> {
        self.ring.iter()
    }

    /// The entry of `tick`, if it is in the ring.
    pub fn entry(&self, tick: Tick) -> Option<&Entry<C::Saved>> {
        let index = self.ring.binary_search_by_key(&tick, |entry| entry.tick).ok()?;
        self.ring.get(index)
    }

    /// The oldest tick in the ring.
    pub fn oldest(&self) -> Option<Tick> {
        self.ring.front().map(|entry| entry.tick)
    }

    /// Predict the next tick: quantise `input` to wire precision (neutral while frozen), step the car with it and
    /// record the tick. Returns the code to send, or `None` when the ring is full and prediction waits.
    pub fn step(&mut self, input: &InputState) -> Option<InputCode> {
        if self.is_stalled() {
            return None;
        }
        let code = match self.frozen {
            true => InputCode::NEUTRAL,
            false => InputCode::quantise(input),
        };
        self.car.step(&code.to_input());
        let entry = Entry { tick: self.next, input: code, state: self.car.state(), saved: self.car.save() };
        self.ring.push_back(entry);
        self.next = self.next.wrapping_add(1);
        Some(code)
    }

    /// Jump the next predicted tick forward to `tick` without stepping (dropped ticks after a stall). Never moves back.
    pub fn skip_to(&mut self, tick: Tick) {
        self.next = self.next.max(tick);
    }

    /// Drop the entries before `tick`: the server confirmed them.
    pub fn confirm(&mut self, tick: Tick) {
        while self.ring.front().is_some_and(|entry| entry.tick < tick) {
            self.ring.pop_front();
        }
        if self.acked.is_none_or(|acked| tick > acked) {
            self.acked = Some(tick);
        }
    }

    /// Forget every prediction and continue from `next` (a clock step or a respawn).
    pub fn clear(&mut self, next: Tick) {
        self.ring.clear();
        self.next = next;
    }

    /// Rewind to the server's state at `tick` and replay the unacknowledged inputs after it (sync-model.md section 2.3
    /// steps 4 and 5). The car is restored from the ring entry of `tick` when there is one, its reduced fields set to
    /// `state`, and every later tick up to the newest is stepped again and rewritten. A tick dropped by a stall is
    /// replayed with the input before it, shift edges cleared, as the server holds it.
    ///
    /// A `tick` past the newest predicted tick puts the car at `state`, empties the ring and continues after it.
    pub fn rewind_replay(&mut self, tick: Tick, state: &CarState) {
        if tick >= self.next {
            self.car.set_state(state);
            self.ring.clear();
            self.next = self.next.max(tick.wrapping_add(1));
            return;
        }
        let start = self.ring.partition_point(|entry| entry.tick < tick);
        if let Some(entry) = self.ring.get(start).filter(|entry| entry.tick == tick) {
            self.car.restore(&entry.saved);
        }
        self.car.set_state(state);
        let mut rest = start;
        if let Some(entry) = self.ring.get_mut(start).filter(|entry| entry.tick == tick) {
            entry.state = self.car.state();
            entry.saved = self.car.save();
            rest += 1;
        }
        let mut held = self.ring.get(rest.wrapping_sub(1)).map_or(InputCode::NEUTRAL, |entry| entry.input);
        let mut t = tick.wrapping_add(1);
        let mut index = rest;
        while t < self.next {
            let recorded = self.ring.get(index).filter(|entry| entry.tick == t).map(|entry| entry.input);
            let Some(code) = recorded else {
                let input = InputCode { shift_up: false, shift_down: false, ..held };
                self.car.step(&input.to_input());
                t = t.wrapping_add(1);
                continue;
            };
            self.car.step(&code.to_input());
            let state = self.car.state();
            let saved = self.car.save();
            if let Some(entry) = self.ring.get_mut(index) {
                entry.state = state;
                entry.saved = saved;
            }
            held = code;
            index += 1;
            t = t.wrapping_add(1);
        }
    }
}
