//! End to end: a toy world as the authority and a client predicting its own car and interpolating another player,
//! through the lossy link simulator, checked against the targets of `docs/specs/multiplayer/features.md` section 4.
//!
//! Every run is deterministic: the simulated time, the link's random stream and both cars' inputs come from the seed and
//! the tick. Run with `--nocapture` to see the measured figures.

mod client;
mod server;
mod wire;

use std::net::SocketAddr;

use blackbox_net_core::link::{LossyLink, MemoryNet};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::predict::ToyCar;
use nfsmw_net::schema::GameSchema;
use nfsmw_sim::toy::{Car, ToyConfig, ToyWorld};
use nfsmw_sim::{InputState, PlayerId, PlayerJoin, Tick, WorldSim};

use client::Client;
use server::Server;

pub const ME: PlayerId = PlayerId(1);
pub const OTHER: PlayerId = PlayerId(2);
pub const SERVER: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)), 7000);
pub const CLIENT: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)), 7001);

/// The client's driving: varying throttle, mostly right-hand loops of changing radius, brake taps, nitrous bursts and
/// the odd handbrake.
pub fn my_input(tick: Tick) -> InputState {
    let t = tick as f32 / 60.0;
    let braking = (0.37 * t).sin() > 0.93;
    let mut input = InputState {
        throttle: 0.75 + 0.25 * (1.3 * t).sin(),
        steer: 0.55 + 0.35 * (0.7 * t).sin(),
        nos: (0.23 * t).sin() > 0.85,
        handbrake: f32::from(u8::from((0.51 * t).sin() > 0.97)),
        ..InputState::default()
    };
    if braking {
        input.throttle = 0.0;
        input.brake = 0.8;
    }
    input
}

/// The other player: slow tight circles far from the client's car.
pub fn other_input(tick: Tick) -> InputState {
    let t = tick as f32 / 60.0;
    InputState { throttle: 0.2, steer: 0.8 + 0.15 * (0.5 * t).sin(), ..InputState::default() }
}

#[derive(Debug, Clone, Copy)]
pub struct Conditions {
    pub loss_percent: f32,
    pub rtt_ms: u32,
    pub jitter_ms: u32,
    /// Relative error of the client's car model (a different build or platform); 0 for the same build.
    pub drift: f32,
    pub seconds: u64,
    pub seed: u64,
}

impl Conditions {
    fn new(loss_percent: f32, rtt_ms: u32, jitter_ms: u32, seconds: u64) -> Self {
        Self { loss_percent, rtt_ms, jitter_ms, drift: 0.0, seconds, seed: 0x5EED }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub minutes: f64,
    pub checks: u64,
    pub corrections: u64,
    pub snaps: u64,
    pub median_m: f32,
    pub p95_m: f32,
    pub largest_m: f32,
    pub remote_median_m: f32,
    pub remote_p95_m: f32,
    pub remote_max_m: f32,
    pub remote_frames: u32,
    pub extrapolated_frames: u32,
    pub lagging_frames: u32,
    pub contacts: u32,
    pub missing_inputs: u32,
    pub max_lead: u32,
    pub stalls: u32,
    pub velocity_only: u32,
    pub final_car: Option<Car>,
    pub line: String,
}

impl Report {
    pub fn snaps_per_minute(&self) -> f64 {
        self.snaps as f64 / self.minutes
    }

    fn print(&self, label: &str) {
        eprintln!(
            "{label}: corrections {} of {} checks, snaps {} ({:.2}/min), median {:.4} m, p95 {:.4} m, largest {:.3} m; \
             remote median {:.4} m, p95 {:.4} m, max {:.3} m, extrapolated {} lagging {} of {} frames; \
             missing inputs {}, max lead {}, stalls {}, velocity-only corrections {}, contacts {}\n    {}",
            self.corrections,
            self.checks,
            self.snaps,
            self.snaps_per_minute(),
            self.median_m,
            self.p95_m,
            self.largest_m,
            self.remote_median_m,
            self.remote_p95_m,
            self.remote_max_m,
            self.extrapolated_frames,
            self.lagging_frames,
            self.remote_frames,
            self.missing_inputs,
            self.max_lead,
            self.stalls,
            self.velocity_only,
            self.contacts,
            self.line,
        );
    }
}

fn percentile(values: &[f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

pub fn run(c: &Conditions) -> Report {
    let schema = GameSchema::with_half_extent(1024.0).unwrap();
    let mut world = ToyWorld::new(c.seed, ToyConfig::bare());
    world.add_player(ME, &PlayerJoin::default()).unwrap();
    world.add_player(OTHER, &PlayerJoin::default()).unwrap();
    world.teleport(OTHER, [350.0, -350.0], [1.0, 0.0]);
    let mut template = ToyCar::new(*world.player_car(ME).unwrap(), world.config().bounds);
    template.car.power *= 1.0 + c.drift;
    let other_entity = world.entity_of(OTHER).unwrap();
    let net = MemoryNet::new();
    let pair = NetSim::new(c.loss_percent, c.rtt_ms, c.jitter_ms).unwrap().link_configs(c.seed);
    let client_link = LossyLink::new(net.endpoint(CLIENT), pair.send);
    let server_link = LossyLink::new(net.endpoint(SERVER), pair.receive);
    let mut server = Server::new(world, server_link, schema.clone(), 3.0);
    let mut client = Client::new(client_link, schema, template, other_entity);
    let tick_ms = 1000.0 / 60.0;
    let (mut ticks, mut frames) = (0u64, 0u64);
    let total_ms = c.seconds * 1000;
    for now in 0..total_ms {
        server.link.set_now(now);
        client.link.set_now(now);
        if now as f64 >= 3.0 + ticks as f64 * tick_ms {
            server.receive(now);
            server.step();
            ticks += 1;
        }
        if now as f64 >= 11.0 + frames as f64 * tick_ms {
            client.frame(now, &server);
            frames += 1;
        }
    }
    let stats = client.reconciler.stats();
    Report {
        minutes: c.seconds as f64 / 60.0,
        checks: stats.checks(),
        corrections: stats.corrections(),
        snaps: stats.snaps(),
        median_m: stats.median().unwrap_or(0.0),
        p95_m: stats.p95().unwrap_or(0.0),
        largest_m: stats.largest(),
        remote_median_m: percentile(&client.remote_errors, 0.5),
        remote_p95_m: percentile(&client.remote_errors, 0.95),
        remote_max_m: percentile(&client.remote_errors, 1.0),
        remote_frames: client.frames,
        extrapolated_frames: client.extrapolated_frames,
        lagging_frames: client.lagging_frames,
        contacts: server.contacts,
        missing_inputs: server.missing,
        max_lead: client.max_lead,
        stalls: client.stalls,
        velocity_only: client.velocity_only,
        final_car: client.predictor.as_ref().map(|p| p.car().car),
        line: client.stats.line(total_ms + 3_600_000).to_string(),
    }
}

/// The LAN targets of features.md section 4: own-car correction median under 5 cm and p95 under 30 cm, fewer than one
/// hard snap a minute without contacts.
fn assert_lan_targets(label: &str, report: &Report) {
    report.print(label);
    assert!(report.checks > 0, "{label}: the client never reconciled");
    assert_eq!(report.contacts, 0, "{label}: the run must be free of contacts");
    assert!(report.median_m < 0.05, "{label}: median correction {} m", report.median_m);
    assert!(report.p95_m < 0.30, "{label}: p95 correction {} m", report.p95_m);
    assert!(report.snaps_per_minute() < 1.0, "{label}: {} snaps per minute", report.snaps_per_minute());
}

#[test]
fn lan_free_roam_meets_the_targets() {
    let report = run(&Conditions::new(2.0, 20, 4, 120));
    assert_lan_targets("lan 2% 20 ms", &report);
    assert!(report.remote_frames > 6000, "the other car was drawn");
    assert!(report.remote_p95_m < 0.05, "remote p95 {} m", report.remote_p95_m);
    assert!(report.remote_max_m < 0.25, "remote max {} m", report.remote_max_m);
    assert!(report.line.starts_with("net rtt "), "{}", report.line);
}

#[test]
fn lossy_links_up_to_150_ms_meet_the_correction_targets() {
    for loss in [2.0, 5.0] {
        for rtt in [20, 50, 100, 150] {
            let report = run(&Conditions::new(loss, rtt, rtt / 5, 60));
            assert_lan_targets(&format!("{loss}% {rtt} ms"), &report);
            assert!(report.remote_p95_m < 0.10, "remote p95 {} m", report.remote_p95_m);
            assert_eq!(report.stalls, 0);
        }
    }
}

/// A client whose car model is 0.2 % off the server's (another build or platform) is corrected often but smoothly.
#[test]
fn a_different_build_is_corrected_without_snaps() {
    let conditions = Conditions { drift: 0.002, ..Conditions::new(2.0, 20, 4, 60) };
    let report = run(&conditions);
    assert_lan_targets("other build", &report);
    assert!(report.corrections > 0, "the drift is corrected");
}

#[test]
fn the_whole_run_is_deterministic() {
    let conditions = Conditions { drift: 0.002, ..Conditions::new(5.0, 100, 20, 20) };
    assert_eq!(run(&conditions), run(&conditions));
}
