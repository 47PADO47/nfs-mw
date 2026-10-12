//! The types a car is set up with: its role, the world it thinks in, and how it begins.

use blackbox_roads::{Body, RoadNetwork, SegmentIndex};

use super::cop::CopState;
use super::stop::Lights;
use super::traffic::Cruise;

/// What a car is for.
pub enum Role {
    Traffic,
    /// A cop chasing the player.
    Cop(Box<CopState>),
}

/// The world a car thinks in.
pub struct Ctx<'a, R> {
    pub net: &'a RoadNetwork,
    pub index: &'a SegmentIndex,
    pub rng: &'a mut R,
    /// Every computer-driven car (a semi with its trailer hitched is one long body), with whether it is a cop,
    /// and which of them is this car; the trailers that came loose follow after the cars.
    pub bodies: &'a [(Body, bool)],
    pub me: usize,
    pub player: Option<Body>,
    /// Where each car's cop goes instead of the player this think, by car index (`None`: the player).
    pub aims: &'a [Option<glam::Vec3>],
    /// The traffic lights, when they are on.
    pub lights: Option<Lights<'a>>,
}

/// How a new car begins: when its first think comes and how fast it rolls.
#[derive(Debug, Clone, Copy)]
pub struct Start {
    /// Physics steps added to the car's clock so the thinks of different cars fall on different steps.
    pub stagger: u32,
    pub speed: f32,
    /// The speeds it cruises at.
    pub cruise: Cruise,
    /// A patrol cop: a cop car driving like traffic.
    pub patrol: bool,
}

