//! `Moved` events: the server put a player's car somewhere its motion did not take it (a reset, a respawn after a bust,
//! a catch-up warp, the road placement of a car that was waiting). Found from the car's position after each tick: a car
//! that was waiting for its road and now stands on it moved, and so did a car that went further in one tick than its
//! speed can carry it.

use glam::Vec3;
use nfsmw_sim::{PlayerId, SimEvent};

use crate::scenes::world::drive::STEP;

/// Metres a car may go in one tick beyond what its speed explains before the step counts as a jump [ours]: a hard
/// contact changes the speed within the tick, and the slack covers it.
const JUMP_SLACK: f32 = 2.0;

/// One player's car after a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarSpot {
    pub player: PlayerId,
    /// Render space, metres.
    pub position: Vec3,
    /// Metres per second.
    pub speed: f32,
    /// The car waits for its road: it has no physics yet.
    pub waiting: bool,
}

/// The players' cars at the last tick.
#[derive(Debug, Clone, Default)]
pub struct MovedTracker {
    last: Vec<CarSpot>,
}

impl MovedTracker {
    /// Compare the cars after this tick with the last tick's; a `Moved` per player that was put somewhere goes to
    /// `events`, in the order of `cars`. A player new since the last tick has no event.
    pub fn update(&mut self, cars: &[CarSpot], events: &mut Vec<SimEvent>) {
        for car in cars {
            let Some(before) = self.last.iter().find(|spot| spot.player == car.player) else { continue };
            if car.waiting {
                continue;
            }
            if before.waiting || jumped(before, car) {
                events.push(SimEvent::Moved { player: car.player });
            }
        }
        self.last.clear();
        self.last.extend_from_slice(cars);
    }
}

/// Whether the car went further than its speed can carry it in one tick.
fn jumped(before: &CarSpot, after: &CarSpot) -> bool {
    let reach = before.speed.abs().max(after.speed.abs()) * STEP + JUMP_SLACK;
    let distance = before.position.distance(after.position);
    !distance.is_finite() || distance > reach
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: PlayerId = PlayerId(0);

    fn spot(x: f32, speed: f32, waiting: bool) -> CarSpot {
        CarSpot { player: ONE, position: Vec3::new(x, 0.0, 0.0), speed, waiting }
    }

    fn run(tracker: &mut MovedTracker, car: CarSpot) -> Vec<SimEvent> {
        let mut events = Vec::new();
        tracker.update(&[car], &mut events);
        events
    }

    #[test]
    fn driving_is_not_a_move_and_a_jump_is() {
        let mut tracker = MovedTracker::default();
        assert_eq!(run(&mut tracker, spot(0.0, 60.0, false)), [], "the first sight of a car is no move");
        assert_eq!(run(&mut tracker, spot(1.0, 60.0, false)), [], "60 m/s goes 1 m a tick");
        assert_eq!(run(&mut tracker, spot(50.0, 60.0, false)), [SimEvent::Moved { player: ONE }]);
    }

    #[test]
    fn a_placement_after_waiting_is_a_move() {
        let mut tracker = MovedTracker::default();
        run(&mut tracker, spot(0.0, 0.0, false));
        assert_eq!(run(&mut tracker, spot(0.0, 0.0, true)), [], "the reset asks for a road");
        assert_eq!(run(&mut tracker, spot(0.5, 0.0, false)), [SimEvent::Moved { player: ONE }]);
        assert_eq!(run(&mut tracker, spot(0.5, 0.0, false)), []);
    }

    #[test]
    fn a_broken_position_counts_as_a_jump() {
        let mut tracker = MovedTracker::default();
        run(&mut tracker, spot(0.0, 0.0, false));
        assert_eq!(run(&mut tracker, spot(f32::NAN, 0.0, false)), [SimEvent::Moved { player: ONE }]);
    }
}
