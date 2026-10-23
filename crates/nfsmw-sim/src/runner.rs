//! A headless fixed-step runner: feeds scripted input to a [`WorldSim`], collects its events and reports its state hash.
//!
//! It is the harness of the determinism tests and of soak runs. The runner owns the world and a log of `(tick, event)`; an
//! [`InputScript`] fills the [`SimInputs`] for each tick and may look at the world to decide. [`compare_runs`] drives two
//! worlds with one script and checks their hashes every second.

use crate::hash::StateHasher;
use crate::{EntityView, PlayerId, SimEvent, SimInputs, TICK_HZ, Tick, WorldSim};
use blackbox_vehicle::InputState;

/// Produces the input of every player for one tick.
pub trait InputScript<W: WorldSim> {
    /// Fill `out` (cleared by the runner) with the input for the tick `world` is about to run.
    fn fill(&mut self, tick: Tick, world: &W, out: &mut SimInputs);
}

impl<W: WorldSim, F: FnMut(Tick, &W, &mut SimInputs)> InputScript<W> for F {
    fn fill(&mut self, tick: Tick, world: &W, out: &mut SimInputs) {
        self(tick, world, out);
    }
}

/// A script that asks `input(player, tick)` for each of `players`.
pub struct PerPlayer<F> {
    pub players: Vec<PlayerId>,
    pub input: F,
}

impl<F: FnMut(PlayerId, Tick) -> InputState> PerPlayer<F> {
    pub fn new(players: impl IntoIterator<Item = PlayerId>, input: F) -> Self {
        Self { players: players.into_iter().collect(), input }
    }
}

impl<W: WorldSim, F: FnMut(PlayerId, Tick) -> InputState> InputScript<W> for PerPlayer<F> {
    fn fill(&mut self, tick: Tick, _world: &W, out: &mut SimInputs) {
        for &player in &self.players {
            out.set(player, (self.input)(player, tick));
        }
    }
}

/// Pseudo-random but reproducible driving: each player holds a random choice of pedals for half a second, drawn from a
/// stateless hash of `(seed, player, tick / 30)`, so the same script gives the same input in any run and any order.
#[derive(Debug, Clone)]
pub struct WanderScript {
    pub seed: u64,
    pub players: Vec<PlayerId>,
}

impl WanderScript {
    pub fn new(seed: u64, players: impl IntoIterator<Item = PlayerId>) -> Self {
        Self { seed, players: players.into_iter().collect() }
    }

    /// The input `player` gives at `tick`.
    pub fn input(&self, player: PlayerId, tick: Tick) -> InputState {
        let phase = u64::from(tick / 30);
        let mut h = StateHasher::new();
        h.write_u64(self.seed);
        h.write_u8(player.0);
        h.write_u64(phase);
        let bits = h.finish();
        let unit = |shift: u32| ((bits >> shift) & 0xff) as f32 / 255.0;
        InputState {
            throttle: gate(bits & 0x700 != 0, unit(0).max(0.5)),
            brake: gate(bits & 0x1800 == 0, unit(16)),
            steer: unit(32) * 2.0 - 1.0,
            handbrake: gate(bits & 0xe000 == 0, 1.0),
            nos: bits & 0x30000 == 0,
            ..InputState::default()
        }
    }
}

/// `value` when `on`, otherwise 0.
fn gate(on: bool, value: f32) -> f32 {
    if on {
        return value;
    }
    0.0
}

impl<W: WorldSim> InputScript<W> for WanderScript {
    fn fill(&mut self, tick: Tick, _world: &W, out: &mut SimInputs) {
        for &player in &self.players {
            out.set(player, self.input(player, tick));
        }
    }
}

/// Owns a world and steps it at a fixed rate with scripted input.
pub struct Runner<W: WorldSim> {
    world: W,
    inputs: SimInputs,
    scratch: Vec<SimEvent>,
    log: Vec<(Tick, SimEvent)>,
}

impl<W: WorldSim> Runner<W> {
    pub fn new(world: W) -> Self {
        Self { world, inputs: SimInputs::default(), scratch: Vec::new(), log: Vec::new() }
    }

    pub fn world(&self) -> &W {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut W {
        &mut self.world
    }

    pub fn into_world(self) -> W {
        self.world
    }

    /// Every event so far with the tick that produced it, in order.
    pub fn events(&self) -> &[(Tick, SimEvent)] {
        &self.log
    }

    /// Take the event log, leaving it empty.
    pub fn take_events(&mut self) -> Vec<(Tick, SimEvent)> {
        std::mem::take(&mut self.log)
    }

    /// The world's state hash.
    pub fn hash(&self) -> u64 {
        self.world.state_hash()
    }

    /// Every entity the world lists, in the order it lists them.
    pub fn entities(&self) -> Vec<EntityView> {
        let mut out = Vec::new();
        self.world.entities(&mut out);
        out
    }

    /// Run one tick with the script's input. Returns the events of that tick (also kept in the log).
    pub fn step(&mut self, script: &mut impl InputScript<W>) -> &[SimEvent] {
        let tick = self.world.tick();
        self.inputs.clear();
        script.fill(tick, &self.world, &mut self.inputs);
        self.scratch.clear();
        self.world.step(&self.inputs, &mut self.scratch);
        self.log.extend(self.scratch.iter().map(|event| (tick, *event)));
        &self.scratch
    }

    /// Run `ticks` ticks and return the final state hash.
    pub fn run(&mut self, ticks: u32, script: &mut impl InputScript<W>) -> u64 {
        for _ in 0..ticks {
            self.step(script);
        }
        self.hash()
    }

    /// Run `ticks` ticks and record the state hash after each one.
    pub fn run_hashed(&mut self, ticks: u32, script: &mut impl InputScript<W>) -> Vec<u64> {
        let mut hashes = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            self.step(script);
            hashes.push(self.hash());
        }
        hashes
    }

    /// Run `seconds` simulated seconds.
    pub fn run_seconds(&mut self, seconds: u32, script: &mut impl InputScript<W>) -> u64 {
        self.run(seconds * TICK_HZ, script)
    }
}

/// The result of [`compare_runs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    pub ticks: u32,
    /// How many times the hashes were compared.
    pub checks: u32,
    /// The first tick after which the worlds differed in events or (at a check) in hash; `None` when they never did.
    pub first_divergence: Option<Tick>,
    pub hash_a: u64,
    pub hash_b: u64,
}

impl Comparison {
    pub fn is_equal(&self) -> bool {
        self.first_divergence.is_none() && self.hash_a == self.hash_b
    }
}

/// Run two worlds for `ticks` ticks with the same script and compare them: events every tick, state hashes every second and
/// at the end. The script is asked once per tick, about world `a`, and its input goes to both worlds. Stops at the first
/// divergence.
pub fn compare_runs<W: WorldSim>(a: W, b: W, ticks: u32, script: &mut impl InputScript<W>) -> Comparison {
    compare_runs_every(a, b, ticks, TICK_HZ, script)
}

/// [`compare_runs`] with the hash compared every `interval` ticks (at least 1).
pub fn compare_runs_every<W: WorldSim>(
    mut a: W,
    mut b: W,
    ticks: u32,
    interval: u32,
    script: &mut impl InputScript<W>,
) -> Comparison {
    let interval = interval.max(1);
    let mut inputs = SimInputs::default();
    let (mut events_a, mut events_b) = (Vec::new(), Vec::new());
    let mut checks = 0;
    let mut first_divergence = None;
    let mut ran = 0;
    for done in 1..=ticks {
        let tick = a.tick();
        inputs.clear();
        script.fill(tick, &a, &mut inputs);
        events_a.clear();
        events_b.clear();
        a.step(&inputs, &mut events_a);
        b.step(&inputs, &mut events_b);
        ran = done;
        if events_a != events_b {
            first_divergence = Some(tick);
            break;
        }
        if done % interval != 0 && done != ticks {
            continue;
        }
        checks += 1;
        if a.state_hash() != b.state_hash() {
            first_divergence = Some(tick);
            break;
        }
    }
    Comparison { ticks: ran, checks, first_divergence, hash_a: a.state_hash(), hash_b: b.state_hash() }
}
