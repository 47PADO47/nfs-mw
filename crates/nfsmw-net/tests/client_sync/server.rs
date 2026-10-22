//! The authority: a toy world, one input window for the client, snapshots at 20 Hz.

use std::collections::{BTreeMap, VecDeque};

use blackbox_net_core::link::{Link, LossyLink, MemoryLink};
use nfsmw_net::clock::Pong;
use nfsmw_net::input::{InputSource, InputWindow, expand_tick};
use nfsmw_net::schema::{CorrectionCause, GameSchema, InputReport, OwnBlock};
use nfsmw_sim::toy::ToyWorld;
use nfsmw_sim::{EntityView, InputState, SimEvent, SimInputs, Tick, WorldSim};

use super::wire::{self, Datagram};
use super::{CLIENT, ME, OTHER, other_input};

/// Ticks of the input report window (sync-model.md section 5.3).
const REPORT_TICKS: usize = 32;

pub struct Server {
    pub world: ToyWorld,
    pub link: LossyLink<MemoryLink>,
    window: InputWindow,
    schema: GameSchema,
    received_any: bool,
    /// How early each tick's input first arrived, ticks.
    earliness: BTreeMap<Tick, u32>,
    /// The last 32 simulated ticks: how early the input was, or `None` when it was missing.
    outcomes: VecDeque<Option<u32>>,
    /// The other player's position after each tick.
    pub truth: Vec<[f32; 3]>,
    pub contacts: u32,
    /// Ticks the client's input was missing after it first arrived.
    pub missing: u32,
    /// Server time origin, milliseconds.
    phase_ms: f64,
}

impl Server {
    pub fn new(world: ToyWorld, link: LossyLink<MemoryLink>, schema: GameSchema, phase_ms: f64) -> Self {
        Self {
            window: InputWindow::new(world.tick()),
            world,
            link,
            schema,
            received_any: false,
            earliness: BTreeMap::new(),
            outcomes: VecDeque::new(),
            truth: Vec::new(),
            contacts: 0,
            missing: 0,
            phase_ms,
        }
    }

    /// The server's fractional tick at `now_ms`.
    fn tick_at(&self, now_ms: u64) -> f64 {
        (now_ms as f64 - self.phase_ms) * 60.0 / 1000.0
    }

    /// The other player's true position at fractional tick `r`, linear between ticks.
    pub fn truth_at(&self, r: f64) -> Option<[f32; 3]> {
        let low = r.floor();
        if low < 0.0 {
            return None;
        }
        let a = *self.truth.get(low as usize)?;
        let b = *self.truth.get(low as usize + 1)?;
        let u = (r - low) as f32;
        Some([a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u])
    }

    pub fn receive(&mut self, now_ms: u64) {
        let mut buf = [0u8; 1500];
        while let Ok(Some((_, len))) = self.link.recv(&mut buf) {
            match wire::decode(&self.schema, &buf[..len]) {
                Some(Datagram::Input(frame)) => {
                    let next = self.world.tick();
                    let first = expand_tick(frame.first_tick(), next);
                    for offset in 0..frame.len() as u32 {
                        let tick = first.wrapping_add(offset);
                        if tick >= next {
                            self.earliness.entry(tick).or_insert(tick - next);
                        }
                    }
                    self.window.accept(&frame, next);
                }
                Some(Datagram::Ping(ping)) => {
                    let at = self.tick_at(now_ms);
                    let pong = Pong {
                        seq: ping.seq,
                        client_time_ms: ping.client_time_ms,
                        server_recv_tick: at,
                        server_send_tick: at,
                    };
                    self.send(&wire::pong(&pong));
                }
                _ => {}
            }
        }
    }

    /// Simulate one tick and send a snapshot every third.
    pub fn step(&mut self) {
        let tick = self.world.tick();
        let (code, source) = self.window.input_for(tick);
        let received = source == InputSource::Received;
        self.received_any |= received;
        let mut mine = code.to_input();
        if !self.received_any {
            // Before the client's first input the car waits, as it would before `LoadComplete`.
            mine = InputState::default();
        }
        if self.received_any {
            let outcome = self.earliness.remove(&tick).filter(|_| received);
            self.missing += u32::from(outcome.is_none());
            if self.outcomes.len() >= REPORT_TICKS {
                self.outcomes.pop_front();
            }
            self.outcomes.push_back(outcome);
        }
        self.earliness.retain(|t, _| *t > tick);
        let mut inputs = SimInputs::default();
        inputs.set(ME, mine);
        inputs.set(OTHER, other_input(tick));
        let mut events = Vec::new();
        self.world.step(&inputs, &mut events);
        self.contacts += events.iter().filter(|e| matches!(e, SimEvent::Contact { .. })).count() as u32;
        let views = self.views();
        let other = views.iter().find(|v| v.owner == Some(OTHER)).map_or([0.0; 3], |v| v.position);
        self.truth.push(other);
        if tick % 3 == 0 {
            self.snapshot(tick, &views);
        }
    }

    fn views(&self) -> Vec<EntityView> {
        let mut views = Vec::new();
        self.world.entities(&mut views);
        views
    }

    fn snapshot(&mut self, tick: Tick, views: &[EntityView]) {
        let Some(me) = views.iter().find(|v| v.owner == Some(ME)) else {
            return;
        };
        let Some(own) = self.world.own_state(ME) else {
            return;
        };
        let block = OwnBlock::new(me, own, CorrectionCause::None, self.report());
        let others: Vec<EntityView> = views.iter().filter(|v| v.owner != Some(ME)).copied().collect();
        let bytes = wire::snapshot(&self.schema, tick, tick, &block, &others);
        self.send(&bytes);
    }

    /// How the last 32 ticks of input arrived.
    fn report(&self) -> InputReport {
        let received: Vec<u32> = self.outcomes.iter().flatten().copied().collect();
        let missing = self.outcomes.len() - received.len();
        let early = received.iter().filter(|e| **e >= 1).count();
        let mean = match received.len() {
            0 => 0,
            n => (received.iter().sum::<u32>() as f32 / n as f32).round() as u32,
        };
        InputReport {
            early: early as u8,
            on_time: (received.len() - early) as u8,
            late: 0,
            missing: missing as u8,
            earliness: mean.min(15) as u8,
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        let _ = self.link.send(CLIENT, bytes);
    }
}
