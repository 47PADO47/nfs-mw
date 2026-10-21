//! Ordering the racers (`docs/specs/race-mode.md` section 4).

use std::cmp::Ordering;

/// What the ranking reads of one racer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standing {
    /// The race time at which the racer finished, when it has.
    pub finished_at: Option<f32>,
    /// Knocked out, totalled or busted, with the race time it happened at.
    pub out_at: Option<f32>,
    pub laps: u32,
    /// Percent complete, 0 to 100.
    pub percent: f32,
    /// Speed-trap points.
    pub points: f32,
}

impl Standing {
    /// A racer still racing.
    pub fn racing(laps: u32, percent: f32) -> Self {
        Self { finished_at: None, out_at: None, laps, percent, points: 0.0 }
    }
}

/// Which of two racers is ahead. A finished racer is ahead of one that has not finished, and the smaller finish time
/// is ahead; a racer that is out is behind every racer still racing (the later it went out, the better); otherwise
/// more laps, then more progress. With `by_points`, the larger points are ahead first and the rest breaks ties.
pub fn compare(a: &Standing, b: &Standing, by_points: bool) -> Ordering {
    if by_points {
        let order = b.points.total_cmp(&a.points);
        if order != Ordering::Equal {
            return order;
        }
    }
    match (a.finished_at, b.finished_at) {
        (Some(x), Some(y)) => return x.total_cmp(&y),
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        (None, None) => {}
    }
    match (a.out_at, b.out_at) {
        (Some(x), Some(y)) => return y.total_cmp(&x),
        (Some(_), None) => return Ordering::Greater,
        (None, Some(_)) => return Ordering::Less,
        (None, None) => {}
    }
    b.laps.cmp(&a.laps).then(b.percent.total_cmp(&a.percent))
}

/// The racers' indices, first place first. The sort is stable, so equal racers keep their order.
pub fn order(standings: &[Standing], by_points: bool) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..standings.len()).collect();
    indices.sort_by(|&a, &b| compare(&standings[a], &standings[b], by_points));
    indices
}

/// The place (1 for first) of each racer, by index.
pub fn places(standings: &[Standing], by_points: bool) -> Vec<u32> {
    let mut places = vec![0; standings.len()];
    for (place, index) in order(standings, by_points).into_iter().enumerate() {
        places[index] = place as u32 + 1;
    }
    places
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(at: f32) -> Standing {
        Standing { finished_at: Some(at), ..Standing::racing(1, 100.0) }
    }

    fn out(at: f32) -> Standing {
        Standing { out_at: Some(at), ..Standing::racing(0, 10.0) }
    }

    #[test]
    fn finished_racers_come_first_by_time() {
        let racers = [Standing::racing(0, 90.0), finished(70.0), finished(60.0)];
        assert_eq!(order(&racers, false), [2, 1, 0]);
    }

    #[test]
    fn racers_still_racing_rank_by_laps_then_progress() {
        let racers = [Standing::racing(1, 10.0), Standing::racing(2, 5.0), Standing::racing(2, 40.0)];
        assert_eq!(order(&racers, false), [2, 1, 0]);
    }

    #[test]
    fn a_racer_that_is_out_is_behind_everyone_still_racing() {
        let racers = [out(30.0), Standing::racing(0, 1.0), out(50.0)];
        assert_eq!(order(&racers, false), [1, 2, 0], "later out is better");
    }

    #[test]
    fn points_decide_in_a_points_race() {
        let a = Standing { points: 100.0, ..Standing::racing(0, 90.0) };
        let b = Standing { points: 300.0, ..Standing::racing(0, 10.0) };
        assert_eq!(order(&[a, b], true), [1, 0]);
        assert_eq!(order(&[a, b], false), [0, 1]);
    }

    #[test]
    fn places_are_one_based_by_index() {
        let racers = [Standing::racing(0, 10.0), Standing::racing(0, 50.0)];
        assert_eq!(places(&racers, false), [2, 1]);
    }

    #[test]
    fn equal_racers_keep_their_order() {
        let racers = [Standing::racing(0, 10.0), Standing::racing(0, 10.0)];
        assert_eq!(order(&racers, false), [0, 1]);
    }
}
