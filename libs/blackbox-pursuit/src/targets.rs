//! Several target cars in one pursuit as pure rules: each target is chased, busted or has left, and the pursuit is over
//! when no target is still chased. [`assign`] splits the cops between the targets, nearest first, so no target gets
//! much more than its share. Nothing here reads a world or a clock: the caller names the targets (a player number) and
//! measures where the cops and the targets are, then applies the decisions.

use glam::Vec3;

/// One target car of a pursuit, chosen by the caller (a player number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TargetId(pub u8);

/// Where one target car stands in the pursuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetStatus {
    /// Still in the chase.
    Chased,
    /// Busted by the cops.
    Busted,
    /// Got away.
    Left,
}

/// One target and its status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suspect {
    pub id: TargetId,
    pub status: TargetStatus,
}

/// The targets of one pursuit, in id order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Targets {
    list: Vec<Suspect>,
}

impl Targets {
    /// A pursuit with no targets yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a target, chased. Returns `false` (and changes nothing) when the id is already present.
    pub fn add(&mut self, id: TargetId) -> bool {
        let index = match self.list.binary_search_by_key(&id, |suspect| suspect.id) {
            Ok(_) => return false,
            Err(index) => index,
        };
        self.list.insert(index, Suspect { id, status: TargetStatus::Chased });
        true
    }

    /// The status of a target, or `None` when the id is not in this pursuit.
    pub fn status(&self, id: TargetId) -> Option<TargetStatus> {
        self.index_of(id).map(|index| self.list[index].status)
    }

    /// The ids of the targets still chased, in id order.
    pub fn chased(&self) -> impl Iterator<Item = TargetId> + '_ {
        self.list.iter().filter(|suspect| suspect.status == TargetStatus::Chased).map(|suspect| suspect.id)
    }

    /// Busts a chased target. Returns `false` when the id is absent or the target is not chased.
    pub fn bust(&mut self, id: TargetId) -> bool {
        self.end_chase(id, TargetStatus::Busted)
    }

    /// Lets a chased target get away. Returns `false` when the id is absent or the target is not chased.
    pub fn leave(&mut self, id: TargetId) -> bool {
        self.end_chase(id, TargetStatus::Left)
    }

    /// How many targets are still chased.
    pub fn remaining(&self) -> usize {
        self.list.iter().filter(|suspect| suspect.status == TargetStatus::Chased).count()
    }

    /// Whether the pursuit is over: no target is still chased.
    pub fn is_over(&self) -> bool {
        self.remaining() == 0
    }

    /// Every target and its status, in id order.
    pub fn all(&self) -> &[Suspect] {
        &self.list
    }

    fn index_of(&self, id: TargetId) -> Option<usize> {
        self.list.binary_search_by_key(&id, |suspect| suspect.id).ok()
    }

    /// Moves a chased target to `status`. Only a chased target can end its chase.
    fn end_chase(&mut self, id: TargetId, status: TargetStatus) -> bool {
        let index = match self.index_of(id) {
            Some(index) => index,
            None => return false,
        };
        let suspect = &mut self.list[index];
        if suspect.status != TargetStatus::Chased {
            return false;
        }
        suspect.status = status;
        true
    }
}

/// Assigns each cop to one target: the result has one entry per cop, `None` for a cop that has no target. The cap per
/// target is `ceil(cops / targets)`, so the cops split evenly. Every cop-target pair is sorted by squared distance
/// (ties by cop index, then target id), and the walk gives a cop its nearest target while that target is under the cap.
/// With no targets every cop gets `None`; with one target every cop gets it.
pub fn assign(cops: &[Vec3], targets: &[(TargetId, Vec3)]) -> Vec<Option<TargetId>> {
    let mut assigned = vec![None; cops.len()];
    if targets.is_empty() {
        return assigned;
    }
    let cap = cops.len().div_ceil(targets.len());
    // (squared distance, cop index, target index) for every pair.
    let mut pairs: Vec<(f32, usize, usize)> = Vec::with_capacity(cops.len() * targets.len());
    for (cop_index, cop) in cops.iter().enumerate() {
        for (target_index, (_, position)) in targets.iter().enumerate() {
            pairs.push(((*cop - *position).length_squared(), cop_index, target_index));
        }
    }
    pairs.sort_by(|a, b| {
        a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(targets[a.2].0.cmp(&targets[b.2].0)).then(a.2.cmp(&b.2))
    });
    let mut counts = vec![0usize; targets.len()];
    for (_, cop_index, target_index) in pairs {
        if assigned[cop_index].is_some() || counts[target_index] >= cap {
            continue;
        }
        assigned[cop_index] = Some(targets[target_index].0);
        counts[target_index] += 1;
    }
    assigned
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_for(assigned: &[Option<TargetId>], id: TargetId) -> usize {
        assigned.iter().filter(|entry| **entry == Some(id)).count()
    }

    #[test]
    fn assign_splits_cops_evenly() {
        let a = TargetId(1);
        let b = TargetId(2);
        let targets = [(a, Vec3::ZERO), (b, Vec3::new(100.0, 0.0, 0.0))];
        let cops =
            [Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0)];
        let assigned = assign(&cops, &targets);
        assert_eq!(count_for(&assigned, a), 2, "two cops on the near target A");
        assert_eq!(count_for(&assigned, b), 2, "the other two take the far target B");
        assert!(assigned.iter().all(Option::is_some), "every cop has a target");
    }

    #[test]
    fn assign_prefers_the_nearest_target() {
        let a = TargetId(1);
        let b = TargetId(2);
        let targets = [(a, Vec3::ZERO), (b, Vec3::new(100.0, 0.0, 0.0))];
        let cops = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(99.0, 0.0, 0.0)];
        assert_eq!(assign(&cops, &targets), vec![Some(a), Some(b)], "each cop takes the target next to it");
        let one_cop = [Vec3::new(90.0, 0.0, 0.0)];
        assert_eq!(assign(&one_cop, &targets), vec![Some(b)], "a lone cop still takes its nearest target");
    }

    #[test]
    fn no_targets_assigns_none() {
        let cops = [Vec3::ZERO, Vec3::X, Vec3::Y];
        assert_eq!(assign(&cops, &[]), vec![None, None, None]);
    }

    #[test]
    fn one_target_takes_every_cop() {
        let only = TargetId(7);
        let targets = [(only, Vec3::new(5.0, 0.0, 0.0))];
        let cops = [Vec3::ZERO, Vec3::X * 40.0, Vec3::Z * 9.0];
        assert_eq!(assign(&cops, &targets), vec![Some(only), Some(only), Some(only)]);
    }

    #[test]
    fn pursuit_is_over_when_every_target_is_busted() {
        let mut targets = Targets::new();
        assert!(targets.add(TargetId(1)));
        assert!(targets.add(TargetId(2)));
        assert!(!targets.is_over());
        assert!(targets.bust(TargetId(1)));
        assert!(!targets.is_over(), "one target is still chased");
        assert_eq!(targets.remaining(), 1);
        assert!(targets.bust(TargetId(2)));
        assert!(targets.is_over());
        assert_eq!(targets.chased().count(), 0);
    }

    #[test]
    fn bust_only_changes_a_chased_target() {
        let mut targets = Targets::new();
        targets.add(TargetId(1));
        assert!(!targets.add(TargetId(1)), "a second add of the same id is refused");
        assert!(targets.leave(TargetId(1)), "a chased target can leave");
        assert_eq!(targets.status(TargetId(1)), Some(TargetStatus::Left));
        assert!(!targets.bust(TargetId(1)), "a target that left cannot be busted");
        assert_eq!(targets.status(TargetId(1)), Some(TargetStatus::Left));
        assert!(!targets.bust(TargetId(9)), "an unknown id changes nothing");
        assert_eq!(targets.status(TargetId(9)), None);
    }
}
