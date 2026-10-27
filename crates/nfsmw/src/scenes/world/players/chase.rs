//! A co-op chase, the world's side (`docs/specs/coop-career.md` section 5, ADR 0010): which players the police chase,
//! which of them is the primary the police's single-player rules follow, and one busted meter per target while two or
//! more are chased. A busted target pays on its own car and goes to the safe house (`career/coop_bust.rs`); the chase
//! goes on for the others and ends Busted only with the last one.

use blackbox_pursuit::Busted;
use blackbox_pursuit::targets::{TargetId, TargetStatus, Targets};

use super::super::WorldScene;
use super::super::ai::coop_chase::SECONDARY_SIGHT;
use super::{LEAD, PlayerId};

/// How near (metres) a cop of a running chase must come to a player for that player to join it as a target [ours].
const JOIN_DISTANCE: f32 = 50.0;

/// A player the police watch outside a chase is replaced by another only when that one is this much nearer a cop
/// (a share of the distance), so the watch does not flicker between two cars [ours].
const WATCH_HYSTERESIS: f32 = 0.8;

/// The co-op chase state, kept in the co-op session.
#[derive(Debug, Clone, Default)]
pub struct CoopChase {
    /// The player the police follow: the one they watch outside a chase, the first chased one in it.
    pub primary: PlayerId,
    /// The chase's targets, by player id; empty outside a chase.
    pub targets: Targets,
    /// Each target's busted timer while two or more are chased.
    meters: Vec<(PlayerId, Busted)>,
    /// The police were chasing at the last tick.
    was_pursuing: bool,
    /// The targets and the primary of the chase that ended last, kept for the career's settlement of it
    /// (`career/pursuit.rs` reads them after the tick that ends the chase).
    pub last_targets: Targets,
    pub last_primary: PlayerId,
}

impl CoopChase {
    /// A target's busted meter, 0 to 1, while the world runs one; `None` otherwise (the police's meter is the one).
    pub fn meter(&self, id: PlayerId, tuning: &blackbox_pursuit::busted::BustTuning) -> Option<f32> {
        self.meters.iter().find(|(player, _)| *player == id).map(|(_, meter)| meter.meter(tuning))
    }

    /// Whether `id` is a chased target of the running chase.
    pub fn is_chased(&self, id: PlayerId) -> bool {
        self.targets.status(target(id)) == Some(TargetStatus::Chased)
    }

    /// The players still chased, in id order.
    pub fn chased(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.targets.chased().map(|id| PlayerId(id.0))
    }
}

fn target(id: PlayerId) -> TargetId {
    TargetId(id.0)
}

impl WorldScene {
    /// The player the police follow: the lead outside co-op.
    pub(in crate::scenes::world) fn chase_primary(&self) -> PlayerId {
        self.coop.as_ref().map_or(LEAD, |session| session.chase.primary)
    }

    /// One police tick of a co-op chase, before the police read the primary: who is chased, who the primary is, the
    /// cops' secondary targets, the shared sighting and the per-target busts. Does nothing outside co-op.
    pub(in crate::scenes::world) fn coop_chase_tick_with_delta(&mut self, dt: f32) {
        let (Some(session), Some(traffic)) = (self.coop.as_mut(), self.traffic.as_mut()) else { return };
        let chase = &mut session.chase;
        let pursuing = traffic.is_pursuing();
        let cars: Vec<_> =
            self.players.iter().filter(|p| p.drive.has_sim()).map(|p| (p.id, p.drive.player_view())).collect();
        if !pursuing {
            if chase.was_pursuing {
                chase.last_targets = std::mem::take(&mut chase.targets);
                chase.last_primary = chase.primary;
            }
            chase.targets = Targets::new();
            chase.meters.clear();
            chase.was_pursuing = false;
            chase.primary =
                watched(chase.primary, cars.iter().map(|(id, view)| (*id, traffic.cop_distance(view.position))));
            traffic.set_secondary_targets([]);
            traffic.set_secondary_seen(false);
            traffic.hold_shared_bust(false);
            return;
        }
        if !chase.was_pursuing {
            chase.was_pursuing = true;
            chase.targets.add(target(chase.primary));
        }
        for (id, view) in &cars {
            if chase.targets.status(target(*id)).is_none() && traffic.cop_distance(view.position) < JOIN_DISTANCE {
                log::info!("co-op: player {} joins the chase", id.0 + 1);
                chase.targets.add(target(*id));
            }
        }
        let shared = chase.targets.remaining() >= 2;
        traffic.hold_shared_bust(shared);
        let mut busted = Vec::new();
        if shared {
            let tuning = traffic.bust_tuning();
            let chased: Vec<_> = cars.iter().filter(|(id, _)| chase.is_chased(*id)).collect();
            for (id, view) in chased {
                let input = traffic.bust_input_for(view, *id == chase.primary);
                let at = match chase.meters.iter().position(|(player, _)| player == id) {
                    Some(at) => at,
                    None => {
                        chase.meters.push((*id, Busted::default()));
                        chase.meters.len() - 1
                    }
                };
                if chase.meters[at].1.update(dt, &input, &tuning) {
                    busted.push(*id);
                }
            }
        }
        for id in &busted {
            chase.targets.bust(target(*id));
            chase.meters.retain(|(player, _)| player != id);
        }
        if chase.targets.remaining() < 2 {
            chase.meters.clear();
        }
        let next = chase.chased().next();
        if !chase.is_chased(chase.primary)
            && let Some(next) = next
        {
            chase.primary = next;
        }
        let secondary = |id: PlayerId| chase.is_chased(id) && id != chase.primary;
        let seen = cars
            .iter()
            .filter(|(id, _)| secondary(*id))
            .any(|(_, view)| traffic.cop_distance(view.position) < SECONDARY_SIGHT);
        traffic.set_secondary_seen(seen);
        let bodies: Vec<_> = self
            .players
            .iter()
            .filter(|p| secondary(p.id) && p.drive.has_sim())
            .map(|p| (target(p.id), p.drive.ai_body()))
            .collect();
        traffic.set_secondary_targets(bodies);
        for id in busted {
            self.coop_bust(id);
        }
    }
}

/// The player the police watch outside a chase: the one nearest a cop, keeping the current one unless another is
/// clearly nearer. The lead with nobody near a cop.
fn watched(current: PlayerId, distances: impl Iterator<Item = (PlayerId, f32)>) -> PlayerId {
    let distances: Vec<(PlayerId, f32)> = distances.collect();
    let Some(&(nearest, best)) = distances.iter().min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0))) else {
        return LEAD;
    };
    let Some(&(_, now)) = distances.iter().find(|(id, _)| *id == current) else { return nearest };
    match best < now * WATCH_HYSTERESIS {
        true => nearest,
        false => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUEST: PlayerId = PlayerId(1);

    #[test]
    fn the_watch_keeps_its_player_unless_another_is_clearly_nearer() {
        assert_eq!(watched(LEAD, [(LEAD, 100.0), (GUEST, 90.0)].into_iter()), LEAD);
        assert_eq!(watched(LEAD, [(LEAD, 100.0), (GUEST, 50.0)].into_iter()), GUEST);
    }

    #[test]
    fn the_watch_falls_back_to_the_lead_without_players() {
        assert_eq!(watched(GUEST, std::iter::empty()), LEAD);
    }

    #[test]
    fn a_player_that_left_is_not_watched() {
        assert_eq!(watched(GUEST, [(LEAD, f32::INFINITY)].into_iter()), LEAD);
    }

    #[test]
    fn a_busted_primary_hands_the_chase_on() {
        let mut chase = CoopChase::default();
        chase.targets.add(target(LEAD));
        chase.targets.add(target(GUEST));
        chase.targets.bust(target(LEAD));
        assert!(!chase.is_chased(LEAD));
        assert_eq!(chase.chased().collect::<Vec<_>>(), [GUEST]);
    }
}
