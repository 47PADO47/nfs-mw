//! Putting one support car on the road at its placement point (spec `ai-pursuit-support.md` sections 6 and 7): the cop spawn
//! path (`spawn_cop_at` in `pursuit.rs`) at the point along the target's road, facing the target, and tagged with its
//! request. A heavy car is placed ahead of the target and a leader car behind it.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::support::placement::{leader_offset, point_along};
use blackbox_pursuit::support::{Car, Strategy, ram};
use blackbox_roads::Body;
use blackbox_vehicle::performance::Performance;
use glam::{Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;

use super::super::TrafficWorld;
use super::super::car::support_drive::{SupportCar, SupportGoal};
use super::super::pursuit::{CopSpot, START_SPEED};
use super::car_model;

/// Where one support car goes and what it is for.
#[derive(Debug, Clone, Copy)]
pub struct SupportSpot {
    pub car: Car,
    pub strategy: Strategy,
    /// The car's place in its request (the leader's offset follows it).
    pub index: u32,
    pub serial: u32,
    /// The signed distance from the target along its road, metres: ahead positive, behind negative.
    pub distance: f32,
    pub target: Body,
    pub performance: Performance,
}

/// The goal a support car of `strategy` takes: a ram SUV rams, a leader car keeps its close-chase slot, and the other heavy
/// cars chase.
pub fn goal_for(strategy: Strategy, index: u32) -> SupportGoal {
    match strategy {
        Strategy::Ram => SupportGoal::Ram(ram::Ram::default()),
        heavy if heavy.is_heavy() => SupportGoal::Chase,
        _ => SupportGoal::Close(leader_offset(index)),
    }
}

/// The speed a car is placed rolling at: a ram SUV at the spec's start speed toward the target, the others at the cop start
/// speed.
fn start_speed(strategy: Strategy, distance: f32, target_speed: f32) -> f32 {
    match strategy {
        Strategy::Ram => ram::start_speed(distance, target_speed),
        _ => START_SPEED,
    }
}

impl TrafficWorld {
    /// Puts the support car of `spot` on the road. Returns whether it stands (a refused spot, or no model of its name,
    /// returns false and nothing is spawned).
    pub(super) fn spawn_support_car(
        &mut self,
        spot: &SupportSpot,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let target = spot.target;
        let point = point_along(Vec2::new(target.position.x, target.position.z), target.forward, spot.distance);
        let at = Vec3::new(point.x, target.position.y, point.y);
        let target_speed = Vec2::new(target.velocity.x, target.velocity.z).length();
        let speed = start_speed(spot.strategy, spot.distance, target_speed);
        let cop = CopSpot { at, target: target.position, speed, player: spot.performance };
        if !self.spawn_cop_at(car_model(spot.car), cop, collision, surfaces) {
            return false;
        }
        let goal = goal_for(spot.strategy, spot.index);
        if let Some(car) = self.cars.last_mut() {
            car.set_support(Some(SupportCar { serial: spot.serial, goal }));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ram_suv_rams_a_leader_car_keeps_its_slot_and_a_heavy_car_chases() {
        assert!(matches!(goal_for(Strategy::Ram, 0), SupportGoal::Ram(_)));
        assert_eq!(goal_for(Strategy::EBrake, 0), SupportGoal::Chase);
        assert_eq!(goal_for(Strategy::CoordinatedEBrake, 1), SupportGoal::Chase);
        assert_eq!(goal_for(Strategy::CrossFollow, 0), SupportGoal::Close(Vec2::new(0.0, -20.0)));
        assert_eq!(goal_for(Strategy::CrossPlusVBlock, 1), SupportGoal::Close(Vec2::new(-4.0, -24.0)));
        assert_eq!(goal_for(Strategy::CrossPlusVBlock, 2), SupportGoal::Close(Vec2::new(4.0, -24.0)));
    }

    #[test]
    fn a_leader_car_is_placed_rolling_at_the_cop_start_speed() {
        assert_eq!(start_speed(Strategy::CrossFollow, -150.0, 20.0), START_SPEED);
    }

    #[test]
    fn a_ram_car_is_placed_at_the_spec_start_speed() {
        // A target at 20 m/s with the car 340 m ahead: 1.2 times the target's speed, under the 26.82 cap.
        assert!((start_speed(Strategy::Ram, 340.0, 20.0) - 24.0).abs() < 1e-4);
    }
}
