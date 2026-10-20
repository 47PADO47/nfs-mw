//! Cars for a roadblock taken from cops that are out of view. Spec: `docs/specs/ai-pursuit-roadblocks.md` section 2.2.
//!
//! The original asks its pool of inactive cop cars first. For the cars the pool cannot give it takes active cop cars
//! that are out of view: off screen for more than 5 s and more than 100 m from the view, the farthest first **[decomp]**.
//! The rewrite has no pool yet (the cop manager's pool is a separate piece of work), so every car of a lottery
//! roadblock is first sought here and the rest are created. Only roaming cops are taken: a cop of the chase is part of
//! its budget, and a roadblock car already stands in a block.

use glam::Vec3;

use crate::scenes::world::ai::TrafficWorld;

/// A cop must have been out of view this long (seconds)...
pub const UNSEEN: f32 = 5.0;
/// ...and this far from the view (metres) to be taken.
pub const FAR: f32 = 100.0;

/// A cop a roadblock may take.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The car's key (`AiCar::key`).
    pub key: u32,
    /// Its model, so the roadblock car is the same kind of cop.
    pub model: String,
    /// Metres from the view.
    pub away: f32,
}

/// The `wanted` farthest of `candidates`.
pub fn farthest(mut candidates: Vec<Candidate>, wanted: usize) -> Vec<Candidate> {
    candidates.sort_by(|a, b| b.away.total_cmp(&a.away).then(a.key.cmp(&b.key)));
    candidates.truncate(wanted);
    candidates
}

impl TrafficWorld {
    /// The roaming cops a roadblock of `wanted` cars may take, measured from `view` (render space).
    pub(super) fn steal_candidates(&self, view: Vec3, wanted: usize) -> Vec<Candidate> {
        let candidates = self
            .cars
            .iter()
            .filter(|car| (car.is_patrol() || car.is_fleeing()) && !car.is_roadblock() && !car.is_wrecked())
            .filter(|car| car.offscreen_time() > UNSEEN)
            .filter_map(|car| {
                let at = car.position();
                let away = (at.x - view.x).hypot(at.y - view.y);
                (away > FAR).then(|| Candidate { key: car.key(), model: car.name.clone(), away })
            })
            .collect();
        farthest(candidates, wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(key: u32, away: f32) -> Candidate {
        Candidate { key, model: "copmidsize".into(), away }
    }

    #[test]
    fn the_farthest_cops_go_first() {
        let got = farthest(vec![candidate(1, 120.0), candidate(2, 400.0), candidate(3, 250.0)], 2);
        assert_eq!(got.iter().map(|c| c.key).collect::<Vec<_>>(), vec![2, 3]);
    }

    #[test]
    fn fewer_cops_than_wanted_gives_what_there_is() {
        assert_eq!(farthest(vec![candidate(1, 150.0)], 4).len(), 1);
        assert!(farthest(Vec::new(), 3).is_empty());
        assert!(farthest(vec![candidate(1, 150.0)], 0).is_empty());
    }

    #[test]
    fn equal_distances_keep_a_stable_order() {
        let got = farthest(vec![candidate(9, 200.0), candidate(4, 200.0)], 2);
        assert_eq!(got.iter().map(|c| c.key).collect::<Vec<_>>(), vec![4, 9]);
    }
}
