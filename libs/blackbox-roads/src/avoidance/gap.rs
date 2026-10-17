//! Whether a lane next to a car is clear of other cars for a lane change.
//! Rewrite extension (spec `docs/specs/ai-traffic-obstacles.md` §10.1).

/// Metres behind the car that a car in the target lane must not be inside.
pub const BEHIND: f32 = 10.0;
/// Seconds of the car's own speed added to the look-ahead of the gap test.
pub const AHEAD_SECONDS: f32 = 2.0;
/// Metres added to that look-ahead.
pub const AHEAD_EXTRA: f32 = 15.0;

/// Another car, relative to the changing car. `along` is metres ahead (negative behind) along the car's direction;
/// `lateral` is metres to the right of the lane line the car follows; the speeds are along the car's direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Other {
    pub along: f32,
    pub lateral: f32,
    pub along_speed: f32,
}

/// Whether no other car is in the lane centred `lane_lateral` metres right of the car's lane line, with
/// `half_width` as the lane's half width plus the car's half width, when a car at `speed` (m/s) moves over.
///
/// A car in the gap blocks the lane unless it is moving away from the car: a car ahead that is slower than the
/// car does not block, and a car behind that is faster does not either. The gap runs from [`BEHIND`] metres back
/// to [`AHEAD_SECONDS`] × speed + [`AHEAD_EXTRA`] metres ahead.
pub fn lane_clear(others: &[Other], lane_lateral: f32, half_width: f32, speed: f32) -> bool {
    let speed = speed.abs();
    others.iter().all(|other| !in_gap(other, lane_lateral, half_width, speed))
}

fn in_gap(other: &Other, lane_lateral: f32, half_width: f32, speed: f32) -> bool {
    let inside_lane = (other.lateral - lane_lateral).abs() <= half_width;
    let reach = AHEAD_SECONDS * speed + AHEAD_EXTRA;
    let in_span = (-BEHIND..=reach).contains(&other.along);
    if !(inside_lane && in_span) {
        return false;
    }
    match other.along >= 0.0 {
        true => other.along_speed < speed,
        false => other.along_speed > speed,
    }
}

/// The lanes next to lane `lane` in `lanes` (the traffic lanes of one direction, centre first): the one towards the
/// centre and the one towards the kerb. `None` where there is none.
pub fn adjacent_lanes(lanes: &[usize], lane: usize) -> [Option<usize>; 2] {
    let Some(at) = lanes.iter().position(|&l| l == lane) else { return [None, None] };
    let towards_centre = at.checked_sub(1).map(|i| lanes[i]);
    let towards_kerb = lanes.get(at + 1).copied();
    [towards_centre, towards_kerb]
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANE: f32 = 4.0;
    const HALF: f32 = 2.0;

    fn other(along: f32, lateral: f32, along_speed: f32) -> Other {
        Other { along, lateral, along_speed }
    }

    #[test]
    fn a_car_in_the_target_lane_within_the_gap_blocks_it() {
        let speed = 2.0;
        assert!(!lane_clear(&[other(15.0, LANE, 0.0)], LANE, HALF, speed), "ahead, stopped");
        assert!(!lane_clear(&[other(-8.0, LANE, 5.0)], LANE, HALF, speed), "behind, closing");
    }

    #[test]
    fn the_gap_runs_ten_metres_back_and_two_seconds_plus_fifteen_ahead() {
        let speed = 2.0;
        let reach = AHEAD_SECONDS * speed + AHEAD_EXTRA;
        assert!(lane_clear(&[other(-BEHIND - 0.5, LANE, 5.0)], LANE, HALF, speed), "just past the back of the gap");
        assert!(lane_clear(&[other(reach + 0.5, LANE, 0.0)], LANE, HALF, speed), "just past the front of the gap");
        assert!(!lane_clear(&[other(-BEHIND + 0.5, LANE, 5.0)], LANE, HALF, speed), "inside the back of the gap");
        assert!(!lane_clear(&[other(reach - 0.5, LANE, 0.0)], LANE, HALF, speed), "inside the front of the gap");
    }

    #[test]
    fn a_car_outside_the_target_lane_does_not_block_it() {
        let speed = 2.0;
        assert!(lane_clear(&[other(5.0, 0.0, 0.0)], LANE, HALF, speed), "still in its own lane");
        assert!(lane_clear(&[other(5.0, LANE + HALF + 0.1, 0.0)], LANE, HALF, speed), "beyond the lane");
    }

    #[test]
    fn a_car_pulling_away_ahead_or_falling_back_behind_is_no_obstacle() {
        let speed = 3.0;
        assert!(lane_clear(&[other(10.0, LANE, 8.0)], LANE, HALF, speed), "ahead and faster");
        assert!(lane_clear(&[other(-5.0, LANE, 1.0)], LANE, HALF, speed), "behind and slower");
    }

    #[test]
    fn neighbours_are_the_lanes_on_either_side_in_the_list() {
        let lanes = [2, 4, 6];
        assert_eq!(adjacent_lanes(&lanes, 4), [Some(2), Some(6)]);
        assert_eq!(adjacent_lanes(&lanes, 2), [None, Some(4)]);
        assert_eq!(adjacent_lanes(&lanes, 6), [Some(4), None]);
        assert_eq!(adjacent_lanes(&lanes, 9), [None, None], "not a traffic lane");
        assert_eq!(adjacent_lanes(&[], 0), [None, None]);
    }
}
