//! The speech machine: the original's `SoundAI` as a pure state machine. Each step reads a [`Snapshot`], the
//! [`Notice`]s since the last step and the [`Channel`], and returns the lines to say and the things to do.

use super::actors::Roster;
use super::config::{Layout, Tune};
use super::cx::Cx;
use super::flows::Flows;
use super::input::{Channel, Notice, Snapshot};
use super::output::Output;
use super::state::{Core, Focus, PursuitState, Rng};
use super::strategy_flow::Step;

/// The main update runs at a tenth of a second, the observations at a quarter (the original's task rates).
const MAIN_PERIOD: f32 = 0.1;
const OBSERVER_PERIOD: f32 = 0.25;

pub struct Machine {
    layout: Layout,
    tune: Tune,
    now: f32,
    main_clock: f32,
    observer_clock: f32,
    core: Core,
    roster: Roster,
    rng: Rng,
    flows: Flows,
}

impl Machine {
    pub fn new(layout: Layout, tune: Tune, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let roster = Roster::new(&layout, &mut rng);
        Self {
            layout,
            tune,
            now: 0.0,
            main_clock: 0.0,
            observer_clock: 0.0,
            core: Core::new(),
            roster,
            rng,
            flows: Flows::default(),
        }
    }

    /// Replaces the thresholds (the game reads them from its data after the machine is made).
    pub fn set_tune(&mut self, tune: Tune) {
        self.tune = tune;
    }

    /// Advances the machine by `dt` seconds. The notices are applied first, in order; the main update and the observation
    /// update run when their periods have passed.
    pub fn step(&mut self, dt: f32, snap: &Snapshot, notices: &[Notice], chan: &dyn Channel) -> Output {
        self.now += dt;
        self.main_clock += dt;
        self.observer_clock += dt;
        let mut out = Output::default();
        let Self { layout, tune, now, core, roster, rng, flows, .. } = self;
        let mut cx = Cx { tune, layout, snap, chan, now: *now, core, roster, rng, out: &mut out };
        for notice in notices {
            cx.apply_notice(flows, *notice);
        }
        if self.main_clock >= MAIN_PERIOD {
            self.main_clock %= MAIN_PERIOD;
            if !cx.core.flags.busted {
                main_update(&mut cx, flows);
            }
        }
        if self.observer_clock >= OBSERVER_PERIOD {
            self.observer_clock %= OBSERVER_PERIOD;
            if !cx.core.flags.busted {
                flows.observer.update(&mut cx);
                flows.roadblock.update(&mut cx);
            }
        }
        out
    }

    /// The speech actors (the cops the machine knows by voice).
    pub fn cops(&self) -> &[super::Actor] {
        &self.roster.actors
    }

    pub fn focus(&self) -> Focus {
        self.core.focus
    }

    pub fn state(&self) -> PursuitState {
        self.core.state
    }

    /// The machine is idle: no pursuit, nothing said, nothing to wait for.
    pub fn is_idle(&self) -> bool {
        self.core.state == PursuitState::Inactive && self.core.focus == Focus::Pursuit && self.roster.actors.is_empty()
    }

    /// A readout for the console: the focus, the state, the flows and the cops.
    pub fn status(&self) -> String {
        let actors: Vec<String> = self
            .roster
            .actors
            .iter()
            .map(|a| {
                let mut flags = String::new();
                flags.push(if a.active { 'a' } else { '-' });
                flags.push(if a.los { 'l' } else { '-' });
                flags.push(if a.in_formation { 'f' } else { '-' });
                let leader = if self.roster.leader == Some(a.id) { "*" } else { "" };
                format!("{leader}voice {} b{}#{} {flags} {:.0} m", a.voice, a.battalion.0, a.call_sign, a.distance)
            })
            .collect();
        format!(
            "focus {:?}, state {:?}, quadrant {:?}; pursuit flow {:?} ({:?}), strategy flow {:?}; {} roadblocks seen; cops: {}",
            self.core.focus,
            self.core.state,
            self.core.quadrant,
            self.flows.pursuit.step,
            self.flows.pursuit.cause,
            self.flows.strategy.step,
            self.flows.roadblock.roadblocks_seen(),
            if actors.is_empty() { "none".to_owned() } else { actors.join("; ") }
        )
    }
}

/// The main update: the pursuit, the cars, the player, the formations, the voices, the flow in focus, and the dead air.
fn main_update(cx: &mut Cx, flows: &mut Flows) {
    cx.sync_pursuit(flows);
    cx.sync_cars();
    cx.sync_player();
    cx.sync_formations();
    cx.shuffle_actors();
    update_flows(cx, flows);
    if cx.chan.dead_air() > 0.0 {
        cx.deal_with_dead_air();
    }
}

/// Runs the flow whose turn it is, and hands over from the opening to the strategy flow.
fn update_flows(cx: &mut Cx, flows: &mut Flows) {
    match cx.core.focus {
        Focus::Pursuit => {
            flows.pursuit.update(cx);
            if flows.pursuit.is_transitionable() {
                cx.core.focus = Focus::Strategy;
                flows.strategy.change_to(Step::CullCheck);
            }
        }
        Focus::Strategy => {
            if flows.strategy.is_transitionable() {
                flows.strategy.change_to(Step::Waiting);
            }
            flows.strategy.update(cx);
        }
        Focus::Lost => {
            if cx.snap.pursuit.exists && cx.state_is(PursuitState::Searching) {
                if flows.strategy.step != Step::Lost {
                    flows.strategy.change_to(Step::Lost);
                }
                flows.strategy.update(cx);
            }
        }
        Focus::Terminal => {}
    }
}
