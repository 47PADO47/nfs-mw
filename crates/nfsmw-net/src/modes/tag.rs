//! MP-09 tag, a new mode (not in the original). One player is "it"; every tick as "it" adds to that player's timer.
//! Contact with a free player passes the tag; the server decides contact from the cars' samples, not from a client.
//! When the round's time is up the player with the least time as "it" wins.
//!
//! Rules and values, all **[ours]** (features.md gives only "a relative-speed and distance rule"):
//!
//! - **Contact** ([`is_tag`]): the cars' centres within [`TAG_DISTANCE`] (inclusive) and a relative speed of at least
//!   [`TAG_MIN_RELATIVE_SPEED`] (inclusive), so two cars parked against each other do not pass the tag back and forth.
//!   When several players qualify in one tick, the nearest one is tagged, the lower player id on a tie.
//! - **No tag-back:** the new holder cannot tag the previous holder for [`TAG_BACK_TICKS`].
//! - **First holder:** the setup's choice, else the lowest player id.
//! - **Departures:** when the holder leaves, the tag goes to the player with the least time as "it" (the lower id on a
//!   tie). With one player left the round ends (last standing), with none it ends empty.
//! - **Places:** ascending time as "it", the lower player id on a tie; players who left rank last.

use nfsmw_sim::{PlayerId, Tick};

use crate::results::{FLAG_LEFT, Placing, ticks_to_ms};

use super::ModeRules;
use super::event::{ModeEvent, OverReason};
use super::input::{CarSample, ModeInput, length, sub};

/// The largest distance between two car centres that counts as contact, m.
pub const TAG_DISTANCE: f32 = 4.0;
/// The smallest relative speed that counts as contact, m/s.
pub const TAG_MIN_RELATIVE_SPEED: f32 = 1.0;
/// How long the new holder cannot tag the previous holder: three seconds.
pub const TAG_BACK_TICKS: u32 = 3 * nfsmw_sim::TICK_HZ;

/// Whether two cars `distance` metres apart, closing or parting at `relative_speed`, touch for tag.
pub fn is_tag(distance: f32, relative_speed: f32) -> bool {
    distance.is_finite()
        && relative_speed.is_finite()
        && distance <= TAG_DISTANCE
        && relative_speed >= TAG_MIN_RELATIVE_SPEED
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Tagger {
    player: PlayerId,
    it_ticks: u32,
    present: bool,
    sample: Option<(Tick, CarSample)>,
}

/// A tag round.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    /// In player-id order.
    players: Vec<Tagger>,
    holder: PlayerId,
    /// The previous holder and the tick until which the holder cannot tag them.
    no_tag_back: Option<(PlayerId, Tick)>,
    start_tick: Tick,
    end_tick: Tick,
    over: Option<OverReason>,
    police: bool,
}

impl Tag {
    /// A round of `round_ticks` for `players` (sorted, unique, at least one) from `start_tick`. `first` is the first
    /// holder; `None` or a player not in the round picks the lowest id. `police` when pursuits are on.
    pub fn new(
        players: &[PlayerId],
        start_tick: Tick,
        round_ticks: u32,
        first: Option<PlayerId>,
        police: bool,
    ) -> Self {
        let lowest = players.first().copied().unwrap_or_default();
        let holder = first.filter(|player| players.contains(player)).unwrap_or(lowest);
        Self {
            players: players
                .iter()
                .map(|player| Tagger { player: *player, it_ticks: 0, present: true, sample: None })
                .collect(),
            holder,
            no_tag_back: None,
            start_tick,
            end_tick: start_tick.saturating_add(round_ticks),
            over: None,
            police,
        }
    }

    /// Who is "it".
    pub fn holder(&self) -> PlayerId {
        self.holder
    }

    /// The ticks `player` spent as "it".
    pub fn it_ticks(&self, player: PlayerId) -> Option<u32> {
        self.index(player).map(|index| self.players[index].it_ticks)
    }

    fn index(&self, player: PlayerId) -> Option<usize> {
        self.players.binary_search_by_key(&player, |tagger| tagger.player).ok()
    }

    fn present(&self) -> usize {
        self.players.iter().filter(|tagger| tagger.present).count()
    }

    fn end(&mut self, reason: OverReason, out: &mut Vec<ModeEvent>) {
        self.over = Some(reason);
        out.push(ModeEvent::Over { reason });
    }

    fn leave(&mut self, player: PlayerId, out: &mut Vec<ModeEvent>) {
        let index = match self.index(player) {
            Some(index) if self.players[index].present => index,
            _ => return,
        };
        self.players[index].present = false;
        match self.present() {
            0 => return self.end(OverReason::Empty, out),
            1 => return self.end(OverReason::LastStanding, out),
            _ => {}
        }
        if player != self.holder {
            return;
        }
        let next =
            self.players.iter().filter(|tagger| tagger.present).min_by_key(|tagger| (tagger.it_ticks, tagger.player));
        if let Some(next) = next {
            self.holder = next.player;
            self.no_tag_back = None;
            out.push(ModeEvent::TagPassed { from: Some(player), to: self.holder });
        }
    }

    /// The player the holder touches this tick, if any.
    fn touched(&self, tick: Tick) -> Option<PlayerId> {
        let holder = self.players[self.index(self.holder)?].sample.filter(|(at, _)| *at == tick)?.1;
        let mut best: Option<(f32, PlayerId)> = None;
        for tagger in &self.players {
            if !tagger.present || tagger.player == self.holder {
                continue;
            }
            let protected = self.no_tag_back.is_some_and(|(previous, until)| previous == tagger.player && tick < until);
            if protected {
                continue;
            }
            let other = match tagger.sample {
                Some((at, sample)) if at == tick => sample,
                _ => continue,
            };
            let distance = length(sub(holder.position, other.position));
            let relative = length(sub(holder.velocity, other.velocity));
            if !is_tag(distance, relative) {
                continue;
            }
            if best.is_some_and(|(nearest, _)| nearest <= distance) {
                continue;
            }
            best = Some((distance, tagger.player));
        }
        best.map(|(_, player)| player)
    }
}

impl ModeRules for Tag {
    fn begin(&mut self, out: &mut Vec<ModeEvent>) {
        out.push(ModeEvent::TagPassed { from: None, to: self.holder });
    }

    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if tick < self.start_tick || self.over.is_some() {
            return;
        }
        match *input {
            ModeInput::Car(sample) => {
                if let Some(index) = self.index(sample.player)
                    && self.players[index].present
                {
                    self.players[index].sample = Some((tick, sample));
                }
            }
            ModeInput::Left { player } => self.leave(player, out),
            _ => {}
        }
    }

    fn end_tick(&mut self, tick: Tick, out: &mut Vec<ModeEvent>) {
        if tick < self.start_tick || self.over.is_some() {
            return;
        }
        if tick >= self.end_tick {
            return self.end(OverReason::TimeUp, out);
        }
        if let Some(index) = self.index(self.holder) {
            self.players[index].it_ticks = self.players[index].it_ticks.saturating_add(1);
        }
        let to = match self.touched(tick) {
            Some(to) => to,
            None => return,
        };
        let from = self.holder;
        self.holder = to;
        self.no_tag_back = Some((from, tick.saturating_add(TAG_BACK_TICKS)));
        out.push(ModeEvent::TagPassed { from: Some(from), to });
    }

    fn is_over(&self) -> bool {
        self.over.is_some()
    }

    fn placings(&self) -> Vec<Placing> {
        let mut order: Vec<&Tagger> = self.players.iter().collect();
        order.sort_by_key(|tagger| (!tagger.present, tagger.it_ticks, tagger.player));
        order
            .into_iter()
            .enumerate()
            .map(|(rank, tagger)| {
                let flags = match tagger.present {
                    true => 0,
                    false => FLAG_LEFT,
                };
                Placing {
                    player: tagger.player,
                    place: u8::try_from(rank + 1).unwrap_or(u8::MAX),
                    time_ms: Some(ticks_to_ms(tagger.it_ticks)),
                    best_lap_ms: None,
                    points: 0.0,
                    flags,
                }
            })
            .collect()
    }

    fn police(&self) -> bool {
        self.police
    }
}
