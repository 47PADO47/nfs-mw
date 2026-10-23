//! The client: clock, prediction and reconciliation of its own car, interpolation of the other player, counters.

use blackbox_net_core::link::{Link, LossyLink, MemoryLink};
use nfsmw_net::clock::{ClockUpdate, LeadControl, NetClock};
use nfsmw_net::input::{InputHistory, MAX_FRAME_TICKS};
use nfsmw_net::interp::{DrawMode, Interpolator};
use nfsmw_net::netstats::NetStats;
use nfsmw_net::predict::{MAX_TICKS_PER_FRAME, PredictedCar, Predictor, ToyCar};
use nfsmw_net::reconcile::{AuthState, Outcome, Reconciler, Tuning};
use nfsmw_net::schema::GameSchema;
use nfsmw_sim::{EntityId, Tick};

use super::server::Server;
use super::wire::{self, Datagram, Snapshot};
use super::{SERVER, my_input};

/// The client's clock runs this far from the simulation's time origin, milliseconds.
const CLIENT_EPOCH_MS: u64 = 3_600_000;

pub struct Client {
    pub link: LossyLink<MemoryLink>,
    pub clock: NetClock,
    lead: Option<LeadControl>,
    pub predictor: Option<Predictor<ToyCar>>,
    pub reconciler: Reconciler,
    history: InputHistory,
    interp: Interpolator,
    pub stats: NetStats,
    schema: GameSchema,
    /// The car the client builds from the loadout, before the first own block places it.
    template: ToyCar,
    acked: Option<Tick>,
    other_entity: EntityId,
    last_frame_ms: Option<u64>,
    /// Remote position errors while interpolating, metres.
    pub remote_errors: Vec<f32>,
    pub extrapolated_frames: u32,
    pub lagging_frames: u32,
    pub frames: u32,
    pub max_lead: u32,
    pub stalls: u32,
    /// Corrections where only the velocity term was over its threshold.
    pub velocity_only: u32,
}

impl Client {
    pub fn new(link: LossyLink<MemoryLink>, schema: GameSchema, template: ToyCar, other: EntityId) -> Self {
        let tuning = Tuning::lan();
        Self {
            link,
            clock: NetClock::new(),
            lead: None,
            predictor: None,
            reconciler: Reconciler::new(tuning),
            history: InputHistory::new(false),
            interp: Interpolator::default(),
            stats: NetStats::new(tuning.snapshot_interval_ticks()),
            schema,
            template,
            acked: None,
            other_entity: other,
            last_frame_ms: None,
            remote_errors: Vec::new(),
            extrapolated_frames: 0,
            lagging_frames: 0,
            frames: 0,
            max_lead: 0,
            stalls: 0,
            velocity_only: 0,
        }
    }

    /// One render frame at simulation time `sim_ms`.
    pub fn frame(&mut self, sim_ms: u64, server: &Server) {
        let now = sim_ms + CLIENT_EPOCH_MS;
        self.receive(now);
        let update = self.clock.update(now);
        let target = self.target_tick();
        if let ClockUpdate::Stepped(_) = update
            && let (Some(predictor), Some(next)) = (self.predictor.as_mut(), target)
        {
            let outcome = self.reconciler.clock_step(predictor, next, now);
            record(&mut self.stats, &outcome, now);
        }
        self.predict(now);
        if let Some(ping) = self.clock.ping_due(now) {
            self.send(&wire::ping(&ping), now);
        }
        let dt_ms = self.last_frame_ms.map_or(0, |last| now - last) as f32;
        self.last_frame_ms = Some(now);
        self.reconciler.advance(dt_ms);
        self.interp.advance(f64::from(dt_ms));
        self.measure_remote(server);
    }

    fn receive(&mut self, now: u64) {
        let mut buf = [0u8; 1500];
        while let Ok(Some((_, len))) = self.link.recv(&mut buf) {
            self.stats.on_bytes_in(len, now);
            match wire::decode(&self.schema, &buf[..len]) {
                Some(Datagram::Pong(pong)) => {
                    if let Some(rtt) = self.clock.on_pong(&pong, now) {
                        self.stats.on_rtt(rtt);
                    }
                }
                Some(Datagram::Snapshot(snapshot)) => self.on_snapshot(&snapshot, now),
                _ => {}
            }
        }
    }

    fn on_snapshot(&mut self, snapshot: &Snapshot, now: u64) {
        self.stats.on_snapshot(snapshot.tick, now);
        let Some(server_tick) = self.clock.server_tick() else {
            return;
        };
        self.interp.on_snapshot(snapshot.tick, server_tick);
        for decoded in &snapshot.entities {
            self.interp.push(snapshot.tick, &decoded.view);
        }
        self.acked = Some(self.acked.map_or(snapshot.ack_input, |acked| acked.max(snapshot.ack_input)));
        let auth = AuthState::from_own_block(snapshot.ack_input, &snapshot.own);
        let Some(predictor) = self.predictor.as_mut() else {
            self.start(&auth, server_tick);
            return;
        };
        let outcome = self.reconciler.apply(predictor, &auth, now);
        if matches!(outcome, Outcome::Corrected { .. }) {
            let (e, t) = (self.reconciler.last_error(), self.reconciler.tuning());
            let velocity_only = e.position < t.position_threshold_m && e.orientation < t.orientation_threshold_deg;
            self.velocity_only += u32::from(velocity_only);
        }
        record(&mut self.stats, &outcome, now);
        if let Some(lead) = self.lead.as_mut() {
            lead.on_report(&snapshot.own.report, now);
            self.max_lead = self.max_lead.max(lead.ticks());
        }
    }

    /// The first own block places the car; prediction starts at the lead.
    fn start(&mut self, auth: &AuthState, server_tick: f64) {
        let rtt = self.clock.best_rtt_ms().unwrap_or(0.0);
        let lead = LeadControl::from_rtt(rtt, self.reconciler.tuning().input_buffer_ticks);
        let mut car = self.template;
        car.set_state(&auth.state);
        let next = (lead.predicted_tick(server_tick).floor() as Tick).max(auth.tick + 1);
        self.max_lead = lead.ticks();
        self.lead = Some(lead);
        self.predictor = Some(Predictor::new(car, next));
    }

    /// The tick to have predicted by now: `floor(S_est + lead)`.
    fn target_tick(&self) -> Option<Tick> {
        let server_tick = self.clock.server_tick()?;
        let lead = self.lead.as_ref()?;
        Some(lead.predicted_tick(server_tick).floor().max(0.0) as Tick)
    }

    fn predict(&mut self, now: u64) {
        let Some(target) = self.target_tick() else {
            return;
        };
        let Some(predictor) = self.predictor.as_mut() else {
            return;
        };
        if target >= predictor.next_tick() + MAX_TICKS_PER_FRAME {
            predictor.skip_to(target + 1 - MAX_TICKS_PER_FRAME);
        }
        let mut stepped = false;
        while predictor.next_tick() <= target {
            let tick = predictor.next_tick();
            let Some(code) = predictor.step(&my_input(tick)) else {
                self.stalls += 1;
                break;
            };
            self.history.record(tick, code);
            stepped = true;
        }
        if !stepped {
            return;
        }
        if let Some(frame) = self.history.frame(self.acked, MAX_FRAME_TICKS) {
            self.send(&wire::input(&frame), now);
        }
    }

    fn measure_remote(&mut self, server: &Server) {
        let Some(server_tick) = self.clock.server_tick() else {
            return;
        };
        let r = self.interp.display_tick(server_tick);
        let Some(drawn) = self.interp.sample(self.other_entity, r) else {
            return;
        };
        self.frames += 1;
        match drawn.mode {
            DrawMode::Interpolated => {}
            DrawMode::Extrapolated => self.extrapolated_frames += 1,
            DrawMode::Lagging => self.lagging_frames += 1,
        }
        if drawn.mode != DrawMode::Interpolated {
            return;
        }
        let Some(truth) = server.truth_at(r) else {
            return;
        };
        let p = drawn.view.position;
        let error = ((p[0] - truth[0]).powi(2) + (p[1] - truth[1]).powi(2) + (p[2] - truth[2]).powi(2)).sqrt();
        self.remote_errors.push(error);
        self.interp.prune(r);
    }

    fn send(&mut self, bytes: &[u8], now: u64) {
        self.stats.on_bytes_out(bytes.len(), now);
        let _ = self.link.send(SERVER, bytes);
    }
}

fn record(stats: &mut NetStats, outcome: &Outcome, now: u64) {
    match outcome {
        Outcome::Corrected { .. } => stats.on_correction(now),
        Outcome::Snapped { .. } => stats.on_snap(now),
        Outcome::Stale | Outcome::Confirmed => {}
    }
}
