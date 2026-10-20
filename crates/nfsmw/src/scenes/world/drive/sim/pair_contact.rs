//! How two cars met, kept for the police radio: the speeds and the contact point in each car's own axes, which tell a
//! rear-end from a T-bone (`blackbox_pursuit::speech::impact`). Taken before the response changes the velocities.

use blackbox_pursuit::speech::impact::Contact;
use blackbox_vehicle::rigid_body::RigidBody;
use glam::Vec3;

/// One of the two cars.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Side {
    /// The contact point from the car's centre, in the car's axes (x right, z forward).
    arm: Vec3,
    /// The car's half extents.
    dimension: Vec3,
    /// The car's speed, and how fast it moved along the contact normal, before the hit.
    speed: f32,
    along_normal: f32,
    forward: Vec3,
}

impl Side {
    fn of(body: &RigidBody, point: Vec3, normal: Vec3) -> Self {
        let axes = body.rotation();
        Self {
            arm: axes.transpose() * (point - body.position),
            dimension: body.dimension(),
            speed: body.linear_velocity.length(),
            along_normal: body.linear_velocity.dot(normal).abs(),
            forward: axes.z_axis,
        }
    }
}

/// A hit between two cars. The first car is the one whose `collide_with` ran, the second the one it hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairContact {
    first: Side,
    second: Side,
    /// How fast the cars closed at the contact, metres per second.
    pub closing: f32,
    /// The speed each car lost to the response, metres per second.
    pub delta_v: [f32; 2],
}

impl PairContact {
    pub(super) fn capture(first: &RigidBody, second: &RigidBody, point: Vec3, normal: Vec3, closing: f32) -> Self {
        Self {
            first: Side::of(first, point, normal),
            second: Side::of(second, point, normal),
            closing,
            delta_v: [0.0; 2],
        }
    }

    /// The speed the two cars lost between them (the original's `impulseA + impulseB`).
    pub fn total_delta_v(&self) -> f32 {
        self.delta_v[0] + self.delta_v[1]
    }

    /// The first car's speed before the hit.
    pub fn first_speed(&self) -> f32 {
        self.first.speed
    }

    /// The first car's speed after the hit, estimated as its speed less what the response took.
    pub fn first_speed_after(&self) -> f32 {
        (self.first.speed - self.delta_v[0]).max(0.0)
    }

    /// The contact as the classification reads it, with the first car the player's (or the cop's when `player_first` is
    /// false).
    pub fn contact(&self, player_first: bool) -> Contact {
        let (player, cop) = match player_first {
            true => (&self.first, &self.second),
            false => (&self.second, &self.first),
        };
        Contact {
            forward_dot: player.forward.dot(cop.forward),
            arm_cop: cop.arm,
            arm_player: player.arm,
            dim_cop: cop.dimension,
            dim_player: player.dimension,
            normal_speed_cop: cop.along_normal,
            normal_speed_player: player.along_normal,
            speed_cop: cop.speed,
            speed_player: player.speed,
        }
    }
}
