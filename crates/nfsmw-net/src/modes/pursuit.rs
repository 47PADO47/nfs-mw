//! One pursuit with several targets (ADR 0010 decision 4, MP-07 as amended), over `blackbox_pursuit::Targets`:
//!
//! - **Shared:** the heat. The chase starts at the higher carried heat of the cars involved (at least 1) and a car that
//!   joins later raises it to its own carried heat (announced in [`ModeEvent::TargetAdded`]); the world reports
//!   changes ([`ModeInput::Heat`]); every value is capped at the session's cop level cap.
//! - **Per target:** busted. A bust ends that car's chase only.
//! - **Escape:** the cool-down is shared, so an evade the world reports ends the chase for every target still chased.
//! - **End:** the pursuit is over when no target is still chased: evaded when the team got away, else busted when a
//!   target was busted, else abandoned (its targets left the session). A departure, or a chase the world drops without
//!   a bust or an escape, ends that target's chase as abandoned. A car whose chase ended does not rejoin the same
//!   pursuit; the next pursuit can take it again. **[ours]**
//!
//! The world's own pursuit rules (sighting, the busted meter, the cool-down) run in the world; this tracker turns
//! their events into one pursuit's events for clients. Pursuit ids count from 1 per session.

use blackbox_pursuit::{TargetId, TargetStatus, Targets};
use nfsmw_sim::{PlayerId, SimEvent};

use crate::message::PursuitEndReason;

use super::event::ModeEvent;
use super::input::ModeInput;

#[derive(Debug, Clone, PartialEq)]
struct Chase {
    id: u16,
    targets: Targets,
    heat: u8,
    evaded: bool,
}

/// The session's pursuit, if one is running.
#[derive(Debug, Clone, PartialEq)]
pub struct PursuitTracker {
    cap: u8,
    carried: Vec<(PlayerId, u8)>,
    last_id: u16,
    chase: Option<Chase>,
}

fn target(player: PlayerId) -> TargetId {
    TargetId(player.0)
}

fn chased_players(targets: &Targets) -> Vec<PlayerId> {
    targets.chased().map(|id| PlayerId(id.0)).collect()
}

impl PursuitTracker {
    /// No pursuit yet. `cap` is the session's cop level cap; `carried` the heat each player brings.
    pub fn new(cap: u8, carried: &[(PlayerId, u8)]) -> Self {
        Self { cap, carried: carried.to_vec(), last_id: 0, chase: None }
    }

    /// The running pursuit's id.
    pub fn current(&self) -> Option<u16> {
        self.chase.as_ref().map(|chase| chase.id)
    }

    /// The shared heat of the running pursuit.
    pub fn heat(&self) -> Option<u8> {
        self.chase.as_ref().map(|chase| chase.heat)
    }

    /// Where `player` stands in the running pursuit.
    pub fn status(&self, player: PlayerId) -> Option<TargetStatus> {
        self.chase.as_ref()?.targets.status(target(player))
    }

    fn carried_heat(&self, player: PlayerId) -> u8 {
        let carried = self.carried.iter().filter(|(id, _)| *id == player).map(|(_, level)| *level).max();
        carried.unwrap_or(0).max(1).min(self.cap.max(1))
    }

    /// Reads one input. Returns how the pursuit ended when this input ended it.
    pub fn feed(&mut self, input: &ModeInput, out: &mut Vec<ModeEvent>) -> Option<PursuitEndReason> {
        match *input {
            ModeInput::Sim(SimEvent::PursuitStarted { player }) => {
                self.start(player, out);
                None
            }
            ModeInput::Sim(SimEvent::Busted { player }) => self.end_target(player, PursuitEndReason::Busted, out),
            ModeInput::Sim(SimEvent::Evaded { .. }) => self.evade(out),
            ModeInput::Sim(SimEvent::PursuitEnded { player }) | ModeInput::Left { player } => {
                self.end_target(player, PursuitEndReason::Abandoned, out)
            }
            ModeInput::Heat { level } => {
                self.set_heat(level, out);
                None
            }
            _ => None,
        }
    }

    fn start(&mut self, player: PlayerId, out: &mut Vec<ModeEvent>) {
        let heat = self.carried_heat(player);
        let chase = match self.chase.as_mut() {
            Some(chase) => chase,
            None => {
                self.last_id = self.last_id.wrapping_add(1).max(1);
                let mut targets = Targets::new();
                targets.add(target(player));
                self.chase = Some(Chase { id: self.last_id, targets, heat, evaded: false });
                out.push(ModeEvent::PursuitStarted { pursuit: self.last_id, targets: vec![player], heat });
                return;
            }
        };
        if !chase.targets.add(target(player)) {
            return;
        }
        chase.heat = chase.heat.max(heat);
        let targets = chased_players(&chase.targets);
        out.push(ModeEvent::TargetAdded { pursuit: chase.id, player, targets, heat: chase.heat });
    }

    fn set_heat(&mut self, level: u8, out: &mut Vec<ModeEvent>) {
        let cap = self.cap.max(1);
        let chase = match self.chase.as_mut() {
            Some(chase) => chase,
            None => return,
        };
        let level = level.clamp(1, cap);
        if level == chase.heat {
            return;
        }
        chase.heat = level;
        let targets = chased_players(&chase.targets);
        out.push(ModeEvent::HeatChanged { pursuit: chase.id, level, targets });
    }

    fn end_target(
        &mut self,
        player: PlayerId,
        reason: PursuitEndReason,
        out: &mut Vec<ModeEvent>,
    ) -> Option<PursuitEndReason> {
        let chase = self.chase.as_mut()?;
        let ended = match reason {
            PursuitEndReason::Busted => chase.targets.bust(target(player)),
            PursuitEndReason::Evaded | PursuitEndReason::Abandoned => chase.targets.leave(target(player)),
        };
        if !ended {
            return None;
        }
        out.push(ModeEvent::TargetEnded { pursuit: chase.id, player, reason });
        self.finish_if_over(out)
    }

    fn evade(&mut self, out: &mut Vec<ModeEvent>) -> Option<PursuitEndReason> {
        let chase = self.chase.as_mut()?;
        let chased: Vec<TargetId> = chase.targets.chased().collect();
        if chased.is_empty() {
            return None;
        }
        chase.evaded = true;
        for id in chased {
            chase.targets.leave(id);
            out.push(ModeEvent::TargetEnded {
                pursuit: chase.id,
                player: PlayerId(id.0),
                reason: PursuitEndReason::Evaded,
            });
        }
        self.finish_if_over(out)
    }

    fn finish_if_over(&mut self, out: &mut Vec<ModeEvent>) -> Option<PursuitEndReason> {
        let chase = self.chase.as_ref()?;
        if !chase.targets.is_over() {
            return None;
        }
        let any_busted = chase.targets.all().iter().any(|suspect| suspect.status == TargetStatus::Busted);
        let reason = match (chase.evaded, any_busted) {
            (true, _) => PursuitEndReason::Evaded,
            (false, true) => PursuitEndReason::Busted,
            (false, false) => PursuitEndReason::Abandoned,
        };
        out.push(ModeEvent::PursuitEnded { pursuit: chase.id, reason });
        self.chase = None;
        Some(reason)
    }
}
