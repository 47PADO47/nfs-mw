//! The race core shared by circuit, sprint, knockout and speedtrap: per-player laps, best lap, finish time with
//! penalties, and who is out and why. Places come from `blackbox_race::ranking`, which keeps equal racers in their
//! order; racers are kept in player-id order, so of two equal finish times the lower player id is ahead. **[ours]**

use blackbox_race::ranking::{self, Standing};
use nfsmw_sim::{PlayerId, SimEvent, Tick};

use crate::results::{FLAG_BUSTED, FLAG_DNF, FLAG_KNOCKED_OUT, FLAG_LEFT, Placing, ticks_to_ms};

use super::event::{ModeEvent, OverReason};
use super::input::{CarSample, ModeInput};

/// Why a racer stopped racing without a finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutCause {
    Left,
    Busted,
    KnockedOut,
}

/// One racer's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Racer {
    pub player: PlayerId,
    pub laps: u32,
    pub percent: f32,
    pub best_lap_ms: Option<u32>,
    pub penalty_ms: u32,
    /// The race time at the finish, penalties included.
    pub finished_ms: Option<u32>,
    /// Why and at which race time the racer went out.
    pub out: Option<(OutCause, u32)>,
    pub points: f32,
}

impl Racer {
    fn new(player: PlayerId) -> Self {
        Self {
            player,
            laps: 0,
            percent: 0.0,
            best_lap_ms: None,
            penalty_ms: 0,
            finished_ms: None,
            out: None,
            points: 0.0,
        }
    }

    /// Still on the course: neither finished nor out.
    pub fn racing(&self) -> bool {
        self.finished_ms.is_none() && self.out.is_none()
    }

    fn standing(&self) -> Standing {
        Standing {
            finished_at: self.finished_ms.map(|ms| ms as f32),
            out_at: self.out.map(|(_, at)| at as f32),
            laps: self.laps,
            percent: self.percent,
            points: self.points,
        }
    }

    fn flags(&self) -> u8 {
        match self.out {
            None => 0,
            Some((OutCause::Left, _)) => FLAG_LEFT | FLAG_DNF,
            Some((OutCause::Busted, _)) => FLAG_BUSTED | FLAG_DNF,
            Some((OutCause::KnockedOut, _)) => FLAG_KNOCKED_OUT | FLAG_DNF,
        }
    }
}

/// The racers of one race and the rules every race shares.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RaceCore {
    pub start_tick: Tick,
    /// In player-id order.
    pub racers: Vec<Racer>,
    pub finishers: u8,
    pub over: bool,
}

impl RaceCore {
    /// A race of `players` (sorted, unique) starting at `start_tick`.
    pub fn new(players: &[PlayerId], start_tick: Tick) -> Self {
        Self { start_tick, racers: players.iter().copied().map(Racer::new).collect(), finishers: 0, over: false }
    }

    /// The race time at `tick`, ms.
    pub fn race_ms(&self, tick: Tick) -> u32 {
        ticks_to_ms(tick.saturating_sub(self.start_tick))
    }

    /// Whether the core reads inputs at `tick`: the race started and is not over.
    pub fn live(&self, tick: Tick) -> bool {
        !self.over && tick >= self.start_tick
    }

    pub fn racer_mut(&mut self, player: PlayerId) -> Option<&mut Racer> {
        let index = self.racers.binary_search_by_key(&player, |racer| racer.player).ok()?;
        Some(&mut self.racers[index])
    }

    pub fn racer(&self, player: PlayerId) -> Option<&Racer> {
        let index = self.racers.binary_search_by_key(&player, |racer| racer.player).ok()?;
        Some(&self.racers[index])
    }

    /// The rules every race shares: samples, laps, the finish, a bust, a reset penalty and a departure. A busted
    /// racer is out of the race and watches the rest as a spectator. **[ours]**
    pub fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if !self.live(tick) {
            return;
        }
        match *input {
            ModeInput::Car(sample) => self.sample(&sample),
            ModeInput::Sim(SimEvent::LapCompleted { player, lap, time_ms }) => self.lap(player, lap, time_ms, out),
            ModeInput::Sim(SimEvent::RaceFinished { player, time_ms, .. }) => self.finish(player, time_ms, out),
            ModeInput::Sim(SimEvent::Busted { player }) => {
                if self.knock(player, OutCause::Busted, tick) {
                    out.push(ModeEvent::Spectate { player });
                }
            }
            ModeInput::Reset { player, penalty_ms } => self.penalise(player, penalty_ms, out),
            ModeInput::Left { player } => {
                self.knock(player, OutCause::Left, tick);
            }
            _ => {}
        }
    }

    fn sample(&mut self, sample: &CarSample) {
        let racer = match self.racer_mut(sample.player) {
            Some(racer) if racer.racing() => racer,
            _ => return,
        };
        racer.laps = racer.laps.max(sample.laps);
        if sample.percent.is_finite() {
            racer.percent = sample.percent.clamp(0.0, 100.0);
        }
    }

    fn lap(&mut self, player: PlayerId, lap: u8, time_ms: u32, out: &mut Vec<ModeEvent>) {
        let racer = match self.racer_mut(player) {
            Some(racer) if racer.racing() => racer,
            _ => return,
        };
        racer.laps = racer.laps.max(u32::from(lap));
        racer.percent = 0.0;
        let best = racer.best_lap_ms.is_none_or(|best| time_ms < best);
        if best {
            racer.best_lap_ms = Some(time_ms);
        }
        out.push(ModeEvent::LapCompleted { player, lap, time_ms, best });
    }

    /// The racer finishes at `time_ms` plus its penalties.
    pub fn finish(&mut self, player: PlayerId, time_ms: u32, out: &mut Vec<ModeEvent>) {
        let racer = match self.racer_mut(player) {
            Some(racer) if racer.racing() => racer,
            _ => return,
        };
        let total = time_ms.saturating_add(racer.penalty_ms);
        racer.finished_ms = Some(total);
        racer.percent = 100.0;
        self.finishers = self.finishers.saturating_add(1);
        out.push(ModeEvent::Finished { player, place: self.finishers, time_ms: total });
    }

    fn penalise(&mut self, player: PlayerId, ms: u32, out: &mut Vec<ModeEvent>) {
        if ms == 0 {
            return;
        }
        let racer = match self.racer_mut(player) {
            Some(racer) if racer.racing() => racer,
            _ => return,
        };
        racer.penalty_ms = racer.penalty_ms.saturating_add(ms);
        out.push(ModeEvent::Penalty { player, ms });
    }

    /// Takes a racing player out of the race. Returns whether anything changed. A player who already finished keeps
    /// the result after leaving.
    pub fn knock(&mut self, player: PlayerId, cause: OutCause, tick: Tick) -> bool {
        let at = self.race_ms(tick);
        let racer = match self.racer_mut(player) {
            Some(racer) if racer.racing() => racer,
            _ => return false,
        };
        racer.out = Some((cause, at));
        true
    }

    /// How many racers are not out (finished racers count).
    pub fn alive(&self) -> usize {
        self.racers.iter().filter(|racer| racer.out.is_none()).count()
    }

    /// Ends the race once no racer is on the course.
    pub fn check_over(&mut self, out: &mut Vec<ModeEvent>) {
        if self.over || self.racers.iter().any(Racer::racing) {
            return;
        }
        self.over = true;
        out.push(ModeEvent::Over { reason: self.over_reason() });
    }

    fn over_reason(&self) -> OverReason {
        if self.finishers > 0 {
            return OverReason::Finished;
        }
        let busted = self.racers.iter().any(|racer| matches!(racer.out, Some((OutCause::Busted, _))));
        if busted {
            return OverReason::AllBusted;
        }
        OverReason::Empty
    }

    /// The racers in place order. With `by_points` (a speedtrap), racers still in rank by points first, and racers
    /// who are out rank behind every racer who is not. **[ours]**
    pub fn placings(&self, by_points: bool) -> Vec<Placing> {
        let standings: Vec<Standing> = self.racers.iter().map(Racer::standing).collect();
        let mut order = ranking::order(&standings, by_points);
        if by_points {
            order.sort_by_key(|&index| self.racers[index].out.is_some());
        }
        order
            .into_iter()
            .enumerate()
            .map(|(rank, index)| {
                let racer = &self.racers[index];
                Placing {
                    player: racer.player,
                    place: u8::try_from(rank + 1).unwrap_or(u8::MAX),
                    time_ms: racer.finished_ms,
                    best_lap_ms: racer.best_lap_ms,
                    points: racer.points,
                    flags: racer.flags(),
                }
            })
            .collect()
    }
}
