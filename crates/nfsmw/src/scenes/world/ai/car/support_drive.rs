//! The drive of a support car (spec `docs/specs/ai-pursuit-support.md` sections 5 and 9). A support car carries the request it
//! came from (its serial) and its goal: a ram SUV drives at its intercept at 100 km/h once the start gate passes; a leader
//! car drives at its pursuit slot behind the target (the close-chase point of the tactics spec section 3); any other support
//! car chases the player as a cop does. Support cars take no formation slot (formations spec section 3); see the port status.

use blackbox_pursuit::goals::approach::{motion_vector, slot_point};
use blackbox_pursuit::support::ram::Ram;
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::AiCar;

/// What a support car drives at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SupportGoal {
    /// The plain chase of a cop.
    Chase,
    /// The close chase at this pursuit offset (`x` to the side, `y` ahead of the target's centre, metres).
    Close(Vec2),
    /// The head-on ram of an SUV.
    Ram(Ram),
}

/// A support car's tag: the request it belongs to (its serial, which the request's timer and cleanup compare) and its goal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportCar {
    pub serial: u32,
    pub goal: SupportGoal,
}

/// The aim of a cop's think: a leader car's close-chase point, or the given aim (the formation's or the tactics') otherwise.
/// Takes the tag by value, so the caller can hold the cop's role while it asks.
pub(super) fn aim(support: Option<SupportCar>, target: &Body, aim: Option<Vec3>) -> Option<Vec3> {
    let Some(SupportCar { goal: SupportGoal::Close(offset), .. }) = support else { return aim };
    let position = Vec2::new(target.position.x, target.position.z);
    let velocity = Vec2::new(target.velocity.x, target.velocity.z);
    let point = slot_point(position, motion_vector(velocity, target.forward), offset);
    Some(Vec3::new(point.x, target.position.y, point.y))
}

impl AiCar {
    /// Tags this car as a support car of a request, or clears the tag (the request's end, which returns it to the chase).
    pub fn set_support(&mut self, support: Option<SupportCar>) {
        self.support = support;
    }

    /// The serial of the request this car supports, if it is a support car.
    pub fn support_serial(&self) -> Option<u32> {
        self.support.map(|support| support.serial)
    }

    /// After the think: a ram SUV that has started drives at its intercept at the ram's speed. The others keep the think's
    /// drive. A ram that waits for the start gate keeps the chase too.
    pub(super) fn support_drive(&mut self, target: &Body) {
        let Some(SupportCar { goal: SupportGoal::Ram(_), .. }) = self.support else { return };
        let me = self.physics_position();
        let speed = self.speed().abs();
        let target_velocity = Vec2::new(target.velocity.x, target.velocity.z);
        let target_point = (Vec2::new(target.position.x, target.position.z), target_velocity, target_velocity.length());
        let Some(SupportCar { goal: SupportGoal::Ram(ram), .. }) = self.support.as_mut() else { return };
        let Some(drive) = ram.think(Vec2::new(me.x, me.z), speed, Some(target_point)) else { return };
        self.target = Vec3::new(drive.point.x, me.y, drive.point.y);
        self.speed = drive.speed;
    }
}
