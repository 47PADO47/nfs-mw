//! How many times the player ran into traffic: the post-race statistics' `Traffic Collisions`
//! (`docs/specs/race-results.md`). A car the player stays in contact with counts once; touching it again after a
//! second apart is another collision. **[ours]**: the original's counter is not read.

use super::TrafficWorld;

/// Seconds of simulated time without contact after which a car can be hit again.
const GAP: f32 = 1.0;

/// The traffic cars the player touched lately, and the collisions counted since the last read.
#[derive(Debug, Default)]
pub(super) struct HitCount {
    /// The cars in contact, with the seconds since the last contact.
    recent: Vec<(usize, f32)>,
    fresh: u32,
}

impl HitCount {
    /// Ages the contacts by `dt` and forgets the ones that have ended.
    pub fn tick(&mut self, dt: f32) {
        for (_, age) in &mut self.recent {
            *age += dt;
        }
        self.recent.retain(|&(_, age)| age < GAP);
    }

    /// The player touched the cars `owners` this step.
    pub fn note(&mut self, owners: impl IntoIterator<Item = usize>) {
        for owner in owners {
            if let Some(entry) = self.recent.iter_mut().find(|(car, _)| *car == owner) {
                entry.1 = 0.0;
                continue;
            }
            self.recent.push((owner, 0.0));
            self.fresh += 1;
        }
    }

    /// The collisions since the last call.
    pub fn take(&mut self) -> u32 {
        std::mem::take(&mut self.fresh)
    }
}

impl TrafficWorld {
    /// The traffic collisions the player had since the last call.
    pub fn take_traffic_collisions(&mut self) -> u32 {
        self.player_hits.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_contact_counts_once() {
        let mut count = HitCount::default();
        for _ in 0..30 {
            count.note([4]);
            count.tick(0.1);
        }
        assert_eq!(count.take(), 1);
        assert_eq!(count.take(), 0);
    }

    #[test]
    fn a_second_hit_after_a_gap_and_another_car_count_again() {
        let mut count = HitCount::default();
        count.note([1, 2]);
        count.tick(1.5);
        count.note([1]);
        assert_eq!(count.take(), 3);
    }
}
